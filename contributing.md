# Contributing
## Adding Languages
If you're not familiar with the typical Github (fork-edit-PR) workflow, please read the [GitHub
guide](https://guides.github.com/introduction/flow/) on the matter.

1. Find a [Docker image](https://hub.docker.com) with the toolchain for the language
  - To minimise disk usage, try to use images with layers in common with existing languages. In particular, the common
  standard is Debian Buster, so use a `buster` tag where possible.
  - A lot of esoteric and golfing languages won't have Docker images. For this purpose, there is my
  [esoterics](https://hub.docker.com/r/pxeger/esoterics). Submit a pull request
  [here](https://github.com/pxeger/esoterics) to add the language to that image first, where possible.
  - Do not pin the image to a specific version. This helps keep languages continually up-to-date because whenever ATO
  is reinstalled, the newest version will be used. The exception to this is in cases of languages like Python 2, where
  the language has since changed enormously and the older version is deliberately wanted.
2. Create a runner script in `runners/`. Here is an example showing the general idea

```sh
#!/bin/sh

# The runner script's name will be used as an identifier. Don't include special characters or whitespace, and keep it
# lowercase. The name presented to the user is in this specially formatted line:
#:name: C (gcc)

# Replace with the Docker image and tag used - take it from the `docker pull` command provided on Docker Hub.
#:image: rikorose/gcc-cmake:latest

# There are no minimal requirements for the Docker image, as long as it doesn't contain a /ATO directory. If the Docker
# image you're using doesn't have a POSIX shell, it is always available as `/ATO/bash`. If you need to use it, make sure
# to change the `#!` # (shebang) line to match that.

# Do whatever is necessary to compile and run the code.
# - code is saved in /ATO/code
# - input provided by the user is saved in /ATO/input
# - options to pass to the compiler or interpreter are stored in /ATO/options, null-terminated
# - options to pass to the program itself are stored in /ATO/arguments

# Use /ATO/yargs to substitute in the command-line options: the first argument is the replacement string, the second is
# the intput file for the arguments, and after is the program and its arguments. The replacement string indicates the
# position of the substitution.
/ATO/yargs % /ATO/options gcc % /ATO/code -o /ATO/compiled

# Note that, while the script will always start in /ATO/, you should always use absolute paths.

# The code itself should always be run in the working directory /ATO/context
cd /ATO/context

# Pass arguments to the compiled file. Also, make sure you give the program input from /ATO/input.
/ATO/yargs % /ATO/arguments /ATO/compiled % < /ATO/input

# Make sure you retain the status code of the program! If you need to do any cleanup for whatever reason, make sure to
# store a copy of the exit code and use it again.
stored_status="$?"
do_cleanup
exit "$stored_status"
```

Here is another example runner for an interpreted language instead:

```sh
#!/bin/sh

#:name: Python
#:image: buildpack-deps

cd /ATO/context

# Use two levels of yargs to substitute in multiple sets of arguments:
/ATO/yargs %1 /ATO/options /ATO/yargs %2 /ATO/arguments python %1 /ATO/code %2 < /ATO/input
```

3. Test your runner! It's not helpful if you submit a broken runner.
4. Make a [Pull Request](https://github.com/pxeger/attempt_this_online/pulls)
