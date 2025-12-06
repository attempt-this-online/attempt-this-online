use crate::constants::*;
use crate::languages::*;
use crate::network::setup_network;
use crate::{Connection, ControlMessage, Error, Request, StreamResponse, check};

use capctl::{caps, prctl};
use clone3::Clone3;
use close_fds::close_open_fds;
use hex::ToHex;
use nix::{
    fcntl::{FcntlArg, OFlag, fcntl},
    mount::{MsFlags, mount},
    poll::{PollFd, PollFlags, poll},
    sys::{
        eventfd::{EfdFlags, eventfd},
        resource::{Resource, UsageWho::RUSAGE_CHILDREN, getrusage, setrlimit},
        stat::Mode,
        time::TimeValLike,
        wait::{self, WaitPidFlag, WaitStatus::*, waitid},
    },
    unistd::{
        Gid, Uid, chdir, close, dup2, execve, mkdir, pipe, pivot_root, read, setresgid, setresuid,
        symlinkat,
    },
};
use rand::Rng;
use serde_bytes::ByteBuf;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
    };
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
            eprintln!("error killing cgroup: {e}");
            return;
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
                    eprintln!(
                        "giving up removing cgroup after {elapsed}ms and {attempt_counter} attempts"
                    );
                }
            } else {
                eprintln!("error removing cgroup: {e}");
            }
            break;
        }
    }
}

fn create_cgroup() -> Result<PathBuf, Error> {
    use std::env::VarError::*;
    let cgroup_path = std::env::var("ATO_CGROUP_PATH").map_err(|e| {
        Error::InternalError(
            match e {
                NotPresent => "error creating cgroup: $ATO_CGROUP_PATH not provided",
                NotUnicode(_) => "error creating cgroup: $ATO_CGROUP_PATH is invalid Unicode",
            }
            .to_string(),
        )
    })?;
    let mut path = PathBuf::from(cgroup_path);
    path.push(random_id());
    check!(std::fs::create_dir(&path), "error creating cgroup dir: {}");
    Ok(path)
}

fn setup_cgroup(path: &PathBuf) -> Result<(), Error> {
    // this sets some resource limits, but the others are set with ordinary POSIX rlimits:
    // see the set_resource_limits function
    // const MEMORY_HIGH: u64 = 512 * MiB;
    // check!(std::fs::write(path.join("memory.high"), MEMORY_HIGH.to_string()), "error writing cgroup memory.high: {}");
    // disable swap
    check!(
        std::fs::write(path.join("memory.swap.max"), "0"),
        "error writing cgroup memory.swap.max: {}"
    );
    Ok(())
}

const RANDOM_ID_SIZE: usize = 16;

fn random_id() -> String {
    rand::thread_rng()
        .r#gen::<[u8; RANDOM_ID_SIZE]>()
        .encode_hex::<String>()
}

pub fn invoke(
    request: &Request,
    language: &Language,
    connection: &mut Connection,
    connection_fd: i32,
) -> Result<(), Error> {
    let cgroup = create_cgroup()?;
    let cgroup_cleanup = Cgroup { cgroup: &cgroup };
    setup_cgroup(&cgroup)?;
    let cgroup_fd = check!(
        nix::fcntl::open(&cgroup, OFlag::O_DIRECTORY | OFlag::O_PATH, Mode::empty()),
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
        .flag_into_cgroup(&cgroup_fd);
    let timer = std::time::Instant::now();
    // this is safe because we haven't used more than one thread so far in this program
    if check!(unsafe { clone3.call() }, "error clone3ing main child: {}") == 0 {
        // in child
        // avoid suicide
        std::mem::forget(cgroup_cleanup);

        // close unused pipe ends to ensure proper synchronisation
        // TODO: do we need to explicitly close these read pipe ends, given the close_range call below?
        check_continue!(close(stdout_r), "error closing stdout read end: {}");
        check_continue!(close(stderr_r), "error closing stderr read end: {}");

        run_child(&request, &language, stdout_w, stderr_w, uid, gid);
        // run_child should never return if successful, so we exit assuming failure
        std::process::exit(2);
    } else {
        // in parent
        // close unused pipe ends
        check!(close(stdout_w), "error closing stdout write end: {}");
        check!(close(stderr_w), "error closing stderr write end: {}");

        run_parent(
            stdout_r,
            stderr_r,
            pidfd,
            cgroup_cleanup,
            timer,
            request.timeout,
            connection,
            connection_fd,
        )
    }
}

fn wait_child(
    pidfd: i32,
    connection: Arc<Mutex<&mut Connection>>,
    connection_fd: i32,
    timeout: i32,
) -> Result<bool, Error> {
    // use a poll to wait for either:
    // - timeout to expire
    // - child to exit
    // - client to request us to kill the child
    let mut poll_args = [
        // pidfd fires a POLLIN event when the process finishes
        PollFd::new(pidfd, PollFlags::POLLIN),
        PollFd::new(connection_fd, PollFlags::POLLIN),
    ];
    let poll_result = check!(
        poll(&mut poll_args, timeout * 1000 /* ms */),
        "error polling: {}"
    );
    let [poll_child, poll_stdin] = poll_args;
    if poll_result == 0 {
        // timed out
        Ok(true)
    } else if poll_child
        .revents()
        .ok_or(Error::InternalError(
            "poll returned unexpected event".into(),
        ))?
        .contains(PollFlags::POLLIN)
    {
        // child finished
        Ok(false)
    } else {
        let stdin_events = poll_stdin.revents().ok_or(Error::InternalError(
            "poll returned unexpected event".into(),
        ))?;
        if stdin_events.contains(PollFlags::POLLIN) {
            // received control message via stdin
            use ControlMessage::*;
            match connection.lock().unwrap().read_message()? {
                Kill => {
                    // continue to drop (i.e. kill), and set timed_out = false
                    Ok(false)
                } // Ok(_) => ...
            }
        } else if stdin_events.contains(PollFlags::POLLHUP) {
            // client disappeared: continue to kill
            Ok(false)
        } else {
            return Err(Error::InternalError(format!(
                "unexpected poll result: {poll_result}, {poll_args:?}"
            )));
        }
    }
}

struct QuitEventFd {
    fd: i32,
}

impl QuitEventFd {
    fn new() -> Result<Self, Error> {
        // eventfd is basically a condition variable, but which we can poll on
        let fd = check!(eventfd(0, EfdFlags::empty()), "error creating eventfd: {}");
        Ok(QuitEventFd { fd })
    }
}

impl Drop for QuitEventFd {
    fn drop(&mut self) {
        check_continue!(
            nix::unistd::write(self.fd, &1u64.to_ne_bytes()),
            "error writing to quit eventfd: {}"
        );
    }
}

fn run_parent(
    stdout_r: i32,
    stderr_r: i32,
    pidfd: i32,
    cgroup_cleanup: Cgroup,
    timer: std::time::Instant,
    timeout: i32,
    connection: &mut Connection,
    connection_fd: i32,
) -> Result<(), Error> {
    let (timed_out, connection, [stdout_truncated, stderr_truncated]) =
        std::thread::scope(move |threads| {
            let connection = Arc::new(Mutex::new(connection));
            // there has to be a better way of doing this
            let connection2 = connection.clone();

            // RAII ensures that the quit eventfd is triggered when it's dropped, so that the
            // output_handler doesn't get confused if the main thread encounters an error
            let quit = QuitEventFd::new()?;

            let output_handler =
                threads.spawn(move || handle_output(stdout_r, stderr_r, quit.fd, connection2));

            // wait for child
            let timed_out = wait_child(pidfd, connection.clone(), connection_fd, timeout)?;

            // kill process
            drop(cgroup_cleanup);

            // tell output_handler to quit
            drop(quit);

            let truncateds = match output_handler.join() {
                // thread panicked, so do likewise
                Err(panic) => std::panic::panic_any(panic),
                // returned normally
                Ok(Ok(truncateds)) => truncateds,
                Ok(Err(e)) => return Err(e),
            };

            let connection = Arc::try_unwrap(connection)
                .unwrap_or_else(|_| panic!("excess references to Arc<Connection>"))
                .into_inner()
                .unwrap();
            Ok((timed_out, connection, truncateds))
        })?;

    // TODO: investigate why this reports ECHILD if the child errors and __WALL is not provided
    // something to do with the fact we use a second thread above which is a different kind of child process?
    // or the rust threading runtime doing strange things to signal handlers?
    // https://github.com/torvalds/linux/blob/a63f2e7cb1107ab124f80407e5eb8579c04eb7a9/kernel/exit.c#L968
    let wait_result = check!(
        waitid(
            wait::Id::PIDFd(pidfd),
            WaitPidFlag::WEXITED | WaitPidFlag::__WALL
        ),
        "error getting waitid result: {}"
    );

    let (status_type, status_value) = match wait_result {
        Exited(_, c) => ("exited", c),
        Signaled(_, c, false) => ("killed", c as i32),
        Signaled(_, c, true) => ("core_dumped", c as i32),
        x => {
            eprintln!("warning: unexpected waitid result: {x:?}");
            ("unknown", -1)
        }
    };

    let stats = check!(
        getrusage(RUSAGE_CHILDREN),
        "error getting resource usage: {}"
    );

    connection.output_message(StreamResponse::Done {
        timed_out,
        status_type,
        status_value,
        stdout_truncated,
        stderr_truncated,
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

fn handle_output(
    stdout_r: i32,
    stderr_r: i32,
    quit: i32,
    connection: Arc<Mutex<&mut Connection>>,
) -> Result<[bool; 2], Error> {
    for (name, pipe) in [("stdout", stdout_r), ("stderr", stderr_r)] {
        check!(
            fcntl(pipe, FcntlArg::F_SETFL(OFlag::O_NONBLOCK)),
            "error setting O_NONBLOCK on {} read end: {}",
            name
        );
    }

    const OUTPUT_BUF_SIZE: usize = 16 * KiB as usize;
    const MAX_SENSIBLE_OUTPUT_SIZE: usize = 128 * KiB as usize;
    let mut totals = [0usize; 2];
    let mut open = [true; 2];
    let mut truncated = [false; 2];

    type StreamId = fn(ByteBuf) -> StreamResponse;

    loop {
        let mut poll_arg = vec![PollFd::new(quit, PollFlags::POLLIN)];
        let mut poll_todo = vec![];

        if open[0] {
            poll_arg.push(PollFd::new(stdout_r, PollFlags::POLLIN));
            poll_todo.push((0, "stdout", stdout_r, StreamResponse::Stdout as StreamId));
        }
        if open[1] {
            poll_arg.push(PollFd::new(stderr_r, PollFlags::POLLIN));
            poll_todo.push((1, "stderr", stderr_r, StreamResponse::Stderr as StreamId));
        }

        check!(
            poll(&mut poll_arg, -1 /* infinite timeout */),
            "error polling for output: {}"
        );
        let poll_quit = poll_arg[0];

        for ((i, name, pipe, stream_id), poll) in poll_todo.into_iter().zip(&poll_arg[1..]) {
            let revents = poll.revents().ok_or(Error::InternalError(
                "poll returned unexpected event".into(),
            ))?;
            if revents.contains(PollFlags::POLLHUP) {
                open[i] = false;
            } else if revents.contains(PollFlags::POLLIN) {
                let mut buf = [0u8; OUTPUT_BUF_SIZE];
                let len = check!(read(pipe, &mut buf), "error reading from {name}: {}");
                totals[i] += len;
                if totals[i] > MAX_SENSIBLE_OUTPUT_SIZE {
                    check!(
                        close(pipe),
                        "error closing {name} (after too much output): {}"
                    );
                    open[i] = false;
                    truncated[i] = true;
                }
                let message = stream_id(ByteBuf::from(&buf[..len]));
                connection.lock().unwrap().output_message(message)?;
            }
        }

        let quit_revents = poll_quit.revents().ok_or(Error::InternalError(
            "poll returned unexpected event".into(),
        ))?;
        if quit_revents.contains(PollFlags::POLLIN) {
            return Ok(truncated);
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
) -> () {
    // to have reliable error reporting, the state of stdout and stderr must be managed carefully:

    // replace current stdout with the pipe we created for it
    if let Err(e) = dup2(stdout_w, STDOUT_FD) {
        // all the possible causes for dup2 to error (see the manual page) should be impossible™
        // so this should never be reached
        eprintln!("error dup2ing stdout: {e}");
        return;
    }

    macro_rules! log_error {
        ($($x:expr),*) => {
            // goes to handle_output and therefore looks like a normal stdout message to the user,
            // so this should be prefixed with "ATO internal error" to make it clearer it's not from the user's program
            println!("ATO internal error: {}", format!($($x,)*));
            // stderr still points to the web server log, so we continue writing there
            eprintln!($($x,)*);
        }
    }

    // TODO: simplify this, because load_env and setup_child only ever return InternalError

    let env = match load_env(&language) {
        Ok(r) => r,
        Err(e) => {
            if let Error::InternalError(e) = e {
                log_error!("{e}");
            }
            return;
        }
    };

    if let Err(e) = setup_child(&request, &language, outside_uid, outside_gid) {
        if let Error::InternalError(e) = e {
            log_error!("{e}");
        }
        return;
    }

    if let Err(e) = dup2(stderr_w, STDERR_FD) {
        log_error!("error dup2ing stderr: {e}");
        return;
    }

    // stderr now points to handle_output too; the web server's log is now inaccessible
    // From here on out, we log errors to stderr only, because logging to both would cause pointless duplication

    // close all remaining FDs except STDIO (0/1/2) - this includes dangling stdxxx_w pipes
    const FIRST_NON_STDIO_FD: i32 = 3;
    // this is safe because it's right before an exec
    unsafe { close_open_fds(FIRST_NON_STDIO_FD, &[]) } // should never error

    let Err(e) = execve(
        cstr!("/ATO/bash"),
        &[cstr!("/ATO/bash"), cstr!("/ATO/runner")],
        &env,
    );
    eprintln!("ATO internal error: error running execve: {e}")
}

fn load_env(language: &Language) -> Result<Vec<CString>, Error> {
    const ENV_BASE_PATH: &str = "/usr/local/lib/ATO/env/";
    let path = String::from(ENV_BASE_PATH) + &language.image.replace("/", "+").replace(":", "+");
    check!(std::fs::read(path), "error reading image env file: {}")
        .split_inclusive(|b| *b == 0) // split after null bytes, and include them in the results
        .map(|s| CString::from_vec_with_nul(s.to_vec()))
        .collect::<Result<Vec<_>, _>>() // collects errors too
        .map_err(|e| Error::InternalError(format!("error building env string: {e}")))
}

fn setup_child(
    request: &Request,
    language: &Language,
    outside_uid: Uid,
    outside_gid: Gid,
) -> Result<(), Error> {
    const SIGKILL: i32 = 9;
    // set up to die if our parent dies
    check!(
        prctl::set_pdeathsig(Some(SIGKILL)),
        "error setting parent death signal: {}"
    );

    set_ids(outside_uid, outside_gid)?;
    setup_network()?;
    setup_filesystem(&request, &language)?;
    drop_caps()?;
    set_resource_limits()?;
    Ok(())
}

fn set_ids(outside_uid: Uid, outside_gid: Gid) -> Result<(), Error> {
    // we're currently nobody (65534) inside the container which means we can't do anything.
    // so we declare ourselves root inside the container, but this requires a mapping for who we become from the perspective of processes outside the container

    const ROOT_U: Uid = Uid::from_raw(0);
    const ROOT_G: Gid = Gid::from_raw(0);

    let uid_map = format!("{ROOT_U} {outside_uid} 1\n");
    check!(
        std::fs::write("/proc/self/uid_map", uid_map),
        "error writing uid_map: {}"
    );
    // we need to set this in order to be able to use gid_map
    check!(
        std::fs::write("/proc/self/setgroups", "deny"),
        "error denying setgroups: {}"
    );
    let gid_map = format!("{ROOT_G} {outside_gid} 1\n");
    check!(
        std::fs::write("/proc/self/gid_map", gid_map),
        "error writing gid_map: {}"
    );

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
    String::from(IMAGE_BASE_PATH) + &language.image.replace("/", "+").replace(":", "+")
}

fn get_default_runner(language_id: &String) -> String {
    const LANGUAGE_BASE_PATH: &str = "/usr/local/share/ATO/runners/";
    String::from(LANGUAGE_BASE_PATH) + language_id
}

fn setup_filesystem(request: &Request, language: &Language) -> Result<(), Error> {
    // find out where the languages' image is stored
    let rootfs = get_rootfs(&language);

    // set the propogation type of all mounts to private - this is because:
    // 1. when we mount /run/ATO, and bind-mount other stuff,
    //    we don't want those to propogate to the parent namespace
    // 2. we don't want any potential mounts in the parent namespace to appear here and mess things up either
    // 3. pivot_root below requires . and its parent to be mounted private anyway
    check!(
        mount::<str, str, str, str>(None, "/", None, MsFlags::MS_PRIVATE | MsFlags::MS_REC, None,),
        "error setting / to MS_PRIVATE: {}"
    );

    // mount a tmpfs to contain the data written to the container's root filesystem
    // (which will be discarded when the container exits)
    mount!("/run/ATO", "tmpfs", MS_NOSUID, "mode=755,size=655350k");
    // overlayfs requires separate "upper" and "work" directories, so create those
    check!(
        mkdir("/run/ATO/upper", Mode::S_IRWXU),
        "error creating overlayfs upper directory: {}"
    );
    check!(
        mkdir("/run/ATO/work", Mode::S_IRWXU),
        "error creating overlayfs work directory: {}"
    );
    // also create a place to mount the new merged writeable rootfs
    check!(
        mkdir("/run/ATO/merged", Mode::S_IRWXU),
        "error creating overlayfs merged directory: {}"
    );

    // mount writeable upper layer on top of rootfs using overlayfs
    // also, the kernel now considers it a mount point, which is required for pivot_root to work
    check!(
        mount::<str, str, str, str>(
            None,
            "/run/ATO/merged",
            Some("overlay"),
            MsFlags::empty(),
            Some(&format!(
                "upperdir=/run/ATO/upper,lowerdir=/usr/local/share/ATO/overlayfs_upper:{rootfs},workdir=/run/ATO/work"
            )),
        ),
        "error mounting new rootfs: {}"
    );

    check!(
        chdir::<str>("/run/ATO/merged"),
        "error changing directory to new rootfs: {}"
    );
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

fn setup_special_files(language_id: &String) -> Result<(), Error> {
    mount!(
        "./tmp",
        "tmpfs",
        MS_NOSUID | MS_NODEV,
        "mode=1755,size=655350k"
    );
    mount!(
        "./ATO",
        "tmpfs",
        MS_NOSUID | MS_NODEV,
        "mode=755,size=655350k"
    );
    check!(
        mkdir(
            "./ATO/context",
            Mode::S_IRWXU | Mode::S_IRGRP | Mode::S_IXGRP | Mode::S_IROTH | Mode::S_IXOTH
        ),
        "error creating /ATO/context: {}"
    );
    mount!("./proc", "proc",);
    mount!(
        "./dev",
        "tmpfs",
        MS_NOSUID | MS_STRICTATIME,
        "mode=755,size=655350k"
    );
    check!(
        mkdir("./dev/pts", Mode::empty()),
        "error creating mount point for /dev/pts: {}"
    );
    mount!(
        "./dev/pts",
        "devpts",
        MS_NOSUID | MS_NOEXEC,
        "newinstance,ptmxmode=0666,mode=0620"
    );
    check!(
        mkdir("./dev/shm", Mode::empty()),
        "error creating mount point for /dev/shm: {}"
    );
    mount!(
        "shm",
        "./dev/shm",
        "tmpfs",
        MS_NOSUID | MS_NODEV | MS_NOEXEC,
        "mode=1777,size=655350k"
    );
    check!(
        mkdir("./dev/mqueue", Mode::empty()),
        "error creating mount point for /dev/mqueue: {}"
    );
    mount!("./dev/mqueue", "mqueue", MS_NOSUID | MS_NODEV | MS_NOEXEC);
    mount!("./sys", "sysfs", MS_NOSUID | MS_NODEV | MS_NOEXEC);
    mount!(
        "./sys/fs/cgroup",
        "cgroup2",
        MS_NOSUID | MS_NODEV | MS_NOEXEC | MS_RELATIME
    );

    // create all the special device files expected on a "real" Linux system

    for dev in ["full", "null", "random", "tty", "urandom", "zero"] {
        let src = "/dev/".to_owned() + dev;
        let dest = "./dev/".to_owned() + dev;
        drop(check!(
            File::create(&dest),
            "error creating mount point for /dev/{}: {}",
            dev
        ));
        mount!(&src, &dest, , MS_NOSUID | MS_NOEXEC | MS_BIND);
    }

    check!(
        symlinkat("/proc/self/fd", None, "dev/fd"),
        "error creating /dev/fd: {}"
    );
    check!(
        symlinkat("/proc/self/fd/0", None, "dev/stdin"),
        "error creating /dev/stdin: {}"
    );
    check!(
        symlinkat("/proc/self/fd/1", None, "dev/stdout"),
        "error creating /dev/stdout: {}"
    );
    check!(
        symlinkat("/proc/self/fd/2", None, "dev/stderr"),
        "error creating /dev/stderr: {}"
    );
    check!(
        symlinkat("/proc/kcore", None, "dev/core"),
        "error creating /dev/core: {}"
    );
    check!(
        symlinkat("pts/ptmx", None, "dev/ptmx"),
        "error creating /dev/ptmx: {}"
    );

    for (src, dest) in [
        ("/usr/local/lib/ATO/bash", "./ATO/bash"),
        ("/usr/local/lib/ATO/yargs", "./ATO/yargs"),
        (&get_default_runner(&language_id), "./ATO/default_runner"),
    ] {
        drop(check!(
            File::create(&dest),
            "error creating mount point for {}: {}",
            dest
        ));
        mount!(&src, &dest, , MS_NOSUID | MS_BIND | MS_RDONLY);
    }

    Ok(())
}

fn setup_request_files(request: &Request) -> Result<(), Error> {
    check!(
        std::fs::write("/ATO/code", &request.code),
        "error writing /ATO/code: {}"
    );
    if let Some(custom_runner) = &request.custom_runner {
        use std::io::Write;
        let mut file = check!(
            std::fs::File::create("/ATO/runner"),
            "error opening /ATO/runner: {}"
        );
        check!(
            file.write_all(&custom_runner),
            "error writing /ATO/runner: {}"
        );
    } else {
        check!(
            std::os::unix::fs::symlink("/ATO/default_runner", "/ATO/runner"),
            "error linking /ATO/runner: {}"
        );
    }
    // TODO: stream input in instead of writing it to a file?
    check!(
        std::fs::write("/ATO/input", &request.input),
        "error writing /ATO/input: {}"
    );
    check!(
        std::fs::write("/ATO/arguments", join_args(&request.arguments)),
        "error writing /ATO/arguments: {}"
    );
    check!(
        std::fs::write("/ATO/options", join_args(&request.options)),
        "error writing /ATO/options: {}"
    );
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

fn drop_caps() -> Result<(), Error> {
    // drop as many privileges as possible
    // for more info on caps, see man capabilities(7)

    // important: make sure this code is written to drop all capabilities, even those we may not
    // know about at compile time because of a kernel version mismatch.
    // https://docs.rs/capctl/latest/capctl/#handling-of-newly-added-capabilities

    check!(
        caps::bounding::clear(),
        "error dropping bounding capabilities: {}"
    );
    check!(
        caps::ambient::clear(),
        "error dropping ambient capabilities: {}"
    );
    // CAP_SETPCAP is required to drop bounding caps, so we have to drop it last
    check!(
        caps::CapState::empty().set_current(),
        "error dropping capabilities: {}"
    );
    // ensure we can't get new privileges from files with set[ug]id or capability xattrs
    check!(
        prctl::set_no_new_privs(),
        "error setting NO_NEW_PRIVS flag: {}"
    );
    Ok(())
}

fn set_resource_limits() -> Result<(), Error> {
    // resource limits work as follows: each one has a "soft" and "hard" limit.
    // for most limits, the process will get an error if it goes beyond the soft limit;
    // the soft limit can be raised by the process but never to higher than the hard limit.
    // Note also that some resource limits are set on the cgroup level - see setup_cgroup

    // CPU time, set as a backup in case the cgroup-based timeout doesn't work
    // if the process reaches the soft limit it will get a SIGXCPU warning signal
    // if it reaches the hard limit it will be forcibly killed
    // TODO: does setting a CPU-time rlimit even work?
    check!(
        setrlimit(Resource::RLIMIT_CPU, 60, 61),
        "error setting CPU resource limit: {}"
    );
    // number of processes/threads, to prevent exhaustion of kernel resources
    check!(
        setrlimit(Resource::RLIMIT_NPROC, 100, 100),
        "error setting NPROC resource limit: {}"
    );
    // written file size, to prevent memory exhaustion by filling up a tmpfs
    check!(
        setrlimit(Resource::RLIMIT_FSIZE, 120 * MiB, 128 * MiB),
        "error setting FSIZE resource limit: {}"
    );

    // these next resource limits are probably not particularly necessary:

    // pending signals
    check!(
        setrlimit(Resource::RLIMIT_SIGPENDING, 100, 100),
        "error setting SIGPENDING resource limit: {}"
    );
    // locked files
    check!(
        setrlimit(Resource::RLIMIT_LOCKS, 100, 100),
        "error setting LOCKS resource limit: {}"
    );
    // bytes in POSIX message queues
    check!(
        setrlimit(Resource::RLIMIT_MSGQUEUE, 100, 100),
        "error setting MSGQUEUE resource limit: {}"
    );
    Ok(())
}
