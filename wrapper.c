/* timeout -- run a command with bounded time
  
   Modified by Patrick Reader from the original from GNU Coreutils, which is:
   Copyright (C) 2008-2021 Free Software Foundation, Inc.

   This program is free software: you can redistribute it and/or modify
   it under the terms of the GNU General Public License as published by
   the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   This program is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU General Public License for more details.

   You should have received a copy of the GNU General Public License
   along with this program.  If not, see <https://www.gnu.org/licenses/>.  */

/* timeout - Start a command, and kill it if the specified timeout expires

   We try to behave like a shell starting a single (foreground) job,
   and will kill the job if we receive the alarm signal we setup.
   The exit status of the job is returned, or one of these errors:
     EXIT_TIMEDOUT      124      job timed out
     EXIT_CANCELED      125      internal error
     EXIT_CANNOT_INVOKE 126      error executing job
     EXIT_ENOENT        127      couldn't find job to exec

   Caveats:
     If user specifies the KILL (9) signal is to be sent on timeout,
     the monitor is killed and so exits with 128+9 rather than 124.

     If you start a command in the background, which reads from the tty
     and so is immediately sent SIGTTIN to stop, then the timeout
     process will ignore this so it can timeout the command as expected.
     This can be seen with 'timeout 10 dd&' for example.
     However if one brings this group to the foreground with the 'fg'
     command before the timer expires, the command will remain
     in the stop state as the shell doesn't send a SIGCONT
     because the timeout process (group leader) is already running.
     To get the command running again one can Ctrl-Z, and do fg again.
     Note one can Ctrl-C the whole job when in this state.
     I think this could be fixed but I'm not sure the extra
     complication is justified for this scenario.

   Written by Pádraig Brady.  */

#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <signal.h>
#include <stdbool.h>
#include <stdio.h>
#include <sys/prctl.h>
#include <sys/resource.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

/* NonStop circa 2011 lacks both SA_RESTART and siginterrupt.  */
#ifndef SA_RESTART
#define SA_RESTART 0
#endif

#define PROGRAM_NAME "timeout"

// #define AUTHORS proper_name("Padraig Brady")

#define TIMEOUT_SECS 60

static int timed_out;
static int term_signal = SIGKILL; /* same default as kill command.  */
static pid_t monitored_pid;
static bool foreground; /* whether to use another program group.  */
static bool preserve_status; /* whether to use a timeout status or not.  */

/* Start the timeout after which we'll receive a SIGALRM. */
static void
settimeout(bool warn)
{

    struct timespec ts = { TIMEOUT_SECS, 0 };
    struct itimerspec its = { { 0, 0 }, ts };
    timer_t timerid;
    if (timer_create(CLOCK_REALTIME, NULL, &timerid) == 0) {
        if (timer_settime(timerid, 0, &its, NULL) == 0)
            return;
        else {
            if (warn)
                perror("warning: timer_settime");
            timer_delete(timerid);
        }
    } else if (warn && errno != ENOSYS)
        perror("warning: timer_create");

    /* fallback to single second resolution provided by alarm().  */
    alarm(TIMEOUT_SECS);
}

/* send SIG avoiding the current process.  */

static int
send_sig(pid_t where, int sig)
{
    /* If sending to the group, then ignore the signal,
     so we don't go into a signal loop.  Note that this will ignore any of the
     signals registered in install_cleanup(), that are sent after we
     propagate the first one, which hopefully won't be an issue.  Note this
     process can be implicitly multithreaded due to some timer_settime()
     implementations, therefore a signal sent to the group, can be sent
     multiple times to this process.  */
    if (where == 0)
        signal(sig, SIG_IGN);
    return kill(where, sig);
}

/* Signal handler which is required for sigsuspend() to be interrupted
   whenever SIGCHLD is received.  */
static void
chld(int sig)
{
}

static void
cleanup(int sig)
{
    if (sig == SIGALRM) {
        timed_out = 1;
        sig = term_signal;
    }
    if (monitored_pid) {
        /* Send the signal directly to the monitored child,
         in case it has itself become group leader,
         or is not running in a separate group.  */
        send_sig(monitored_pid, sig);
    } else /* we're the child or the child is not exec'd yet.  */
        _exit(128 + sig);
}

static void
unblock_signal(int sig)
{
    sigset_t unblock_set;
    sigemptyset(&unblock_set);
    sigaddset(&unblock_set, sig);
    if (sigprocmask(SIG_UNBLOCK, &unblock_set, NULL) != 0)
        perror("warning: sigprocmask");
}

static void
install_sigchld(void)
{
    struct sigaction sa;
    sigemptyset(&sa.sa_mask); /* Allow concurrent calls to handler */
    sa.sa_handler = chld;
    sa.sa_flags = SA_RESTART; /* Restart syscalls if possible, as that's
                                 more likely to work cleanly.  */

    sigaction(SIGCHLD, &sa, NULL);

    /* We inherit the signal mask from our parent process,
     so ensure SIGCHLD is not blocked. */
    unblock_signal(SIGCHLD);
}

static void
install_cleanup(int sigterm)
{
    struct sigaction sa;
    sigemptyset(&sa.sa_mask); /* Allow concurrent calls to handler */
    sa.sa_handler = cleanup;
    sa.sa_flags = SA_RESTART; /* Restart syscalls if possible, as that's
                                 more likely to work cleanly.  */

    sigaction(SIGALRM, &sa, NULL); /* our timeout.  */
    sigaction(SIGINT, &sa, NULL); /* Ctrl-C at terminal for example.  */
    sigaction(SIGQUIT, &sa, NULL); /* Ctrl-\ at terminal for example.  */
    sigaction(SIGHUP, &sa, NULL); /* terminal closed for example.  */
    sigaction(SIGTERM, &sa, NULL); /* if we're killed, stop monitored proc.  */
    sigaction(sigterm, &sa, NULL); /* user specified termination signal.  */
}

/* Block all signals which were registered with cleanup() as the signal
   handler, so we never kill processes after waitpid() returns.
   Also block SIGCHLD to ensure it doesn't fire between
   waitpid() polling and sigsuspend() waiting for a signal.
   Return original mask in OLD_SET.  */
static void
block_cleanup_and_chld(int sigterm, sigset_t* old_set)
{
    sigset_t block_set;
    sigemptyset(&block_set);

    sigaddset(&block_set, SIGALRM);
    sigaddset(&block_set, SIGINT);
    sigaddset(&block_set, SIGQUIT);
    sigaddset(&block_set, SIGHUP);
    sigaddset(&block_set, SIGTERM);
    sigaddset(&block_set, sigterm);

    sigaddset(&block_set, SIGCHLD);

    if (sigprocmask(SIG_BLOCK, &block_set, old_set) != 0)
        perror("warning: sigprocmask");
}

int main(int argc, char** argv)
{
    if (argc != 2) {
        // file descriptor must be given as argument
        return 1;
    }
    int fd = 0;
    if (argv[1][0] < '1') {
        // invalid integer
        return 1;
    }
    for (int i = 0; argv[1][i]; i++) {
        if (argv[1][i] < '0' || argv[1][i] > '9') {
            // invalid integer
            return 1;
        }
        fd *= 10;
        fd += argv[1][i] - '0';
    }
    errno = 0;
    fcntl(fd, F_GETFD);
    if (errno) {
        perror("wrapper");
        return errno;
    }

    preserve_status = true;

    /* Ensure we're in our own group so all subprocesses can be killed.
     Note we don't just put the child in a separate group as
     then we would need to worry about foreground and background groups
     and propagating signals between them.  */
    if (!foreground)
        setpgid(0, 0);

    /* Setup handlers before fork() so that we
     handle any signals caused by child, without races.  */
    install_cleanup(term_signal);
    signal(SIGTTIN, SIG_IGN); /* Don't stop if background child needs tty.  */
    signal(SIGTTOU, SIG_IGN); /* Don't stop if background child needs tty.  */
    install_sigchld(); /* Interrupt sigsuspend() when child exits.   */

    monitored_pid = fork();
    if (monitored_pid == -1) {
        perror("fork system call failed");
        return 2;
    } else if (monitored_pid == 0) { /* child */
        /* exec doesn't reset SIG_IGN -> SIG_DFL.  */
        signal(SIGTTIN, SIG_DFL);
        signal(SIGTTOU, SIG_DFL);

        close(fd);
        execlp("/ATO/runner", "/ATO/runner", (char*)NULL);
        perror("execlp");
        return 1;
    } else {
        pid_t wait_result;
        int status;
        char* status_type = "unknown";

        /* We configure timers so that SIGALRM is sent on expiry.
         Therefore ensure we don't inherit a mask blocking SIGALRM.  */
        unblock_signal(SIGALRM);

        settimeout(true);

        /* Ensure we don't cleanup() after waitpid() reaps the child,
         to avoid sending signals to a possibly different process.  */
        sigset_t cleanup_set;
        block_cleanup_and_chld(term_signal, &cleanup_set);

        while ((wait_result = waitpid(monitored_pid, &status, WNOHANG)) == 0)
            sigsuspend(&cleanup_set); /* Wait with cleanup signals unblocked.  */

        if (wait_result < 0) {
            /* shouldn't happen.  */
            perror("error waiting for command");
            status = -1;
        } else {
            if (WIFEXITED(status)) {
                status_type = "exited";
                status = WEXITSTATUS(status);
            } else if (WIFSIGNALED(status)) {
                status_type = "killed";
                if (WCOREDUMP(status))
                    status_type = "core_dump";
                status = WTERMSIG(status);
            } else {
                /* shouldn't happen.  */
                status = -1;
            }
        }

        int result = dprintf(fd, "\"timed_out\":%s,\"status_type\":\"%s\",\"status_value\":%d,", (timed_out ? "true" : "false"), status_type, status);
        if (result < 0) {
            perror("dprintf");
        }

        return 0;
    }
}
