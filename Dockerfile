FROM archlinux as build

WORKDIR /tmp
RUN pacman -Syu --noconfirm base-devel rustup && rustup install nightly

COPY yargs.c /tmp/
RUN gcc -Wall -Werror -static /tmp/yargs.c -o /yargs

COPY languages.json Cargo.toml Cargo.lock /tmp/
COPY src /tmp/src
COPY .cargo /tmp/.cargo
ARG CARGO_FLAGS=--release
RUN cargo build $CARGO_FLAGS && mv target/**/attempt-this-online /

FROM archlinux

# install dependencies
RUN <<"EOF"
set -ex
pacman -Syu --noconfirm base-devel jq skopeo entr strace

mkdir -p /usr/local/lib/ATO

curl -Lo /usr/local/bin/containers-storage "https://github.com/attempt-this-online/containers-storage/releases/download/v1.53.0/containers-storage.linux.amd64"
chmod +x /usr/local/bin/containers-storage

curl -Lo /usr/local/lib/ATO/bash "https://github.com/attempt-this-online/static-bash/releases/download/5.2.0(1)-rc2/bash"
chmod +x /usr/local/lib/ATO/bash
EOF

# cnfigure system
RUN <<"EOF"
mkdir -p /usr/local/share/ATO/overlayfs_upper/{ATO,proc,dev}
mkdir -p /usr/local/share/ATO/runners
ln -s /mnt/rootfs /usr/local/lib/ATO/rootfs
ln -s /mnt/env /usr/local/lib/ATO/env
useradd -U ato
EOF

COPY <<"EOF" /entrypoint.sh
#!/bin/sh
set -ex

GRAPH=/mnt/containers_storage
RUN=/run/containers/storage
STORAGE_BACKEND="containers-storage:[$GRAPH+$RUN]"

if [ ! -f /mnt/done ]; then
  echo "image data missing; downloading..."
  rm -rf /mnt/*
  mkdir /mnt/containers_storage /mnt/rootfs /mnt/env
  jq -r '.[].image' < /languages.json | sort -u > /mnt/images.txt
  for image in $(cat /mnt/images.txt); do
    echo "$image"
    # download image from Docker registry
    skopeo copy docker://"$image" "$STORAGE_BACKEND$image"

    # replace / so the image name is safe in filenames,
    # and : so it's safe in mount arguments which accept a colon-separated list
    image_pathsafe="$(echo "$image" | tr '/:' '+')"

    # extract environment variables from the image
    skopeo inspect "$STORAGE_BACKEND$image" \
      | jq --raw-output0 '.Env[]' > /mnt/env/"$image_pathsafe"
  done
  touch /mnt/done
fi

for image in $(cat /mnt/images.txt); do
  mountpoint=$(
    containers-storage --graph "$GRAPH" --run "$RUN" --storage-driver overlay \
      mount --read-only "$image"
  )
  image_pathsafe="$(echo "$image" | tr '/:' '+')"
  mount --bind --mkdir -o ro=recursive "$mountpoint" /mnt/rootfs/"$image_pathsafe"
done
echo "all images mounted"

# move self into a subtree of $ATO_CGROUP_PATH, because otherwise $ATO_CGROUP_PATH cannot be configured properly.
# See https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html#no-internal-process-constraint
mkdir /sys/fs/cgroup/server
echo "$$" > /sys/fs/cgroup/server/cgroup.procs

export ATO_CGROUP_PATH=/sys/fs/cgroup
echo +memory > "$ATO_CGROUP_PATH/cgroup.subtree_control"
chown -R ato:ato "$ATO_CGROUP_PATH"

mkdir -p /run/ATO

exec "$@"
EOF
RUN chmod +x /entrypoint.sh
ENTRYPOINT ["/entrypoint.sh"]

COPY runners/ /usr/local/share/ATO/runners/
RUN chmod -R a+rx /usr/local/share/ATO/runners
COPY languages.json /
COPY --from=build /yargs /usr/local/lib/ATO/yargs
COPY --from=build /attempt-this-online /usr/local/lib/ATO/server

ENV ATO_BIND=0.0.0.0:8500
EXPOSE 8500
CMD setpriv --reuid ato --regid ato --init-groups /usr/local/lib/ATO/server
