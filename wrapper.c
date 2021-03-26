// Thin wrapper around runner script, largely to manage writing the status code of the process to the file descriptor
// given as the first command-line argument. This will be run inside the container so it should be
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <signal.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

// do-while is a hack to allow a trailing semicolon
#define PERROR(result, string) do { if ((result) < 0) { perror(string); return errno; }; } while (0)

int main(int argc, char * argv []) {
    if (argc != 2) {
        // file descriptor must be given as argument
        return 1;
    };
    int fd = 0;
    if (argv[1][0] < '1') {
        // invalid integer
        return 1;
    };
    for (int i = 0; argv[1][i]; i++) {
        if (argv[1][i] < '0' || argv[1][i] > '9') {
            // invalid integer
            return 1;
        }
        fd *= 10;
        fd += argv[1][i] - '0';
    };
    errno = 0;
    fcntl(fd, F_GETFD);
    if (errno) {
        perror("wrapper");
        return errno;
    };
    pid_t pid = fork();
    PERROR(pid, "fork");
    if (pid == 0) {  // in the child
        close(fd);  // Disallow the untrusted code to access the file descriptor where we will write the timing info
        PERROR(execl("/ATO/runner", "/ATO/runner", (char *) NULL), "execl");
    } else {
        siginfo_t info;
        char * status_type;
        PERROR(waitid(P_PID, pid, &info, WEXITED | WSTOPPED | WCONTINUED), "waitid");
        switch (info.si_code) {
            case CLD_EXITED:
                status_type = "exited";
                break;
            case CLD_KILLED:
                status_type = "killed";
                break;
            case CLD_DUMPED:
                status_type = "dumped";
                break;
            case CLD_STOPPED:
                status_type = "stopped";
                break;
            case CLD_TRAPPED:
                status_type = "trapped";
                break;
            case CLD_CONTINUED:
                status_type = "continued";
                break;
            default:
                status_type = "unknown";
                break;
        };
        // I don't quite understand the way waitid works - it seems like it might return even if the process is still
        // running, so kill it just in case I guess?
        kill(pid, SIGKILL);
        // write JSON info
        PERROR(dprintf(fd, "\"status_type\":\"%s\",\"status_value\":%d,", status_type, info.si_status), "dprintf");
        return 0;
    };
};
