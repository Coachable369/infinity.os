#ifndef INFINITY_COMPILER_DIRENT_H
#define INFINITY_COMPILER_DIRENT_H
#ifdef __cplusplus
extern "C" {
#endif
typedef struct InfinityDirectory DIR;
struct dirent { char d_name[96]; };
DIR *opendir(const char *);
struct dirent *readdir(DIR *);
int closedir(DIR *);
#ifdef __cplusplus
}
#endif
#endif
