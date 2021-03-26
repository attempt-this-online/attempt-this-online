#include <errno.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdbool.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>

#define ARGS_BUFFER_SIZE 10
#define FILE_BUFFER_SIZE 4096

#define APPEND(ptr) do { \
    if (args_length >= args_buffer_size) { /* needs realloc */ \
        args_buffer_size += ARGS_BUFFER_SIZE * sizeof ptr; \
        args_buffer = realloc(args_buffer, args_buffer_size); \
        if (args_buffer == NULL) { \
            perror("malloc"); \
            return 1; \
        }; \
    }; \
    args_buffer[args_length] = (ptr); \
    args_length++; \
} while (0)

int main(int argc, char * argv []) {
    if (argc < 4) {
        fprintf(stderr, "%s\n", "yargs: too few arguments");
        return 1;
    };
    char * replace_string = argv[1];
    char * file_name = argv[2];
    char * program = argv[3];
    int fd = openat(AT_FDCWD, file_name, O_CLOEXEC);
    if (fd < 0) {
        perror("yargs: openat");
        return 1;
    };
    char * file_buf = NULL;
    size_t file_size = 0;
    while (true) {
        file_buf = realloc(file_buf, file_size + FILE_BUFFER_SIZE);
        if (file_buf == NULL) {
            perror("yargs: malloc");
            return 1;
        };
        ssize_t n = read(fd, file_buf, FILE_BUFFER_SIZE);
        if (n == 0) {
            // EOF
            break;
        } else if (n < 0) {
            perror("yargs: read");
            return 1;
        } else {
            file_size += n;
        };
    };
    char * * args_buffer = NULL;
    size_t args_buffer_size = 0;
    size_t args_length = 0;
    bool replaced = false;
    APPEND(program);
    for (size_t i = 4; i < argc; i++) {
        if (!replaced && strcmp(argv[i], replace_string) == 0) {
            replaced = true;
            size_t arg = 0;
            size_t j;
            for (j = 0; j < file_size; j++) {
                if (file_buf[j] == 0) {
                    // null-termination = end of string
                    APPEND(file_buf + arg);
                    arg = j + 1;
                };
            };
            if (file_size != 0 && file_buf[j] != 0) {
                fprintf(stderr, "%s\n", "yargs: string was not null-terminated!");
            };    
        } else {
            APPEND(argv[i]);
        };
    };
    if (!replaced) {
        fprintf(stderr, "%s\n", "yargs: warning: no replacement string was found");
    };
    // execv requires a null pointer to terminate the argument array
    APPEND(NULL);
    execvp(program, args_buffer);
    // shouldn't reach this point if execvp succeeds
    perror("yargs: execvp");
    return 1;
};
