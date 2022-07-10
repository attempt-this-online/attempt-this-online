mod codes;
mod languages;
use crate::{codes::*, languages::*};
use clone3::Clone3;
use defer_lite::defer;
use hex::ToHex;
use nix::{
    fcntl::{OFlag, splice, SpliceFFlags},
    dir::Dir,
    poll::*,
    sys::memfd::*,
    // sys::signal::*,
    // sys::signalfd::*,
    sys::stat::Mode,
    unistd::*,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use std::io::{Read, Write};
use std::os::unix::io::FromRawFd;
use std::path::*;

macro_rules! log_error {
    ($($x:expr),*) => {
        // so user sees the error
        println!($($x,)*);
        // so ATO system log sees the error
        eprintln!($($x,)*);
    }
}

macro_rules! check {
    ($x:expr, $f:literal $(, $($a:literal),+)? $(,)?) => {
        $x.map_err(|e| format!($f, e, $($($a,)*)?))?
    };
    ($x:expr $(,)?) => {
        $x.map_err(|e| e.to_string())?
    }
}

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
            log_error!("decode error: {}", e);
            return std::process::ExitCode::from(POLICY_VIOLATION);
        }
    };
    let language = match validate(&request) {
        Ok(l) => l,
        Err(e) => {
            log_error!("invalid request: {}", e);
            return std::process::ExitCode::from(POLICY_VIOLATION);
        }
    };
    let result = match invoke(&request, language) {
        Ok(r) => r,
        Err(e) => {
            log_error!("internal error: {}", e);
            return std::process::ExitCode::from(INTERNAL_ERROR);
        }
    };
    let encoded_output = match rmp_serde::to_vec_named(&result) {
        Ok(r) => r,
        Err(e) => {
            log_error!("error encoding output: {}", e);
            return std::process::ExitCode::from(INTERNAL_ERROR);
        }
    };
    match std::io::stdout().write_all(&encoded_output[..]) {
        Ok(()) => std::process::ExitCode::from(NORMAL),
        Err(e) => {
            log_error!("error writing output: {}", e);
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

const STDIN_FD: std::os::unix::io::RawFd = 0;
const STDOUT_FD: std::os::unix::io::RawFd = 1;
const STDERR_FD: std::os::unix::io::RawFd = 2;

fn invoke(request: &Request, language: &Language) -> Result<Response, String> {
    let _ = (request, language);
    let cgroup = create_cgroup()?;
    defer! { // using defer is easier than making a special type to represent the cgroup
        cleanup_cgroup(&cgroup);
    }
    setup_cgroup(&cgroup)?;
    let cgroup_fd = check!(
        Dir::open(&cgroup, OFlag::O_DIRECTORY | OFlag::O_PATH, Mode::empty()),
        "error opening cgroup dir: {}",
    );

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
    if check!(unsafe { clone3.call() }, "error clone3ing main child: {}") == 0 {
        // in child
        check!(close(stdout_r), "error closing stdout read end: {}");
        check!(dup2(stdout_w, STDOUT_FD), "error dup2ing stdout: {}");
        check!(close(stdout_w), "error closing stdout write end: {}");
        // do stderr last so error reporting works for as long as possible
        check!(close(stderr_r), "error closing stderr read end: {}");
        check!(dup2(stderr_w, STDERR_FD), "error dup2ing stderr: {}");
        check!(close(stderr_w), "error closing stderr write end: {}");
        drop_caps()?;
        // TODO: pivot_root
        // TODO: various bind mounts
        // TODO: write stdin and code to /ATO files
        let result = execve(cstr!("TODO"), &[cstr!("TODO")], &[cstr!("TODO")]);
        let e = result.err().expect("execve should never return if successful");
        std::process::exit(e as i32);
    }
    // in parent

    macro_rules! copy_stdxx {
        ($name:literal, $max:ident, $pipe:ident) => {{
            let file = check!(memfd_create(cstr!($name), MemFdCreateFlag::empty()), "error creating {} memfd: {}", $name);
            let mut clone3 = Clone3::default();
            // ensure it's killed by putting it into the cgroup which we gonna kill soon
            clone3.flag_into_cgroup(&cgroup_fd);
            if check!(unsafe { clone3.call() }, "error clone3ing {} reader: {}", $name) == 0 {
                let mut remaining = $max as isize;
                while remaining > 0 {
                    match splice($pipe, None, file, None, remaining as usize, SpliceFFlags::empty()) {
                        Err(e) => {
                            eprintln!("error splicing {}: {}", $name, e);
                            std::process::exit(e as i32);
                        }
                        Ok(0) => {
                            // EOF
                            break;
                        }
                        Ok(spliced) => {
                            remaining -= spliced as isize;
                        }
                    }
                }
                std::process::exit(0);
            }
            unsafe { std::fs::File::from_raw_fd(file) }
        }}
    }

    let mut stdout_f = copy_stdxx!("stdout", MAX_STDOUT_SIZE, stdout_r);
    let mut stderr_f = copy_stdxx!("stderr", MAX_STDERR_SIZE, stderr_r);

    // wait for the process to finish, but with the given timeout:
    // if the timeout expires before the process finishes,
    // it will be killed by the deferred cleanup_cgroup call above
    let mut poll_arg = PollFd::new(pidfd, PollFlags::POLLIN);
    let mut poll_args = std::slice::from_mut(&mut poll_arg);
    // poll wants milliseconds; request.timeout is seconds
    let poll_result = check!(poll(&mut poll_args, request.timeout * 1000));
    let timed_out = poll_result == 0;

    let mut stdout = Vec::new();
    check!(stdout_f.read_to_end(&mut stdout), "error reading stdout: {}");
    let mut stderr = Vec::new();
    check!(stderr_f.read_to_end(&mut stderr), "error reading stderr: {}");

    Ok(Response {
        stdout: ByteBuf::from(stdout),
        stderr: ByteBuf::from(b"goodbye".to_vec()),
        timed_out,
    })
}

fn drop_caps() -> Result<(), String> {
    // TODO
    Ok(())
}

fn cleanup_cgroup(cgroup: &Path) {
    if let Err(e) = std::fs::write(cgroup, "1") {
        eprintln!("error killing contents of cgroup: {}", e)
        // but continue anyway if possible
    }
    if let Err(e) = std::fs::remove_dir(&cgroup) {
        eprintln!("error removing cgroup dir: {}", e)
    }
}

fn create_cgroup() -> Result<PathBuf, String> {
    let mut path = PathBuf::from(CGROUP_PATH);
    path.push(random_id());
    check!(std::fs::create_dir(&path), "error creating cgroup dir: {}");
    Ok(path)
}

// TODO: dynamically work out the cgroup path
const CGROUP_PATH: &str = "/sys/fs/cgroup/system.slice/ATO.service";
#[allow(non_upper_case_globals)]
const KiB: u64 = 1024;
#[allow(non_upper_case_globals)]
const MiB: u64 = KiB * KiB;
const MEMORY_HIGH: u64 = 200 * MiB;
const MEMORY_MAX: u64 = 256 * MiB;

const MAX_STDOUT_SIZE: u64 = 128 * KiB;
const MAX_STDERR_SIZE: u64 = 32 * KiB;

fn setup_cgroup(path: &PathBuf) -> Result<(), String> {
    check!(std::fs::write(path.join("memory.high"), MEMORY_HIGH.to_string()));
    check!(std::fs::write(path.join("memory.max"), MEMORY_MAX.to_string()));
    check!(std::fs::write(path.join("memory.swap.max"), "0"));
    Ok(())
}

const RANDOM_ID_SIZE: usize = 16;

fn random_id() -> String {
    rand::thread_rng().gen::<[u8; RANDOM_ID_SIZE]>().encode_hex::<String>()
}
