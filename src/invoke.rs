#![feature(io_error_more, let_chains)]

mod constants;
mod languages;
use capctl::{caps, prctl};
use crate::{constants::*, languages::*};
use clone3::Clone3;
use close_fds::close_open_fds;
use hex::ToHex;
use nix::{
    dir::Dir,
    errno::Errno,
    fcntl::{fcntl, FcntlArg, OFlag},
    mount::{mount, MsFlags},
    poll::{PollFd, PollFlags, poll},
    sys::{
        eventfd::{eventfd, EfdFlags},
        resource::{getrusage, setrlimit, Resource, UsageWho::RUSAGE_CHILDREN},
        stat::Mode,
        time::TimeValLike,
        wait::{self, waitid, WaitPidFlag, WaitStatus::*},
    },
    unistd::{chdir, close, dup2, execve, Gid, mkdir, pipe, pivot_root, read, setresgid, setresuid, symlinkat, Uid},
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use std::ffi::{CStr, CString};
use std::cmp::min;
use std::fs::File;
use std::path::PathBuf;

/// like the ? postfix operator, but formats errors to strings
macro_rules! check {
    ($x:expr, $f:literal $(, $($a:expr),+)? $(,)?) => {
        $x.map_err(|e| Some(format!($f, $($($a,)*)? e)))?
    };
    ($x:expr $(,)?) => {
        $x.map_err(|e| Some(e.to_string()))?
    }
}

/// log `Err`s to stderr but don't stop execution
macro_rules! check_continue {
    ($x:expr, $f:literal $(, $($a:expr),+)? $(,)?) => {
        if let Err(e) = $x {
            eprintln!($f, $($($a,)*)? e);
        }
    };
    ($x:expr $(,)?) => {
        if let Err(e) = $x {
            eprintln!("{e}");
        }
    }
}

/// convert a string literal into a C string object
macro_rules! cstr {
    ($x:literal) => {
        unsafe { CStr::from_bytes_with_nul_unchecked(concat!($x, "\0").as_bytes()) }
    }
}

#[derive(Serialize)]
enum StreamResponse {
    Stdout(ByteBuf),
    Stderr(ByteBuf),
    Done {
        timed_out: bool,
        status_type: &'static str,
        status_value: i32,
        real: i64,
        kernel: i64,
        user: i64,
        max_mem: i64,
        waits: i64,
        preemptions: i64,
        major_page_faults: i64,
        minor_page_faults: i64,
        input_ops: i64,
        output_ops: i64,
    }
}

fn output_message(message: StreamResponse) -> Result<(), Option<String>> {
    let encoded_message = check!(rmp_serde::to_vec_named(&message));
    // to ensure packeted writes do not get split up, they must be <= PIPE_BUF
    // see pipe(2) § O_DIRECT and pipe(7) § PIPE_BUF
    assert!(encoded_message.len() <= PIPE_BUF);
    match nix::unistd::write(STDOUT_FD, &encoded_message) {
        Ok(_) => Ok(()),
        Err(Errno::EPIPE) => {
            // client went away
            Err(None)
        }
        Err(e) => {
            Err(Some(format!("error writing message: {e}")))
        }
    }
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Request {
    language: String,
    code: ByteBuf,
    input: ByteBuf,
    arguments: Vec<ByteBuf>,
    options: Vec<ByteBuf>,
    #[serde(default = "default_timeout")]
    timeout: i32,
}

fn default_timeout() -> i32 { 60 }

struct PacketRead {
    fd: i32,
    buf: [u8; PIPE_BUF],
    buf_len: usize,
    buf_pos: usize,
}

impl PacketRead {
    const fn new(fd: i32) -> Self {
        PacketRead {
            fd,
            buf: [0; PIPE_BUF],
            buf_len: 0,
            buf_pos: 0,
        }
    }
}

impl std::io::Read for PacketRead {
    fn read(&mut self, out: &mut [u8]) -> Result<usize, std::io::Error> {
        let mut total = 0;
        // TODO: chunking here makes an effort to fill all of `out`. Is that what we want?
        for chunk in out.chunks_mut(PIPE_BUF) {
            assert!(self.buf_pos <= self.buf_len);
            if self.buf_pos == self.buf_len {
                // refill buffer
                self.buf_len = read(self.fd, &mut self.buf).map_err(Into::<std::io::Error>::into)?;
                self.buf_pos = 0;
            }
            let amount_read = min(chunk.len(), self.buf_len - self.buf_pos);
            total += amount_read;
            chunk[..amount_read].copy_from_slice(&self.buf[self.buf_pos..][..amount_read]);
            self.buf_pos += amount_read;
        }
        Ok(total)
    }
}

fn main() -> std::process::ExitCode {
    // some care must be taken over error messages - see comments in run_child

    // note also that throughout the code, the Error variant of our Result types is Option<String>
    // when this is Some("..."), it's a normal error message
    // but when this is None, it means there was an "expected" error:
    // we need to exit the program because of it
    // but nothing's actually gone wrong, so it's not logged
    // so far this only happens when the client goes away during execution

    // use stdout in "packet" mode - see pipe(2) § O_DIRECT
    // this ensures that calls to write(2) correspond one-to-one with websocket messages.
    // and also use stdin in packet mode:
    // this ensures that calls to read(2) never return the contents of more than one websocket message
    // (although incoming websocket messages may be spread across multiple pipe packets and thus require multiple reads)
    // This prevents both:
    // - data loss from multiple messages being put into one read, with messages after the first being ignored
    // - "slow loris" attacks - see https://github.com/attempt-this-online/attempt-this-online/issues/88
    // Note that this means read(2)s from stdin must always ask for at least PIPE_BUF bytes;
    // otherwise, the remained of a packet may be silently discarded.
    // PacketRead contains an implementation of Read which provides this guarantee
    for (name, fd) in [("stdout", STDOUT_FD), ("stdin", STDIN_FD)] {
        if let Err(e) = fcntl(fd, FcntlArg::F_SETFL(OFlag::O_DIRECT)) {
            eprintln!("error setting {name} pipe to O_DIRECT: {e}");
            return std::process::ExitCode::from(INTERNAL_ERROR)
        }
    }

    let mut packet_stdin: PacketRead = PacketRead::new(STDIN_FD);

    let request = match rmp_serde::from_read::<_, Request>(&mut packet_stdin) {
        Ok(r) => r,
        Err(e) => {
            use rmp_serde::decode::Error::*;
            let why =
                if let InvalidMarkerRead(ref ei) | InvalidDataRead(ref ei) = e
                    && ei.kind() != std::io::ErrorKind::UnexpectedEof {
                    eprintln!("error reading request: {e}");
                    INTERNAL_ERROR
                } else {
                    eprintln!("invalid request: {e}");
                    POLICY_VIOLATION
                };
            return std::process::ExitCode::from(why);
        }
    };
    let language = match validate(&request) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("invalid request: {e}");
            return std::process::ExitCode::from(POLICY_VIOLATION);
        }
    };
    if let Err(Some(e)) = invoke(packet_stdin, &request, language) {
        eprintln!("{e}");
        return std::process::ExitCode::from(INTERNAL_ERROR);
    } else {
        return std::process::ExitCode::from(NORMAL);
    }
}

enum ValidationError<'a> {
    NoSuchLanguage(&'a String),
    NullByteInArgument,
    InvalidTimeout(i32),
}

impl<'a> std::fmt::Display for ValidationError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::NoSuchLanguage(name) => write!(f, "no such language: {}", name),
            ValidationError::NullByteInArgument => write!(f, "argument contains null byte"),
            ValidationError::InvalidTimeout(val) => write!(f, "timeout not in range 1-60: {}", val),
        }
    }
}

fn validate(request: &Request) -> Result<&Language, ValidationError> {
    if request.timeout < 1 || request.timeout > 60 {
        return Err(ValidationError::InvalidTimeout(request.timeout));
    }
    for arg in request.options.iter().chain(request.arguments.iter()) {
        if arg.contains(&0) {
            return Err(ValidationError::NullByteInArgument);
        }
    }
    if let Some(l) = LANGUAGES.get(&request.language) {
        Ok(l)
    } else {
        Err(ValidationError::NoSuchLanguage(&request.language))
    }
}

const STDIN_FD: std::os::unix::io::RawFd = 0;
const STDOUT_FD: std::os::unix::io::RawFd = 1;
const STDERR_FD: std::os::unix::io::RawFd = 2;

struct Cgroup<'a> {
    cgroup: &'a PathBuf,
}

impl Drop for Cgroup<'_> {
    fn drop(&mut self) {
        // clean up this cgroup by killing all its processes and then removing it
        if let Err(e) = std::fs::write(self.cgroup.join("cgroup.kill"), "1") {
            eprintln!("error killing cgroup: {e}");
            return
        }

        const CGROUP_REMOVE_MAX_ATTEMPT_TIME: u128 = 100; // ms

        let timer = std::time::Instant::now();
        let mut attempt_counter = 0;
        // note: even though the cgroup directory is not empty, the kernel wants us to just use rmdir on it
        // (because we can't remove any of the special files inside individually)
        while let Err(e) = std::fs::remove_dir(&self.cgroup) {
            if e.kind() == std::io::ErrorKind::ResourceBusy {
                // cgroup not finished dying yet: retry?
                let elapsed = timer.elapsed().as_millis();
                if elapsed < CGROUP_REMOVE_MAX_ATTEMPT_TIME {
                    attempt_counter += 1;
                    std::thread::yield_now();
                    continue;
                } else {
                    eprintln!("giving up removing cgroup after {elapsed}ms and {attempt_counter} attempts");
                }
            } else {
                eprintln!("error removing cgroup: {e}");
            }
            break;
        }
    }
}

fn create_cgroup() -> Result<PathBuf, Option<String>> {
    // TODO: dynamically work out the cgroup path
    // const CGROUP_PATH: &str = "/sys/fs/cgroup/system.slice/ATO.service";
    const CGROUP_PATH: &str = "/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service/ATOtest";
    let mut path = PathBuf::from(CGROUP_PATH);
    path.push(random_id());
    check!(std::fs::create_dir(&path), "error creating cgroup dir: {}");
    Ok(path)
}

fn setup_cgroup(path: &PathBuf) -> Result<(), Option<String>> {
    // this sets some resource limits, but the others are set with ordinary POSIX rlimits:
    // see the set_resource_limits function
    const MEMORY_HIGH: u64 = 200 * MiB;
    const MEMORY_MAX: u64 = 256 * MiB;
    check!(std::fs::write(path.join("memory.high"), MEMORY_HIGH.to_string()), "error writing cgroup memory.high: {}");
    check!(std::fs::write(path.join("memory.max"), MEMORY_MAX.to_string()), "error writing cgroup memory.max: {}");
    // disable swap
    check!(std::fs::write(path.join("memory.swap.max"), "0"), "error writing cgroup memory.swap.max: {}");
    Ok(())
}

const RANDOM_ID_SIZE: usize = 16;

fn random_id() -> String {
    rand::thread_rng().gen::<[u8; RANDOM_ID_SIZE]>().encode_hex::<String>()
}

fn invoke(packet_stdin: PacketRead, request: &Request, language: &Language) -> Result<(), Option<String>> {
    let cgroup = create_cgroup()?;
    let cgroup_cleanup = Cgroup{ cgroup: &cgroup };
    setup_cgroup(&cgroup)?;
    let cgroup_fd = check!(
        Dir::open(&cgroup, OFlag::O_DIRECTORY | OFlag::O_PATH, Mode::empty()),
        "error opening cgroup dir: {}",
    );

    let (stdout_r, stdout_w) = check!(pipe(), "error creating stdout pipe: {}");
    let (stderr_r, stderr_w) = check!(pipe(), "error creating stderr pipe: {}");

    let uid = Uid::current();
    let gid = Gid::current();

    let mut pidfd = -1;
    let mut clone3 = Clone3::default();
    clone3
        .flag_clear_sighand()
        .flag_newcgroup()
        .flag_newipc()
        .flag_newns()
        .flag_newnet()
        .flag_newpid()
        .flag_newuser()
        .flag_newuts()
        .flag_pidfd(&mut pidfd)
        .flag_into_cgroup(&cgroup_fd)
        ;
    let timer = std::time::Instant::now();
    // this is safe because we haven't used more than one thread so far in this program
    if check!(unsafe { clone3.call() }, "error clone3ing main child: {}") == 0 { // in child
        // avoid suicide
        std::mem::forget(cgroup_cleanup);

        // close unused pipe ends to ensure proper synchronisation
        // TODO: do we need to explicitly close these read pipe ends, given the close_range call below?
        check_continue!(close(stdout_r), "error closing stdout read end: {}");
        check_continue!(close(stderr_r), "error closing stderr read end: {}");

        run_child(&request, &language, stdout_w, stderr_w, uid, gid);
        // run_child should never return if successful, so we exit assuming failure
        std::process::exit(2);
    } else { // in parent
        // close unused pipe ends
        check!(close(stdout_w), "error closing stdout write end: {}");
        check!(close(stderr_w), "error closing stderr write end: {}");

        run_parent(packet_stdin, stdout_r, stderr_r, pidfd, cgroup_cleanup, timer, request.timeout)
    }
}

fn run_parent(
    mut packet_stdin: PacketRead,
    stdout_r: i32,
    stderr_r: i32,
    pidfd: i32,
    cgroup_cleanup: Cgroup,
    timer: std::time::Instant,
    timeout: i32,
) -> Result<(), Option<String>> {
    // eventfd is basically a condition variable, but which we can poll on
    let quit = check!(eventfd(0, EfdFlags::empty()), "error creating eventfd: {}");

    let output_handler = std::thread::spawn(move || handle_output(stdout_r, stderr_r, quit));

    let timed_out = {
        // use a poll to wait for either:
        // - timeout to expire
        // - child to exit
        // - client to request us to kill the child
        let mut poll_args = [
            // pidfd fires a POLLIN event when the process finishes
            PollFd::new(pidfd, PollFlags::POLLIN),
            // when a websocket message is received
            PollFd::new(STDIN_FD, PollFlags::POLLIN),
        ];
        let poll_result = check!(poll(&mut poll_args, timeout * 1000 /* ms */), "error polling: {}");
        let [poll_child, poll_stdin] = poll_args;
        if poll_result == 0 {
            // timed out
            true
        } else if poll_child.revents().ok_or(Some("poll returned unexpected event".into()))?.contains(PollFlags::POLLIN) {
            // child finished
            false
        } else {
            let stdin_events = poll_stdin.revents().ok_or(Some("poll returned unexpected event".into()))?;
            if stdin_events.contains(PollFlags::POLLIN) {
                // received control message via stdin
                #[derive(Deserialize)]
                enum ControlMessage {
                    Kill,
                }
                use ControlMessage::*;
                match rmp_serde::from_read::<_, ControlMessage>(&mut packet_stdin) {
                    Err(e) => {
                        return Err(Some(format!("error reading control message: {e}")));
                    }
                    Ok(Kill) => {
                        // continue to drop (i.e. kill), and set timed_out = false
                        false
                    }
                    // Ok(_) => ...
                }
            } else if stdin_events.contains(PollFlags::POLLHUP) {
                // client disappeared: continue to kill
                false
            } else {
                return Err(Some(format!("unexpected poll result: {poll_result}, {poll_args:?}")));
            }
        }
    };

    // kill process
    drop(cgroup_cleanup);

    check!(nix::unistd::write(quit, &1u64.to_ne_bytes()), "error writing to quit eventfd: {}");

    if let Err(e) = output_handler.join() {
        // output handler panicked, so do likewise
        std::panic::panic_any(e);
    }

    // TODO: investigate why this reports ECHILD if the child errors and __WALL is not provided
    // something to do with the fact we use a second thread above which is a different kind of child process?
    // or the rust threading runtime doing strange things to signal handlers?
    // https://github.com/torvalds/linux/blob/a63f2e7cb1107ab124f80407e5eb8579c04eb7a9/kernel/exit.c#L968
    let wait_result = check!(waitid(wait::Id::PIDFd(pidfd), WaitPidFlag::WEXITED | WaitPidFlag::__WALL), "error getting waitid result: {}");

    let (status_type, status_value) =
        match wait_result {
            Exited(_, c) => ("exited", c),
            Signaled(_, c, false) => ("killed", c as i32),
            Signaled(_, c, true) => ("core_dumped", c as i32),
            x => {
                eprintln!("warning: unexpected waitid result: {x:?}");
                ("unknown", -1)
            }
        };

    let stats = check!(getrusage(RUSAGE_CHILDREN), "error getting resource usage: {}");

    output_message(StreamResponse::Done {
        timed_out,
        status_type,
        status_value,
        real: timer.elapsed().as_nanos() as i64,
        kernel: stats.system_time().num_nanoseconds(),
        user: stats.user_time().num_nanoseconds(),
        max_mem: stats.max_rss(),
        waits: stats.voluntary_context_switches(),
        preemptions: stats.involuntary_context_switches(),
        major_page_faults: stats.major_page_faults(),
        minor_page_faults: stats.minor_page_faults(),
        input_ops: stats.block_reads(),
        output_ops: stats.block_writes(),
    })?;
    Ok(())
}

fn handle_output(stdout_r: i32, stderr_r: i32, quit: i32) -> Result<(), Option<String>> {
    for (name, pipe) in [("stdout", stdout_r), ("stderr", stderr_r)] {
        check!(fcntl(pipe, FcntlArg::F_SETFL(OFlag::O_NONBLOCK)), "error setting O_NONBLOCK on {} read end: {}", name);
    }

    const MAX_SENSIBLE_OUTPUT_SIZE: usize = 1 * MiB as usize;
    let mut totals = [0usize; 2];

    let mut poll_arg = [
        PollFd::new(quit, PollFlags::POLLIN),
        PollFd::new(stdout_r, PollFlags::POLLIN),
        PollFd::new(stderr_r, PollFlags::POLLIN),
    ];
    loop {
        check!(poll(&mut poll_arg, -1 /* infinite timeout */), "error polling for output: {}");
        let [poll_quit, poll_stdout, poll_stderr] = poll_arg;
        type StreamId = fn(ByteBuf) -> StreamResponse;
        for (i, name, poll, pipe, stream_id) in [
            (0, "stdout", poll_stdout, stdout_r, StreamResponse::Stdout as StreamId),
            (1, "stderr", poll_stderr, stderr_r, StreamResponse::Stderr as StreamId),
        ] {
            if poll.revents().ok_or(Some("poll returned unexpected event".into()))?.contains(PollFlags::POLLIN) {
                let mut buf = [0u8; OUTPUT_BUF_SIZE];
                let len = check!(read(pipe, &mut buf), "error reading from {}: {}", name);
                totals[i] += len;
                if totals[i] > MAX_SENSIBLE_OUTPUT_SIZE {
                    todo!("can't handle too much output yet")
                }
                let message = stream_id(ByteBuf::from(&buf[..len]));
                output_message(message)?;
            }
        }
        if poll_quit.revents().ok_or(Some("poll returned unexpected event".into()))?.contains(PollFlags::POLLIN) {
            return Ok(())
        }
    }
}

fn run_child(
    request: &Request,
    language: &Language,
    stdout_w: i32,
    stderr_w: i32,
    outside_uid: Uid,
    outside_gid: Gid,
) -> (/* never returns on success */) {
    // to have reliable error reporting, the state of stdout and stderr must be managed carefully:

    // also, error messages that don't go via the web server's error handling logic (i.e., which look like just stdout/stderr messages)
    //  should be prefixed with "ATO internal error" to make it clear they're not from the user's program

    // here, stdout points "directly" to the client websocket, so we mustn't print junk to stdout.
    // stderr points to the web server's error handling logic (which puts them in the websocket close messages and the systemd log)
    // so we log errors to stderr only

    // replace current stdout with the pipe we created for it
    if let Err(e) = dup2(stdout_w, STDOUT_FD) {
        eprintln!("error dup2ing stdout: {e}");
        return
    }
    // stdout now points to a pipe that the parent handles, so we messages to it will reach the user safely and we don't need to worry about junk.
    // so we should log errors to both stderr and stdout
    macro_rules! eoprintln {
        ($($x:expr),*) => {
            // goes to handle_output and therefore looks like a normal stdout message, so must be prefixed
            println!("ATO internal error: {}", format!($($x,)*));
            // goes via web server logic, so ok alone
            eprintln!($($x,)*);
        }
    }

    let env = match load_env(&language) {
        Ok(r) => r,
        Err(e) => {
            if let Some(e) = e {
                eoprintln!("{e}");
            }
            return
        }
    };

    if let Err(e) = setup_child(&request, &language, outside_uid, outside_gid) {
        if let Some(e) = e {
            eoprintln!("{e}");
        }
        return
    }

    if let Err(e) = dup2(stderr_w, STDERR_FD) {
        eoprintln!("error dup2ing stderr: {e}");
        return
    }
    // stderr now points to handle_output too; the web server's error handling logic is now useless.
    // From here on out, we log errors to stderr only, because logging to both would cause pointless duplication

    // close all remaining FDs except STDIO (0/1/2) - this includes dangling stdxxx_w pipes
    const FIRST_NON_STDIO_FD: i32 = 3;
    // this is safe because it's right before an exec
    unsafe { close_open_fds(FIRST_NON_STDIO_FD, &[]) } // should never error

    if let Err(e) = execve(cstr!("/ATO/runner"), &[cstr!("/ATO/runner")], &env) {
        eprintln!("ATO internal error: error running execve: {e}")
    } else {
        eprintln!("ATO internal error: execve should never return if successful")
    }
}

fn load_env(language: &Language) -> Result<Vec<CString>, Option<String>> {
    const ENV_BASE_PATH: &str = "/usr/local/lib/ATO/env/";
    let path = String::from(ENV_BASE_PATH) + &language.image.replace("/", "+");
    check!(std::fs::read(path), "error reading image env file: {}")
        .split_inclusive(|b| *b == 0) // split after null bytes, and include them in the results
        .map(|s| CString::from_vec_with_nul(s.to_vec()))
        .collect::<Result<Vec<_>, _>>() // collects errors too
        .map_err(|e| Some(format!("error building env string: {e}")))
}

fn setup_child(
    request: &Request,
    language: &Language,
    outside_uid: Uid,
    outside_gid: Gid,
) -> Result<(), Option<String>> {
    const SIGKILL: i32 = 9;
    // set up to die if our parent dies
    check!(prctl::set_pdeathsig(Some(SIGKILL)), "error setting parent death signal: {}");

    set_ids(outside_uid, outside_gid)?;
    setup_filesystem(&request, &language)?;
    drop_caps()?;
    set_resource_limits()?;
    Ok(())
}

fn set_ids(outside_uid: Uid, outside_gid: Gid) -> Result<(), Option<String>> {
    // we're currently nobody (65534) inside the container which means we can't do anything.
    // so we declare ourselves root inside the container, but this requires a mapping for who we become from the perspective of processes outside the container

    const ROOT_U: Uid = Uid::from_raw(0);
    const ROOT_G: Gid = Gid::from_raw(0);

    let uid_map = format!("{ROOT_U} {outside_uid} 1\n");
    check!(std::fs::write("/proc/self/uid_map", uid_map), "error writing uid_map: {}");
    // we need to set this in order to be able to use gid_map
    check!(std::fs::write("/proc/self/setgroups", "deny"), "error denying setgroups: {}");
    let gid_map = format!("{ROOT_G} {outside_gid} 1\n");
    check!(std::fs::write("/proc/self/gid_map", gid_map), "error writing gid_map: {}");

    check!(setresuid(ROOT_U, ROOT_U, ROOT_U), "error setting UIDs: {}");
    check!(setresgid(ROOT_G, ROOT_G, ROOT_G), "error setting GIDs: {}");
    Ok(())
}

macro_rules! mount_ {
    ($src:expr, $dest:expr, $type:expr, $($flag:ident)|*, $options:expr) => {
        check!(mount::<str, str, str, str>($src, $dest, $type, MsFlags::empty() $(| MsFlags::$flag)*, $options), "error mounting {}: {}", $dest)
    }
}
macro_rules! mount {
    ($dest:literal, $type:literal, $($flag:ident)|*) =>
        { mount_!(Some($type), $dest, Some($type), $($flag)|*, None) };
    ($dest:expr, $type:literal, $($flag:ident)|*, $options:literal) =>
        { mount_!(Some($type), $dest, Some($type), $($flag)|*, Some($options)) };
    ($src:expr, $dest:expr, $type:expr, $($flag:ident)|*) =>
        { mount_!(Some($src), $dest, Some($type), $($flag)|*, None) };
    ($src:expr, $dest:expr, , $($flag:ident)|*) =>
        { mount_!(Some($src), $dest, None, $($flag)|*, None) };
    ($src:expr, $dest:expr, $type:expr, $($flag:ident)|*, $options:literal) =>
        { mount_!(Some($src), $dest, Some($type), $($flag)|*, Some($options)) };
}


fn get_rootfs(language: &Language) -> String {
    const IMAGE_BASE_PATH: &str = "/usr/local/lib/ATO/rootfs/";
    String::from(IMAGE_BASE_PATH) + &language.image.replace("/", "+")
}

fn get_runner(language_id: &String) -> String {
    const LANGUAGE_BASE_PATH: &str = "/usr/local/share/ATO/runners/";
    String::from(LANGUAGE_BASE_PATH) + language_id
}

fn setup_filesystem(request: &Request, language: &Language) -> Result<(), Option<String>> {
    let rootfs = get_rootfs(&language);

    // set the propogation type of all mounts to private - this is because:
    // 1. when we bind-mount stuff we don't want that to propogate to the parent namespace
    // 2. we don't want any potential mounts in the parent namespace to appear here and mess things up either
    // 2. pivot_root below requires . and its parent to be mounted private anyway
    check!(mount::<str, str, str, str>(
        None,
        "/",
        None,
        MsFlags::MS_PRIVATE | MsFlags::MS_REC,
        None,
    ), "error setting / to MS_PRIVATE: {}");

    // bind-mount rootfs onto itself for two reasons:
    // 1. the kernel now considers it a mount point, which is required for pivot_root to work
    // 2. we can make it read-only
    check!(mount::<str, str, str, str>(
        Some(&rootfs),
        &rootfs,
        None,
        MsFlags::MS_BIND | MsFlags::MS_RDONLY | MsFlags::MS_REC,
        None,
    ), "error bind-mounting new rootfs onto itself: {}");

    check!(chdir::<str>(&rootfs), "error changing directory to new rootfs: {}");
    // now . points to the new rootfs

    setup_special_files(&request.language)?;

    // swap (or "pivot") the meanings of / and .
    // so now, / points to the new container rootfs, and . points to the old system root
    // (note that this means . is not actually anywhere in the directory tree!)
    check!(pivot_root(".", "."), "error pivoting root: {}");

    setup_request_files(&request)?;

    // cwd after pivot_root is not well-defined, so we have to go somewhere
    // since we need to go to /ATO at some point anyway, let's got there
    check!(chdir("/ATO"), "error changing directory to /ATO: {}");
    Ok(())
}

fn setup_special_files(language_id: &String) -> Result<(), Option<String>> {
    mount!("./ATO", "tmpfs", MS_NOSUID | MS_NODEV, "mode=755,size=65535k");
    check!(mkdir("./ATO/context", Mode::S_IRWXU | Mode::S_IRGRP | Mode::S_IXGRP | Mode::S_IROTH | Mode::S_IXOTH), "error creating /ATO/context: {}");
    mount!("./proc", "proc",);
    mount!("./dev", "tmpfs", MS_NOSUID | MS_STRICTATIME, "mode=755,size=65535k");
    check!(mkdir("./dev/pts", Mode::empty()), "error creating mount point for /dev/pts: {}");
    mount!("./dev/pts", "devpts", MS_NOSUID | MS_NOEXEC, "newinstance,ptmxmode=0666,mode=0620");
    check!(mkdir("./dev/shm", Mode::empty()), "error creating mount point for /dev/shm: {}");
    mount!("shm", "./dev/shm", "tmpfs", MS_NOSUID | MS_NODEV | MS_NOEXEC, "mode=1777,size=65536k");
    check!(mkdir("./dev/mqueue", Mode::empty()), "error creating mount point for /dev/mqueue: {}");
    mount!("./dev/mqueue", "mqueue", MS_NOSUID | MS_NODEV | MS_NOEXEC);
    mount!("./sys", "sysfs", MS_NOSUID | MS_NODEV | MS_NOEXEC);
    mount!("./sys/fs/cgroup", "cgroup2", MS_NOSUID | MS_NODEV | MS_NOEXEC | MS_RELATIME);

    // create all the special device files expected on a "real" Linux system

    for dev in ["full", "null", "random", "tty", "urandom", "zero"] {
        let src = "/dev/".to_owned() + dev;
        let dest = "./dev/".to_owned() + dev;
        drop(check!(File::create(&dest), "error creating mount point for /dev/{}: {}", dev));
        mount!(&src, &dest, , MS_NOSUID | MS_NOEXEC | MS_BIND);
    }

    check!(symlinkat("/proc/self/fd", None, "dev/fd"), "error creating /dev/fd: {}");
    check!(symlinkat("/proc/self/fd/0", None, "dev/stdin"), "error creating /dev/stdin: {}");
    check!(symlinkat("/proc/self/fd/1", None, "dev/stdout"), "error creating /dev/stdout: {}");
    check!(symlinkat("/proc/self/fd/2", None, "dev/stderr"), "error creating /dev/stderr: {}");
    check!(symlinkat("/proc/kcore", None, "dev/core"), "error creating /dev/core: {}");
    check!(symlinkat("pts/ptmx", None, "dev/ptmx"), "error creating /dev/ptmx: {}");

    for (src, dest) in [
        ("/usr/local/lib/ATO/bash", "./ATO/bash"),
        ("/usr/local/lib/ATO/yargs", "./ATO/yargs"),
        (&get_runner(&language_id), "./ATO/runner"),
    ] {
        drop(check!(File::create(&dest), "error creating mount point for {}: {}", dest));
        mount!(&src, &dest, , MS_NOSUID | MS_BIND | MS_RDONLY);
    }

    Ok(())
}

fn setup_request_files(request: &Request) -> Result<(), Option<String>> {
    check!(std::fs::write("/ATO/code", &request.code), "error writing /ATO/code: {}");
    // TODO: stream input in instead of writing it to a file?
    check!(std::fs::write("/ATO/input", &request.input), "error writing /ATO/input: {}");
    check!(std::fs::write("/ATO/arguments", join_args(&request.arguments)), "error writing /ATO/arguments: {}");
    check!(std::fs::write("/ATO/options", join_args(&request.options)), "error writing /ATO/options: {}");
    Ok(())
}

fn join_args(args: &Vec<ByteBuf>) -> ByteBuf {
    let mut b = ByteBuf::new();
    for a in args {
        b.extend(a);
        b.extend([0]);
    }
    b
}

fn drop_caps() -> Result<(), Option<String>> {
    // drop as many privileges as possible
    // for more info on caps, see man capabilities(7)

    // important: make sure this code is written to drop all capabilities, even those we may not
    // know about at compile time because of a kernel version mismatch.
    // https://docs.rs/capctl/latest/capctl/#handling-of-newly-added-capabilities

    check!(caps::bounding::clear(), "error dropping bounding capabilities: {}");
    check!(caps::ambient::clear(), "error dropping ambient capabilities: {}");
    // CAP_SETPCAP is required to drop bounding caps, so we have to drop it last
    check!(caps::CapState::empty().set_current(), "error dropping capabilities: {}");
    // ensure we can't get new privileges from files with set[ug]id or capability xattrs
    check!(prctl::set_no_new_privs(), "error setting NO_NEW_PRIVS flag: {}");
    Ok(())
}

fn set_resource_limits() -> Result<(), Option<String>> {
    // resource limits work as follows: each one has a "soft" and "hard" limit.
    // for most limits, the process will get an error if it goes beyond the soft limit;
    // the soft limit can be raised by the process but never to higher than the hard limit.
    // Note also that some resource limits are set on the cgroup level - see setup_cgroup

    // CPU time, set as a backup in case the cgroup-based timeout doesn't work
    // if the process reaches the soft limit it will get a SIGXCPU warning signal
    // if it reaches the hard limit it will be forcibly killed
    // TODO: does setting a CPU-time rlimit even work?
    check!(setrlimit(Resource::RLIMIT_CPU, 60, 61), "error setting CPU resource limit: {}");
    // number of processes/threads, to prevent exhaustion of kernel resources
    check!(setrlimit(Resource::RLIMIT_NPROC, 100, 100), "error setting NPROC resource limit: {}");
    // written file size, to prevent memory exhaustion by filling up a tmpfs
    check!(setrlimit(Resource::RLIMIT_FSIZE, 120 * MiB, 128 * MiB), "error setting FSIZE resource limit: {}");

    // these next resource limits are probably not particularly necessary:

    // pending signals
    check!(setrlimit(Resource::RLIMIT_SIGPENDING, 100, 100), "error setting SIGPENDING resource limit: {}");
    // locked files
    check!(setrlimit(Resource::RLIMIT_LOCKS, 100, 100), "error setting LOCKS resource limit: {}");
    // bytes in POSIX message queues
    check!(setrlimit(Resource::RLIMIT_MSGQUEUE, 100, 100), "error setting MSGQUEUE resource limit: {}");
    Ok(())
}
