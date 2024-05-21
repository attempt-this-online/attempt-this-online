# Usage in Docker

The ATO backend can be run inside a Docker container.
This doesn't make it more secure (see [Security Considerations](#security-considerations) below)
but it does make it easier to deploy repeatably, with fewer requirements on the host operating system.

The Docker image is published at TODO.

The frontend is not currently Dockerised (see #140), so only the API will be available.
It is available on port 8500.

If using this in production, you'll need a reverse proxy (such as nginx or Caddy),
to provide HTTPS, and to serve both the frontend and backend from the same origin.

## System requirements

You'll need to be using either:

* A recent version of Docker Desktop; or:

* Podman or Docker Engine, and:

  * Your operating system must support unprivileged user namespaces,
    and cgroup v2 (what systemd calls the "unified cgroup hierarchy")
  * You must use Docker/Podman in rootful mode; ATO will not work with rootless containers

## Container requirements

* The ATO container must be run using the `--privileged` Docker option
* The ATO container must run in its own cgroup namespace. If this is not the default on your system,
  pass the `--cgroupns=private` option to Docker when creating the ATO container,
* The ATO container must have a volume mounted to the path `/mnt`.
  For example, you could pass `-v ATO_images:/mnt` to Docker,
  to create a persistent anonymous volume called `ATO_images` which will be mounted there.

This example Docker command will take care of these requirements for you:

```
sudo docker run -d --name ato -p 127.0.0.1:8500:8500 --privileged --cgroupns private -v ato_images:/mnt (TODO: image name?)
```

## Security considerations

Note that this container must be run in privileged mode,
which means using a Docker container provides **no additional security** compared to the
[standard installation method](./installation.md).

If someone is able to escape the ATO sandbox, there is no Docker sandbox to contain them,
so they will be able to gain full access to the host system.

Ideally you should run the ATO container in its own virtual machine, with minimal network access,
so that if that happens, they can't control anything other than the ATO backend.

The backend does not run its server with HTTPS, as it is designed to be used behind a reverse proxy.
Therefore, you should not serve it directly to users on a network.
