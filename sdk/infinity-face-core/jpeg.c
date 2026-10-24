/* Native worker-only JPEG adapter. Pinned stb_image is fetched by the build
 * recipe, compiled as C with every non-JPEG format and host I/O disabled. */
#include <stddef.h>
#include <stdint.h>
#include <string.h>
static unsigned char *arena;
static size_t arena_length, arena_used;

// ------------------------=
// FUNC: face_alloc
// DESC: Allocates aligned decoder storage from a fixed private per-frame arena without OS or host allocation.
// ------------------=
static void *face_alloc(size_t length) {
    if (!length || length>arena_length || arena_used>arena_length-length || arena_length-arena_used-length<16) return NULL;
    size_t at=(arena_used+15)&~(size_t)15;
    if (at>arena_length || length>arena_length-at) return NULL;
    arena_used=at+length; return arena+at;
}
// ------------------------=
// FUNC: face_realloc
// DESC: Copies into another bounded arena range; all allocations are discarded and wiped after one decode.
// ------------------=
static void *face_realloc(void *previous, size_t old_length, size_t length) {
    void *result=face_alloc(length);
    if (result && previous) memcpy(result,previous,old_length<length?old_length:length);
    return result;
}
#define STBI_ONLY_JPEG
#define STBI_NO_STDIO
#define STBI_NO_HDR
#define STBI_NO_LINEAR
#define STBI_NO_SIMD
#define STBI_NO_THREAD_LOCALS
#define STBI_MAX_DIMENSIONS 640
#define STBI_ASSERT(x) ((void)0)
#define STBI_MALLOC(n) face_alloc(n)
#define STBI_REALLOC_SIZED(p,old,n) face_realloc(p,old,n)
#define STBI_FREE(p) ((void)(p))
#define STB_IMAGE_IMPLEMENTATION
#include "stb_image.h"

// ------------------------=
// FUNC: infinity_face_decode
// DESC: Decodes a bounded real 640x480 camera JPEG into RGB in isolated memory, wiping decoder scratch on every exit.
// ------------------=
int infinity_face_decode(const unsigned char *jpeg,size_t length,unsigned char *rgb,size_t rgb_length,
    unsigned char *scratch,size_t scratch_length) {
    if (!jpeg || !rgb || !scratch || length<4 || length>1228800 || rgb_length!=640*480*3 || scratch_length!=8*1024*1024) return -1;
    arena=scratch; arena_length=scratch_length; arena_used=0;
    int width=0,height=0,channels=0,status=-2;
    if (stbi_info_from_memory(jpeg,(int)length,&width,&height,&channels) && width==640 && height==480) {
        unsigned char *decoded=stbi_load_from_memory(jpeg,(int)length,&width,&height,&channels,3);
        if (decoded && width==640 && height==480) {memcpy(rgb,decoded,rgb_length);status=0;}
    }
    if (status) memset(rgb,0,rgb_length);
    for (size_t i=0;i<arena_used;++i) ((volatile unsigned char *)scratch)[i]=0;
    arena=NULL; arena_length=0; arena_used=0;
    return status;
}
