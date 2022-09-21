#![feature(io_error_more)]

mod codes;
mod languages;
use capctl::{caps, prctl};
use crate::{codes::*, languages::*};
use clone3::Clone3;
use close_fds::close_open_fds;
use hex::ToHex;
use nix::{
    dir::Dir,
    fcntl::OFlag,
    mount::{mount, MsFlags},
    poll::{PollFd, PollFlags, poll},
    sys::{stat::Mode, resource::{setrlimit, Resource}},
    unistd::{chdir, close, dup2, execve, Gid, mkdir, pipe, pivot_root, setresgid, setresuid, symlinkat, Uid},
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::FromRawFd;
use std::path::PathBuf;

/// like println but writes to both stderr and stdout
macro_rules! eoprintln {
    ($($x:expr),*) => {
        println!($($x,)*);
        eprintln!($($x,)*);
    }
}

/// like the ? postfix operator, but formats errors to strings
macro_rules! check {
    ($x:expr, $f:literal $(, $($a:expr),+)? $(,)?) => {
        $x.map_err(|e| format!($f, $($($a,)*)? e))?
    };
    ($x:expr $(,)?) => {
        $x.map_err(|e| e.to_string())?
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
            eprintln!("{}", e);
        }
    }
}

/// convert a string literal into a C string object
macro_rules! cstr {
    ($x:literal) => {
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(concat!($x, "\0").as_bytes()) }
    }
}

#[derive(Serialize)]
struct Response {
    stdout: ByteBuf,
    stderr: ByteBuf,
    timed_out: bool,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Request {
    language: String,
    code: ByteBuf,
    input: ByteBuf,
    arguments: Vec<ByteBuf>,
    options: Vec<ByteBuf>,
    timeout: i32,
}

fn main() -> std::process::ExitCode {
    let request = match rmp_serde::from_read::<_, Request>(std::io::stdin()) {
        Ok(r) => r,
        Err(e) => {
            eoprintln!("decode error: {}", e);
            return std::process::ExitCode::from(POLICY_VIOLATION);
        }
    };
    let language = match validate(&request) {
        Ok(l) => l,
        Err(e) => {
            eoprintln!("invalid request: {}", e);
            return std::process::ExitCode::from(POLICY_VIOLATION);
        }
    };
    let result = match invoke(&request, language) {
        Ok(r) => r,
        Err(e) => {
            eoprintln!("internal error: {}", e);
            return std::process::ExitCode::from(INTERNAL_ERROR);
        }
    };
    let encoded_output = match rmp_serde::to_vec_named(&result) {
        Ok(r) => r,
        Err(e) => {
            eoprintln!("error encoding output: {}", e);
            return std::process::ExitCode::from(INTERNAL_ERROR);
        }
    };
    match std::io::stdout().write_all(&encoded_output[..]) {
        Ok(()) => std::process::ExitCode::from(NORMAL),
        Err(e) => {
            eoprintln!("error writing output: {}", e);
            std::process::ExitCode::from(INTERNAL_ERROR)
        }
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

const STDOUT_FD: std::os::unix::io::RawFd = 1;
const STDERR_FD: std::os::unix::io::RawFd = 2;

struct Cgroup<'a> {
    cgroup: &'a PathBuf,
}

impl Drop for Cgroup<'_> {
    fn drop(&mut self) {
        // clean up this cgroup by killing all its processes and then removing it
        if let Err(e) = std::fs::write(self.cgroup.join("cgroup.kill"), "1") {
            eprintln!("error killing cgroup: {}", e);
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
                    eprintln!("giving up removing cgroup after {}ms and {} attempts", elapsed, attempt_counter);
                }
            } else {
                eprintln!("error removing cgroup: {}", e);
            }
            break;
        }
    }
}

fn create_cgroup() -> Result<PathBuf, String> {
    // TODO: dynamically work out the cgroup path
    const CGROUP_PATH: &str = "/sys/fs/cgroup/system.slice/ATO.service";
    let mut path = PathBuf::from(CGROUP_PATH);
    path.push(random_id());
    check!(std::fs::create_dir(&path), "error creating cgroup dir: {}");
    Ok(path)
}

#[allow(non_upper_case_globals)]
const KiB: u64 = 1024;
#[allow(non_upper_case_globals)]
const MiB: u64 = KiB * KiB;

fn setup_cgroup(path: &PathBuf) -> Result<(), String> {
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

fn invoke(request: &Request, language: &Language) -> Result<Response, String> {
    let cgroup = create_cgroup()?;
    let cgroup_cleanup = Cgroup{ cgroup: &cgroup };
    setup_cgroup(&cgroup)?;
    let cgroup_fd = check!(
        Dir::open(&cgroup, OFlag::O_DIRECTORY | OFlag::O_PATH, Mode::empty()),
        "error opening cgroup dir: {}",
    );

    // Currently the child process will just block forever if it tries to write more than the kernel's maximum pipe buffer size to stdout or stderr
    // (this is because we only read from the pipes after the process has been killed).
    // (see man pipe(7) and fcntl(2) § F_SETPIPE_SZ for details on how this limit is defined)
    // We rely purely on this mechanism to prevent excessive output.
    // If the process gets to this point (where it's blocking forever), it will eventually time out and die.
    // This means the read_to_end calls below won't try to read an infinite or very large stream.
    // TODO: use a better way of limiting stdout / stderr size
    let (stdout_r, stdout_w) = check!(pipe(), "error creating stdout pipe: {}");
    let (stderr_r, stderr_w) = check!(pipe(), "error creating stderr pipe: {}");
    // const MAX_STDOUT_SIZE: u64 = 128 * KiB;
    // const MAX_STDERR_SIZE: u64 = 32 * KiB;
    // ...

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
    // this is safe because we only ever use one thread in this program
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

        // wait for the process to finish, but with the given timeout:
        // if the timeout expires before the process finishes, it will be killed when the cgroup struct is dropped
        let mut poll_arg = PollFd::new(pidfd, PollFlags::POLLIN);
        let mut poll_args = std::slice::from_mut(&mut poll_arg);
        // poll wants milliseconds; request.timeout is seconds
        let poll_result = check!(poll(&mut poll_args, request.timeout * 1000));
        let timed_out = poll_result == 0;

        // kill process
        drop(cgroup_cleanup);

        // these unsafe blocks are safe as long as we don't use stdout_r or stderr_r again
        let mut stdout_f = unsafe { File::from_raw_fd(stdout_r) };
        // dropping this has no runtime effect (as it's just an integer), but it makes it clear that we can't use it again
        drop(stdout_r);
        let mut stdout = Vec::new();
        check!(stdout_f.read_to_end(&mut stdout), "error reading stdout: {}");

        let mut stderr_f = unsafe { File::from_raw_fd(stderr_r) };
        drop(stderr_r);
        let mut stderr = Vec::new();
        check!(stderr_f.read_to_end(&mut stderr), "error reading stderr: {}");

        Ok(Response {
            stdout: ByteBuf::from(stdout),
            stderr: ByteBuf::from(stderr),
            timed_out,
        })
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

    // here, stdout points directly to the client websocket, so we mustn't print junk to stdout.
    // stderr points to the systemd log as normal, so we log errors to stderr only

    // replace current stdout with the pipe we created for it
    if let Err(e) = dup2(stdout_w, STDOUT_FD) {
        eprintln!("ATO internal error: error dup2ing stdout: {}", e);
        return
    }
    // stdout now points to a pipe that the parent handles, so we messages to it will reach the user safely and we don't need to worry about junk.
    // so we should log errors to both stderr and stdout

    if let Err(e) = setup_child(&request, &language, outside_uid, outside_gid) {
        eoprintln!("ATO internal error: {}", e);
        return
    }

    if let Err(e) = dup2(stderr_w, STDERR_FD) {
        eoprintln!("ATO internal error: error dup2ing stderr: {}", e);
        return
    }
    // stderr now points to the user; the systemd log is now unreachable
    // we log errors to stderr only because logging to both would cause pointless duplication

    // close all remaining FDs except STDIO (0/1/2), including dangling stdxxx_w pipes
    const FIRST_NON_STDIO_FD: i32 = 3;
    // this is safe because it's right before an exec
    unsafe { close_open_fds(FIRST_NON_STDIO_FD, &[]) } // should never error

    // TODO: wrap runner to gather statistics again?
    // TODO: load environment variables per image
    if let Err(e) = execve(cstr!("/ATO/runner"), &[cstr!("/ATO/runner")], &[cstr!("TODO=TODO")]) {
    // if let Err(e) = execve(cstr!("/bin/sh"), &[cstr!("bash"), cstr!("-c"), cstr!("ls -la /ATO")], &[cstr!("TODO=TODO")]) {
        eprintln!("ATO internal error: error running execve: {}", e)
    } else {
        eprintln!("ATO internal error: execve should never return if successful")
    }
}

fn setup_child(
    request: &Request,
    language: &Language,
    outside_uid: Uid,
    outside_gid: Gid,
) -> Result<(), String> {
    const SIGKILL: i32 = 9;
    // set up to die if our parent dies
    check!(prctl::set_pdeathsig(Some(SIGKILL)), "error setting parent death signal: {}");

    set_ids(outside_uid, outside_gid)?;
    setup_filesystem(&request, &language)?;
    drop_caps()?;
    set_resource_limits()?;
    Ok(())
}

fn set_ids(outside_uid: Uid, outside_gid: Gid) -> Result<(), String> {
    // we're currently nobody (65534) inside the container which means we can't do anything.
    // so we declare ourselves root inside the container, but this requires a mapping for who we become from the perspective of processes outside the container

    const ROOT_U: Uid = Uid::from_raw(0);
    const ROOT_G: Gid = Gid::from_raw(0);

    let uid_map = format!("{} {} 1\n", ROOT_U, outside_uid);
    check!(std::fs::write("/proc/self/uid_map", uid_map), "error writing uid_map: {}");
    // we need to set this in order to be able to use gid_map
    check!(std::fs::write("/proc/self/setgroups", "deny"), "error denying setgroups: {}");
    let gid_map = format!("{} {} 1\n", ROOT_G, outside_gid);
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

fn setup_filesystem(request: &Request, language: &Language) -> Result<(), String> {
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

fn setup_special_files(language_id: &String) -> Result<(), String> {
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
        ("/usr/local/bin/ATO_bash", "./ATO/bash"),
        ("/usr/local/bin/ATO_yargs", "./ATO/yargs"),
        (&get_runner(&language_id), "./ATO/runner"),
    ] {
        drop(check!(File::create(&dest), "error creating mount point for {}: {}", dest));
        mount!(&src, &dest, , MS_NOSUID | MS_BIND | MS_RDONLY);
    }

    Ok(())
}

fn setup_request_files(request: &Request) -> Result<(), String> {
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

fn drop_caps() -> Result<(), String> {
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

fn set_resource_limits() -> Result<(), String> {
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
