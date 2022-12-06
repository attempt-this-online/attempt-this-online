# API Reference
The official instance uses the base url `https://ato.pxeger.com`. Note that use of the API on the
official instance must abide by the [Terms of Use](https://ato.pxeger.com/legal#terms-of-use):
**you must have explicit permission from me to use the API**.

## Websocket connect `/api/v1/ws/execute`
Socket flow looks like this:

- connect to websocket
- client sends request message
- server sends stdout and stderr messages
- server sends done message

All messages are of binary type.

The same connection can be reused for multiple requests, but only one request at a time. If a second request is sent during the
execution of the first request, it will be silently ignored.

The server will not close the connection unless the client does, or an error occurs.

Websocket close codes are:
- Normal closure (1000): everything was ok
- Unsupported data (1003): received a text message instead of a binary message
- Policy violation (1008): request was invalid; the reason may include extra info. Causes include:
    - invalid msgpack data
    - msgpack data did not match the schema of Message 1
    - an argument or an option contained a null byte
    - the given language did not exist
    - the timeout value was not in the range 1 to 60
- Message too big (1009): request exceeded the maximum size, which is currently 65536 bytes
- Internal server error (1011): something went wrong inside ATO

### Request Message
A [msgpack]-encoded payload - a map with the following string keys:
- `language`: the identifier of the language interpreter or compiler to use. The identifier is a filename from the
  [`runners/` directory]
- `code`: a binary containing the program data
- `input`: a binary containing the data to be passed to the standard input of the program
- `options`: an array of binaries - command-line arguments to be passed to the **interpreter or compiler**
- `arguments`: an array of binaries - command-line arguments to be passed to the **program itself**
- `timeout`: (optional) an integer which specifies the duration in seconds for which the program is allowed to run. Must
be less than or equal to 60. If not specified, 60 is used.

Typing is fairly lax; strings will be accepted in place of binaries (they will be encoded in UTF-8).

### Stdout and Stderr Messages
A [msgpack]-encoded payload - a map containing one key, `Stdout`, or `Stderr`, whose value is a binary containing a
chunk of the program's output to stdout or stderr.

### Done Message
A [msgpack]-encoded payload - a map containing one key, `Done`, whose value is another map with the following entries:
- `status_type`: the reason the process ended - one of:
    - `exited`: terminated normally by returning from `main` or calling `exit`
    - `killed`: terminated by a signal; only happens on timeout or if the process killed itself for some reason
    - `core_dumped`: core dumped, e.g. due to a segmentation fault
    - `unknown`: meaning of the value is not known; should never normally happen
- `status_value`: the status code of the end of the process. Its exact meaning depends on `status_type`:
    - `exited`: the exit code that the program returned
    - `killed`: the number of the signal that killed the process (see [`signal(7)`]). Might also be -1 due to technical
      limitations.
    - `core_dumped`: the number of the signal that caused the process to dump its core (see [`signal(7)`], [`core(5)`])
    - `unknown`: always `-1`
- `timed_out`: whether the process had to be killed because it overran its 60 second timeout. If this is the case, the
  process will have been killed by `SIGKILL` (ID 9)
- `real`: real elapsed time in nanoseconds
- `kernel`: CPU nanoseconds spent in kernel mode
- `user`: CPU nanoseconds spent in user mode
- `max_mem`: total maximum memory usage at any one time, in kilobytes
- `waits`: number of voluntary context switches
- `preemptions`: number of involuntary context switches
- `major_page_faults`: number of major page faults (where a memory page needed to be brought from the disk)
- `minor_page_faults`: number of minor page faults
- `input_ops`: number of input operations
- `output_ops`: number of output operations

[msgpack]: https://msgpack.org
[`runners/` directory]: https://github.com/attempt-this-online/attempt-this-online/tree/main/runners
[`signal(7)`]: https://man.archlinux.org/man/core/man-pages/signal.7.en
[`core(5)`]: https://man.archlinux.org/man/core/man-pages/core.5.en

## WebSocket API Example
Simple example of using WebSocket API from JavaScript. It corresponds to this [ATO link](https://ato.pxeger.com/run?1=m700OT49OXlVtJKuS6KtiZFS7IKlpSVpuhbbcxMz8zQ0qwuKMvNK0jSUVFOUdBI1rWshslBFC6A0AA).
It prints `stdout: 42` to the JavaScript console.

```javascript
// get `msgpack` from https://www.npmjs.com/package/@msgpack/msgpack
// use NPM or something

function ato_run()
{
	let socket = new WebSocket("wss://ato.pxeger.com/api/v1/ws/execute");
	socket.onopen = () => socket.send(msgpack.encode({
		language: "c_gcc", // get this from https://github.com/attempt-this-online/attempt-this-online/tree/main/runners
		code: 'main(){printf("%d",a);}',
		input: "",
		options: ["-Da=42"],
		arguments: [],
		timeout: 60,
	}));
    stdout = '';
    return new Promise(resolve => {
        socket.onmessage = async event => {
            let response = await msgpack.decodeAsync(event.data.stream());
            if ('Done' in response) {
                console.log(response.Done);
                resolve(stdout);
            }
            if ('Stdout' in response) {
                stdout += new TextDecoder().decode(response.stdout);
            }
        };
    });
}
```
