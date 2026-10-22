/* Bounded UVC 1.0 MJPEG transport. No allocation, firmware types, or host services. */
#ifndef INFINITY_UVC_TRANSPORT_H
#define INFINITY_UVC_TRANSPORT_H
#include <stdint.h>
#include <stddef.h>
/* VirtualBox GET_CUR returns 4 bytes/pixel despite the 2-byte descriptor bound. */
#define UVC_FRAME_LIMIT 1228800u
#define UVC_PAYLOAD_LIMIT 65536u
typedef enum { UVC_IDLE, UVC_RUNNING, UVC_FRAME, UVC_CANCELLED, UVC_DISCONNECTED,
               UVC_TIMEOUT, UVC_PROTOCOL_ERROR } UvcState;
typedef struct {
    void *context;
    int (*control)(void *, uint8_t, uint16_t, uint8_t *, size_t);
    /* 0 success, 1 timeout, -1 device failure. Timeout must never be zero. */
    int (*bulk)(void *, uint8_t *, size_t *, unsigned);
} UvcIo;
typedef struct {
    UvcState state;
    size_t length, payload_limit;
    uint64_t deadline_ms;
    uint8_t fid, synchronized;
} UvcStream;

// ------------------------=
// FUNC: uvc_u32
// DESC: Reads an unaligned little endian device value without aliasing.
// ------------------=
static uint32_t uvc_u32(const uint8_t *p) {
    return (uint32_t)p[0] | (uint32_t)p[1]<<8 | (uint32_t)p[2]<<16 | (uint32_t)p[3]<<24;
}
// ------------------------=
// FUNC: uvc_cancel
// DESC: Invalidates queued frame data immediately; no further USB calls are issued.
// ------------------=
static void uvc_cancel(UvcStream *s) {
    s->length=0; s->synchronized=0; s->state=UVC_CANCELLED;
}
// ------------------------=
// FUNC: uvc_start
// DESC: Negotiates the observed VirtualBox 640x480 MJPEG mode at five frames/sec with strict bounds.
// ------------------=
static UvcState uvc_start(UvcStream *s, UvcIo io, uint64_t now_ms) {
    uint8_t probe[26]={1,0,1,5,0x80,0x84,0x1e,0};
    *s=(UvcStream){.state=UVC_IDLE};
    if(io.control(io.context,1,0x100,probe,sizeof(probe)) ||
       io.control(io.context,0x81,0x100,probe,sizeof(probe)))
        return s->state=UVC_DISCONNECTED;
    uint32_t frame=uvc_u32(probe+18), payload=uvc_u32(probe+22);
    if(probe[2]!=1 || probe[3]!=5 || uvc_u32(probe+4)!=2000000 ||
       !frame || frame>UVC_FRAME_LIMIT || payload<2 || payload>UVC_PAYLOAD_LIMIT)
        return s->state=UVC_PROTOCOL_ERROR;
    if(io.control(io.context,1,0x200,probe,sizeof(probe)))
        return s->state=UVC_DISCONNECTED;
    s->payload_limit=payload; s->deadline_ms=now_ms+5000;
    return s->state=UVC_RUNNING;
}
// ------------------------=
// FUNC: uvc_step
// DESC: Performs at most one ten-millisecond bulk read, reassembling a bounded frame or failing closed.
// ------------------=
static UvcState uvc_step(UvcStream *s, UvcIo io, uint64_t now_ms, int cancelled,
                         uint8_t *payload, uint8_t *frame) {
    if(cancelled) { uvc_cancel(s); return s->state; }
    if(s->state!=UVC_RUNNING) return s->state;
    if(now_ms>=s->deadline_ms) {s->length=0; return s->state=UVC_TIMEOUT;}
    size_t n=s->payload_limit;
    int result=io.bulk(io.context,payload,&n,10);
    if(result==1) return s->state;
    if(result) {s->length=0; return s->state=UVC_DISCONNECTED;}
    if(n<2 || n>s->payload_limit || payload[0]<2 || payload[0]>n || (payload[1]&0x40)) {
        s->length=0; return s->state=UVC_PROTOCOL_ERROR;
    }
    uint8_t fid=payload[1]&1;
    if(!s->synchronized || fid!=s->fid) { s->length=0; s->fid=fid; s->synchronized=1; }
    size_t bytes=n-payload[0];
    if(bytes>UVC_FRAME_LIMIT-s->length) {s->length=0; return s->state=UVC_PROTOCOL_ERROR;}
    for(size_t i=0;i<bytes;++i) frame[s->length+i]=payload[payload[0]+i];
    s->length+=bytes;
    if(payload[1]&2) {
        if(s->length<4 || frame[0]!=0xff || frame[1]!=0xd8 ||
           frame[s->length-2]!=0xff || frame[s->length-1]!=0xd9) {
            s->length=0; return s->state=UVC_PROTOCOL_ERROR;
        }
        s->state=UVC_FRAME;
    }
    return s->state;
}
#endif
