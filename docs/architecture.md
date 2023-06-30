# Architecture
The best way to see how Attempt This Online works is by listing the steps taken between user button-press and output
display. This assumes ATO is being run with the default full setup.

- User clicks run on the webpage
- Frontend concatenates header, body, and footer of program, and encodes code and input
- Browser opens websocket to `https://ato.pxeger.com/api/v1/ws/execute`
    - Sends a request containing [msgpack](https://msgpack.org)-encoded data, which is a map containing:
        - `language` (string): the name/identifier of the language
        - `code` (bytes): the program code to run
        - `input` (bytes): the input to give to the program
        - `arguments` (bytes[]): the command-line arguments to run the program with
        - `options` (bytes[]): the command-line arguments to give to the language compiler or interpreter
        - `timeout` (int): the maximum number of seconds to run the program for
- [`nginx`](https://en.wikipedia.org/wiki/Nginx) server receives the request
- `nginx` forwards the request to the Rust API server over the local port `8500`
- Rust backend ([`main.rs`]) forks a new process to handle the request
- The new process reads and decodes the WebSocket request, passing it to the `invoke` function in [`sandbox.rs`]
- `sandbox.rs` creates an isolated Linux container in a new [cgroup](https://docs.kernel.org/admin-guide/cgroup-v2.html)
    - The container has mounted:
         - `/` (the root file system): from `/usr/local/lib/ATO/rootfs`, an extracted Docker image containing the root
         file system for the relevant language.
         - Over the top of the root file system are an extra layer `/usr/local/share/ATO/overlayfs_upper`,
            (which contains empty directories for mount points for `/ATO`, `/proc`, and `/dev`)
            and a temporary writeable directory `/run/ATO/upper` to allow the user to "write" to the filesystem
         - Various other special Linux filesystems and files (`/proc`, `/sys`, `/dev/*`, `/tmp`)
         - `/ATO/`: A `tmpfs` where the following few files will be put
         - `/ATO/bash`: A statically linked `/bin/bash` ([stolen from Debian](https://packages.debian.org/unstable/amd64/bash-static/download)),
         in case the language's Docker image doesn't have a bash
         - `/ATO/yargs`: a wrapper to execute a command with null-terminated arguments from a file
    - The container has temporary files `/ATO/code`, `/ATO/input`, `/ATO/arguments`, `/ATO/options` created, containing
      the input values from the API request
    - It has `rlimit`s and some cgroup values set to limit resource usage
- Meanwhile, `sandbox.rs` spawns a second thread which monitors the program's output, and feeds it into WebSocket
  response messages (encoded with `msgpack` again)
- `sandbox.rs` waits for the process to finish, and then stops the second thread
- `sandbox.rs` kills all processes remaining inside the container's cgroup, and removes the cgroup
- API takes in the output to create a "done" response with some more details about the program, which is sent back to
  the client over the WebSocket again
- The frontend decodes and lays out the result

More details of the sandbox container can be found by consulting its (not-well-commented) source code.

[`main.rs`]: ../src/main.rs
[`sandbox.rs`]: ../src/sandbox.rs

## Image loading
Images are downloaded from Docker Hub using [`skopeo`] and stored into `/usr/local/lib/ATO/containers`, which is managed
by [`containers-storage`]. When ATO [starts up][startup], it uses [`containers-storage`] to mount the images;
from there, they are remounted into `/usr/local/lib/ATO/rootfs` to ensure correct permissions and predictable path names.

[startup]: ../setup/ATO
[`skopeo`]: https://github.com/containers/skopeo
[`containers-storage`]: https://github.com/containers/storage
