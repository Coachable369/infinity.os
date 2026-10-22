#ifndef INFINITY_CAMERA_H
#define INFINITY_CAMERA_H
#include <infinity/syscall.h>
#define INFINITY_CAMERA_FRAME_LIMIT 1228800u
typedef struct { uint64_t sequence, bytes; } InfinityCameraFrame;
// ------------------------=
// FUNC: infinity_camera_start
// DESC: Starts a bounded capture only when native policy has already granted this caller camera authority.
// ------------------=
static inline int64_t infinity_camera_start(void) {return infinity_system_call(0x1260,0,0,0,0,0,0);}
// ------------------------=
// FUNC: infinity_camera_frame
// DESC: Retrieves sequence and encoded byte length; -11 means no completed frame is available.
// ------------------=
static inline int64_t infinity_camera_frame(InfinityCameraFrame *frame) {return infinity_system_call(0x1261,0,0,0,0,(uint64_t)(uintptr_t)frame,0);}
// ------------------------=
// FUNC: infinity_camera_read
// DESC: Copies at most 4096 encoded JPEG bytes into private process memory; -116 requests a new sequence.
// ------------------=
static inline int64_t infinity_camera_read(uint64_t sequence,uint64_t offset,void *destination,uint64_t count) {
    return infinity_system_call(0x1261,sequence,offset,(uint64_t)(uintptr_t)destination,count,0,0);
}
// ------------------------=
// FUNC: infinity_camera_stop
// DESC: Invalidates pending frame delivery and requests transport shutdown without exposing USB handles.
// ------------------=
static inline int64_t infinity_camera_stop(void) {return infinity_system_call(0x1262,0,0,0,0,0,0);}
#endif
