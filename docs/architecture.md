# Architecture
The best way to see how Attempt This Online works is by listing the steps taken between user button-press and output
display. This assumes ATO is being run with the default full setup.

- User clicks run on the webpage
- Frontend concatenates header, body, and footer of program
- Browser sends `POST` request to `https://ato.pxeger.com/api`
    - Request contains [msgpack](https://msgpack.org)-encoded data, which is a dictionary containing:
        - `language` (string): the name/identifier of the language
        - `code` (bytes): the program code to run
        - `input` (bytes): the input to give to the program
        - `arguments` (bytes): the command-line arguments to run the program with
        - `options` (bytes): the command-line arguments to give to the language compiler or interpreter
        - `timeout` (int): the maximum number of seconds to run the program for
- `nginx` server receives the request
- <s>`nginx` scans the request for malicious content using `modsecurity`</s>
- `nginx` forwards the request to the Go API server over the local port `4568`
- `msgpack` request is decoded and validated
- `invoke` function is called, with the invocation payload described above, a hashed identifier describing the
  client's IP address, and a random string identifying the individual request
- The code, input, options, and arguments are written to files in `/run/ATO/{request_id}/` for the sandbox to read
- Two named pipes, `/run/ATO/{request_id}/stdout` and `.../stderr`, are created
- The `sandbox` wrapper script is executed which has, as arguments, the IP hash, request ID, selected language, image
  that the selected language needs, and the timeout for the execution in seconds
- `sandbox` creates `cgroup`s according to the IP hash and invocation ID to keep track of the resource usage, and sets
  their resource limits
- `sandbox` creates an isolated [Bubblewrap](https://github.com/containers/bubblewrap) container in the `cgroup`
    - The container has mounted:
         - `/` (the root file system): from `/usr/local/lib/ATO/rootfs`, an extracted Docker image containing the root
         file system for the relevant language. The extraction is done as part of the `setup/setup` script, and the
         layers are mounted using `overlayfs` and added to `/etc/fstab` by `setup/overlayfs_genfstab`
         - `/ATO/`: A `tmpfs` where the following few files will be put
         - `/ATO/bash`: A statically linked `/bin/bash` ([stolen from Debian](https://packages.debian.org/unstable/amd64/bash-static/download)),
         in case the language's Docker image doesn't have it
         - `/ATO/yargs`: a wrapper to execute a command with null-terminated arguments from a file
         - `/ATO/code` etc.: the input files from `/run/ATO/{request_id}` on the host
         - `/ATO/wrapper`
    - The command run in the container is `ATO_wrapper`, which wraps the main runner to save the exit code, track
    resource usage, and limit execution time to 60 seconds
    - `wrapper` executes the runner, which is a script dependent on the language requested
    - The standard output and standard error are passed, via `head` to limit the amount they can produce, and written to
    the named pipes in `/run/ATO/{request_id}/` (`stdout`, `stderr`)
    - `wrapper` writes its information in JSON format to `/run/ATO/{request_id}/status`
- `sandbox` cleans up the `cgroup`s
- `sandbox` grants the API permission to remove the `/run/ATO/{request_id}` files
- API takes in the output and status, adds the output to the status object to create a whole response which is packed
  again using `msgpack` and sent back to the client via `uvicorn` and `nginx`
- API cleans up the `/run/ATO/{request_id}` and `/run/ATO/{request_id}` files
- The frontend decodes and lays out the result

<!-- TODO: add links to all these things -->
