# NOTE: this Dockerfile is only to set up a minimal developer environment for the backend.
# It needs to be run --privileged in order for the sandbox to work.

FROM archlinux

# with some care, we can skip re-downloading the attemptthisonline/zsh docker image by recreating it on the fly
RUN pacman -Syu --noconfirm zsh entr \
    && mkdir -p /usr/local/lib/ATO/rootfs/attemptthisonline+zsh \
    && tar --exclude /usr/local/lib/ATO --exclude /sys --exclude /proc --exclude /dev -c / | tar -xC /usr/local/lib/ATO/rootfs/attemptthisonline+zsh

RUN mkdir -p /usr/local/share/ATO/runners /usr/local/lib/ATO/rootfs/attemptthisonline+zsh/{ATO,proc,sys,dev} /usr/local/lib/ATO/env /src \
    && printf 'PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin\0LANG=C.UTF-8\0' > /usr/local/lib/ATO/env/attemptthisonline+zsh \
    && ln -s /src/runners/zsh /usr/local/share/ATO/runners/zsh \
    && ln -s /src/target/debug/invoke /usr/local/lib/ATO/invoke \
    && ln -s /src/dist/attempt_this_online/yargs /usr/local/lib/ATO/yargs \
    && ln -s /usr/bin/bash /usr/local/lib/ATO/bash

WORKDIR /src

ENV ATO_CGROUP_PATH=/run/cgroup2
ENV ATO_BIND=0.0.0.0:8500

CMD ls target/debug/attempt-this-online | entr -cr /_
