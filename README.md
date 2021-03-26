# Attempt This Online
A clone of [Try It Online!](https://github.com/TryItOnline/tryitonline).

## Installation
### Fully Automated
Download and execute the [setup script](./setup/setup) **on a fresh install of Arch Linux**. Using the setup script in
any other distribution, or on an Arch Linux machine that has already had changes, is not supported.

Attempt It Online is designed to be run as a wholly packaged appliance and as such requires a dedicated virtual machine.
It cannot be run in Docker or similar. *If you do manage to get it working inside a container, please let me know as it
would be very useful for me.*

Upgrading is also not recommended beyond `pacman -Syu`, and there is no script for it. Instead, you should use an
Infrastructure As Code tool like [Terraform](https://terraform.io) to automatically provision and set up new virtual
machines with the new version (also using the setup script).

### Manually (not recommended)
**Warning:** All the code and configuration files in this repository are tuned exactly to a fresh Arch Linux setup and
you will need to change *a lot* of things to get it to work with a custom setup. Absolutely no changes will be made to
support any other system setups.

Read and follow the steps described in the source code of `setup/setup`, adjusting them to your setup as necessary.

## Security
See the [Security Policy](security.md).

## How it works
The best way to see how Attempt This Online works is by listing the steps taken between user button-press and output
display. This assumes ATO is being run with the default full setup.

- <s>User clicks run on the webpage</s>
- <s>Frontend concatenates header, body, and footer of program</s>
- <s>Browser</s> sends `POST` request to `https://ato.pxeger.com/api`
    - Request contains [msgpack](https://msgpack.org)-encoded data, which is a dictionary containing:
        - `language` (string): the name/identifier of the language
        - `code` (bytes): the program code to run
        - `input` (bytes): the input to give to the program
        - `arguments` (bytes): the command-line arguments to run the program with
        - `options` (bytes): the command-line arguments to give to the language compiler or interpreter
- `nginx` server receives the request
- `nginx` scans the request for malicious content using `modsecurity`
- `nginx` forwards the request to the Python API server over the `/run/ATO.sock` Unix socket
- `uvicorn` interprets the request and calls the `starlette` server using [ASGI](https://asgi.readthedocs.io)
- `starlette` API server runs the `execute` endpoint
- msgpack request is decoded and validated using `pydantic`
- `run` function is called, with the invocation payload described above, a hashed identifier describing the client's IP
  address, and a random string identifying the individual request.
- Several Unix `pipe`s are created for the code, input, arguments, options, output, standard error output, timing
  information, and status code
- Code `fork`s and `dup2`s the pipes onto file descriptors in the range 40 to 46
- The `sandbox` wrapper script is executed as the user `sandbox` using `sudo`, which is passed the file descriptors,
  and, as arguments, the IP hash, request ID, and selected language.
- `sandbox` creates `cgroup`s according to the IP hash and invocation ID to keep track of the resource usage, and sets
  their resource limits
- `sandbox` creates an isolated [Bubblewrap](https://github.com/containers/bubblewrap) container in the `cgroup` which
  is run with a timeout
    - The command run in the container is `ATO_run`, which wraps the main runner to save the exit code
    - `ATO_run` executes the runner, which is a script dependant on the language requested
- `sandbox` cleans up the `cgroup`s
- API takes in the output, repacks it back into a response, and sends it back to the client (also via uvicorn and nginx)
- <s>The frontend decodes and lays out the result</s>

## Licence
© Patrick Reader 2021. Attempt This Online is licensed under the GNU Affero General Public License 3.0 - see the
[`LICENCE.txt`](./LICENCE.txt) file for more information.
