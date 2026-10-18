#ifndef INFINITY_IMAGE_RUNTIME_COMPAT_H
#define INFINITY_IMAGE_RUNTIME_COMPAT_H
#include <dirent.h>
/* libc++ declares directory operations even though image inference only opens
 * explicit model files. No directory-descriptor authority is granted here. */
#ifdef __cplusplus
extern "C" {
#endif
DIR *fdopendir(int);
#ifdef __cplusplus
}
#endif
#endif
