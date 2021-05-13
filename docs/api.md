# API Reference
The official instance uses the base url `https://ato.pxeger.com`. Note that use of the API on the
official instance must abide by the [Terms of Use](https://ato.pxeger.com/legal#terms-of-use):
**you must have explicit permission from me to use the API**.

## POST `/api/v0/execute`
### Request
A [msgpack]-encoded payload - a map with the following string keys:
- `language`: the identifier of the language interpreter or compiler to use. The identifier is a filename from the
  [`runners/` directory]
- `code`: a binary containing the program data
- `input`: a binary containing the data to be passed to the standard input of the program
- `options`: an array of binaries - command-line arguments to be passed to the **interpreter or compiler**
- `arguments`: an array of binaries - command-line arguments to be passed to the **program itself**

### Response
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
    - `killed`: the number of the signal that killed the process (see [`signal(7)`])
    - `core_dumped`: the number of the signal that caused the process to dump its core (see [`signal(7)`], [`core(5)`])
    - `unknown`: always `-1`
- `timed_out`: whether the process had to be killed because it overran its 60 second timeout. If this is the case,
  the process will have been killed by `SIGKILL` (ID 9)
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

[msgpack]: https://msgpack.org
[`runners/` directory]: https://github.com/pxeger/attempt_this_online/tree/main/runners
[`signal(7)`]: https://man.archlinux.org/man/core/man-pages/signal.7.en
[`core(5)`]: https://man.archlinux.org/man/core/man-pages/core.5.en
