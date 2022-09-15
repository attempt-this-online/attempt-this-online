#![feature(io_error_more, duration_constants, never_type)]

mod codes;
mod languages;
use crate::{codes::*, languages::*};
use clone3::Clone3;
use close_fds::close_open_fds;
use hex::ToHex;
use nix::{
    dir::Dir,
    fcntl::{OFlag, open},
    mount::{mount, MsFlags},
    poll::{PollFd, PollFlags, poll},
    sys::stat::Mode,
    unistd::{close, dup2, execve, fchdir, pipe, pivot_root},
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
    ($x:expr, $f:literal $(, $($a:literal),+)? $(,)?) => {
        $x.map_err(|e| format!($f, $($($a,)*)? e))?
    };
    ($x:expr $(,)?) => {
        $x.map_err(|e| e.to_string())?
    }
}

/// log `Err`s to stderr but don't stop execution
macro_rules! check_continue {
    ($x:expr, $f:literal $(, $($a:literal),+)? $(,)?) => {
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

const CGROUP_REMOVE_MAX_ATTEMPT_TIME: u128 = 100; // ms

impl Drop for Cgroup<'_> {
    fn drop(&mut self) {
        if let Err(e) = std::fs::write(self.cgroup.join("cgroup.kill"), "1") {
            eprintln!("error killing cgroup: {}", e);
            return
        }

        let timer = std::time::Instant::now();
        let mut attempt_counter = 0;
        while let Err(e) = std::fs::remove_dir(&self.cgroup) {
            if e.kind() == std::io::ErrorKind::ResourceBusy {
                // cgroup not succesfully killed yet: retry?
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

fn invoke(request: &Request, language: &Language) -> Result<Response, String> {
    let cgroup = create_cgroup()?;
    let cgroup_cleanup = Cgroup{ cgroup: &cgroup };
    setup_cgroup(&cgroup)?;
    let cgroup_fd = check!(
        Dir::open(&cgroup, OFlag::O_DIRECTORY | OFlag::O_PATH, Mode::empty()),
        "error opening cgroup dir: {}",
    );

    // Currently the child process will just block forever if it tries to write more than
    // $MAX_PIPE_SIZE bytes to stdout or stderr; we rely purely on this mechanism to prevent
    // excessive output. If the process gets to this point (where it's blocking forever), it will
    // eventually time out and die. This means the read_to_end calls below won't try to read an
    // infinite or very large stream.
    let (stdout_r, stdout_w) = check!(pipe(), "error creating stdout pipe: {}");
    let (stderr_r, stderr_w) = check!(pipe(), "error creating stderr pipe: {}");

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
    if check!(unsafe { clone3.call() }, "error clone3ing main child: {}") == 0 { // in child
        // avoid suicide
        std::mem::forget(cgroup_cleanup);

        // close unused pipe ends to ensure proper synchronisation
        check_continue!(close(stdout_r), "error closing stdout read end: {}");
        check_continue!(close(stderr_r), "error closing stderr read end: {}");

        run_child(&request, &language, stdout_w, stderr_w);
        // run_child should never return if successful, so we exit assuming failure
        std::process::exit(2);
    } else { // in parent
        // close unused pipe ends
        check!(close(stdout_w), "error closing stdout write end: {}");
        check!(close(stderr_w), "error closing stderr write end: {}");

        // wait for the process to finish, but with the given timeout:
        // if the timeout expires before the process finishes,
        // it will be killed when the cgroup struct is dropped
        let mut poll_arg = PollFd::new(pidfd, PollFlags::POLLIN);
        let mut poll_args = std::slice::from_mut(&mut poll_arg);
        // poll wants milliseconds; request.timeout is seconds
        let poll_result = check!(poll(&mut poll_args, request.timeout * 1000));
        let timed_out = poll_result == 0;

        // kill process
        drop(cgroup_cleanup);

        let mut stdout = Vec::new();
        check!(unsafe { File::from_raw_fd(stdout_r) }.read_to_end(&mut stdout), "error reading stdout: {}");
        let mut stderr = Vec::new();
        check!(unsafe { File::from_raw_fd(stderr_r) }.read_to_end(&mut stderr), "error reading stderr: {}");

        Ok(Response {
            stdout: ByteBuf::from(stdout),
            stderr: ByteBuf::from(stderr),
            timed_out,
        })
    }
}

fn run_child(request: &Request, language: &Language, stdout_w: i32, stderr_w: i32) -> (/* never returns on success */) {
    // to have reliable error reporting, the state of stdout and stderr must be managed carefully:

    // here, stdout points directly to the client websocket, so we mustn't print junk to stdout.
    // stderr points to the systemd log as normal, so we log errors to stderr only

    // replace current stdout with the pipe we created for it
    if let Err(e) = dup2(stdout_w, STDOUT_FD) {
        eprintln!("error dup2ing stdout: {}", e);
        return
    }
    // stdout now points to a pipe that the parent handles, so we messages to it will reach the
    // user safely and we don't need to worry about junk.
    // so we should log errors to both stderr and stdout

    if let Err(e) = setup_child(&request, &language) {
        eoprintln!("internal error: {}", e);
        return
    }

    if let Err(e) = dup2(stderr_w, STDERR_FD) {
        eoprintln!("error dup2ing stderr: {}", e);
        return
    }
    // stderr now points to the user; the systemd log is now unreachable
    // we log errors to stderr only because logging to both would cause pointless duplication

    // close all remaining FDs, including dangling stdxxx_w pipes
    // this is safe because it's right before an exec
    unsafe { close_open_fds(3, &[]) } // should never error

    if let Err(e) = execve(cstr!("./inside"), &[cstr!("inside"), cstr!("arg1")], &[cstr!("TODO=TODO")]) {
        eprintln!("internal error: error running execve: {}", e)
    } else {
        eprintln!("internal error: execve should never return if successful")
    }
}

fn setup_child(request: &Request, language: &Language) -> Result<(), String> {
    drop_caps()?;
    setup_filesystem(&language)?;

    let _ = request;
    Ok(())
}

fn drop_caps() -> Result<(), String> {
    // TODO
    Ok(())
}

fn setup_filesystem(language: &Language) -> Result<(), String> {
    let old_cwd = check!(open(".", OFlag::O_DIRECTORY | OFlag::O_PATH, Mode::empty()), "error opening old working directory: {}");
    
    let rootfs_path = get_rootfs(&language);
    let rootfs_string = rootfs_path.to_str().expect("rootfs path is invalid UTF-8");

    // set the propogation type of all mounts to private - this is because:
    // 1. pivot_root below requires . and its parent to be mounted private
    // 2. when we bind-mount stuff we don't want that to propogate to the parent namespace
    check!(mount::<str, str, str, str>(
        None,
        "/",
        None,
        MsFlags::MS_PRIVATE | MsFlags::MS_REC,
        None,
    ), "error setting / to MS_PRIVATE: {}");

    // bind-mount . onto itself; this has no effect, other than making the kernel consider . a
    // mount point, which is required for pivot_root to work
    check!(mount::<str, str, str, str>(
        Some(&rootfs_string),
        &rootfs_string,
        None,
        MsFlags::MS_BIND | MsFlags::MS_REC,
        None,
    ), "error bind-mounting new rootfs onto itself: {}");

    // mark rootfs as private as well (unncessary?)
    check!(mount::<str, str, str, str>(
        None,
        &rootfs_string,
        None,
        MsFlags::MS_PRIVATE | MsFlags::MS_REC,
        None,
    ), "error setting new rootfs to MS_PRIVATE: {}");

    // no idea why we do this (copied from crun strace)
    check!(mount::<str, str, str, str>(
        None,
        &rootfs_string,
        None,
        MsFlags::MS_REMOUNT | MsFlags::MS_BIND,
        None,
    ), "error re-bind-mounting new rootfs: {}");

    // not sure why we need a file descriptor to this?
    let rootfs = check!(
        open(&rootfs_path,
             OFlag::O_DIRECTORY | OFlag::O_PATH | OFlag::O_CLOEXEC,
             Mode::empty()
        ), "error opening new rootfs: {}");

    check!(mount::<str, str, str, str>(
        None,
        &format!("/proc/self/fd/{}", rootfs),
        None,
        MsFlags::MS_RDONLY | MsFlags::MS_REMOUNT | MsFlags::MS_BIND,
        None,
    ), "error mounting (?): {}");

    check!(fchdir(rootfs), "error changing directory to new rootfs: {}");

    // swap (or "pivot") the meanings of / and .
    // so now, / points to the new container rootfs, and . points to the old system root
    // (note that this means . is not actually anywhere in the directory tree!)
    check!(pivot_root(".", "."), "error pivoting root: {}");

    check!(mount::<str, str, str, str>(
        Some("tmpfs"),
        "/ATO",
        Some("tmpfs"),
        MsFlags::MS_NOSUID | MsFlags::MS_NODEV,
        None,
    ));

    // TODO: bind mounts

    // cwd after pivot_root is not techincally well-defined
    // so TODO really we should chdir to old root?
    check!(fchdir(old_cwd), "error changing directory to old cwd: {}");

    Ok(())
}

const IMAGE_BASE_PATH: &str = "/usr/local/lib/ATO/rootfs/";

fn get_rootfs(language: &Language) -> PathBuf {
    let mut path = PathBuf::from(IMAGE_BASE_PATH);
    path.push(language.image.replace("/", "+"));
    path
}

fn create_cgroup() -> Result<PathBuf, String> {
    let mut path = PathBuf::from(CGROUP_PATH);
    path.push(random_id());
    check!(std::fs::create_dir(&path), "error creating cgroup dir: {}");
    Ok(path)
}

// TODO: dynamically work out the cgroup path
// const CGROUP_PATH: &str = "/sys/fs/cgroup/system.slice/ATO.service";
const CGROUP_PATH: &str = "/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service/ATOtest";
#[allow(non_upper_case_globals)]
const KiB: u64 = 1024;
#[allow(non_upper_case_globals)]
const MiB: u64 = KiB * KiB;
const MEMORY_HIGH: u64 = 200 * MiB;
const MEMORY_MAX: u64 = 256 * MiB;

const MAX_STDOUT_SIZE: u64 = 128 * KiB;
const MAX_STDERR_SIZE: u64 = 32 * KiB;

fn setup_cgroup(path: &PathBuf) -> Result<(), String> {
    check!(std::fs::write(path.join("memory.high"), MEMORY_HIGH.to_string()), "error writing cgroup memory.high: {}");
    check!(std::fs::write(path.join("memory.max"), MEMORY_MAX.to_string()), "error writing cgroup memory.max: {}");
    check!(std::fs::write(path.join("memory.swap.max"), "0"), "error writing cgroup memory swap.max: {}");
    Ok(())
}

const RANDOM_ID_SIZE: usize = 16;

fn random_id() -> String {
    rand::thread_rng().gen::<[u8; RANDOM_ID_SIZE]>().encode_hex::<String>()
}
