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
- Rust backend ([`main.rs`]) spawns the Rust sandbox program ([`invoke.rs`]), feeding the websocket request into its
  STDIN
- `msgpack` request is decoded and validated in `invoke.rs`
- `invoke` creates an isolated Linux container in a new [cgroup](https://docs.kernel.org/admin-guide/cgroup-v2.html)
    - The container has temporary files `/ATO/code`, `/ATO/input`, `/ATO/arguments`, `/ATO/options` created, containing
      the input values from the API request
    - It has `rlimit`s and some cgroup values set to limit resource usage
    - The container has mounted:
         - `/` (the root file system): from `/usr/local/lib/ATO/rootfs`, an extracted Docker image containing the root
         file system for the relevant language. The extraction is done as part of the `setup/setup` script, and the
         layers are mounted using `overlayfs` and added to `/etc/fstab` by `setup/overlayfs_genfstab`
         - Various other special Linux filesystems and files (`/proc`, `/sys`, `/dev/*`)
         - `/ATO/`: A `tmpfs` where the following few files will be put
         - `/ATO/bash`: A statically linked `/bin/bash` ([stolen from Debian](https://packages.debian.org/unstable/amd64/bash-static/download)),
         in case the language's Docker image doesn't have a bash
         - `/ATO/yargs`: a wrapper to execute a command with null-terminated arguments from a file
- `invoke` kills all processes remaining inside the container's cgroup, and removes the cgroup
- API takes in the output to create a whole response which is packed again using `msgpack` and sent back to the client
  via the server in `main.rs`, and `nginx`
- The frontend decodes and lays out the result

More details of the container created by `invoke` can be found by consulting its well-commented source code.

[`main.rs`]: ../src/main.rs
[`invoke.rs`]: ../src/invoke.rs
