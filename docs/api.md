# API Reference
The official instance uses the base url `https://ato.pxeger.com`. Note that use of the API on the
official instance must abide by the [Terms of Use](https://ato.pxeger.com/legal#terms-of-use):
**you must have explicit permission from me to use the API**.

## Websocket connect `/api/v0/ws/execute`
Socket flow is pretty simple; the sequence of messages looks like this:

- connect to websocket
- client sends message 1 (as a binary message)
- server sends message 2 (as a binary message)
- server closes connection

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

### Message 1
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

### Message 2
A [msgpack]-encoded payload - a map with the following string keys:
- `stdout`: the standard output from the program and compilation (limited to 128 KiB)
- `stderr`: the standard error from the program and compilation (limited to 32 KiB)
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
<!--
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

## GET `/api/v0/metadata`
### Request
No parameters required.

### Response
A [msgpack]-encoded payload - a map from language IDs to a dictionary of their properties. For example (in JSON):

```json
{ "python": { "name": "Python", "image": "python:3-buster" } }
```

The language ID is the filename of a script in the [`runners/` directory], to be passed to the execute endpoint.

Currently recognised proprties are:
- `name` (string): human-readable name of the language
- `image` (string): Docker image used for execution of the language
- `version` (string): What version constraints ATO places on this language. (Not a guarantee)
- `url` (string): homepage URL for the language
- `sbcs` (boolean): true if the language's byte-counter should assume all characters are one byte long
- `se_class` (string, optional): language ID used for syntax highlighting when a StackExchange post is generated. If
  empty or not present, then language will have no syntax highlighting

-->

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
	let socket = new WebSocket("wss://ato.pxeger.com/api/v0/ws/execute");
	socket.onopen = () => socket.send(msgpack.encode({
		language: "c_gcc", // get this from https://github.com/attempt-this-online/attempt-this-online/tree/main/runners
		code: 'main(){printf("%d",a);}',
		input: "",
		options: ["-Da=42"],
		arguments: [],
		timeout: 60,
	}));
	socket.onmessage = async event => {
		let response = await msgpack.decodeAsync(event.data.stream());
		console.log("stdout:", new TextDecoder().decode(response.stdout));
	}
}
```
