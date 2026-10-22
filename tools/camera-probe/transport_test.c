/* State-machine tests only. Synthetic packets are not real-camera evidence. */
#include <assert.h>
#include <string.h>
#include "uvc_transport.h"
static unsigned calls;
static int bulk_result;
static uint8_t flags=2;
// ------------------------=
// FUNC: control
// DESC: Supplies a bounded negotiation reply for deterministic protocol tests.
// ------------------=
static int control(void *ctx,uint8_t op,uint16_t selector,uint8_t *p,size_t n) {
    (void)ctx; (void)selector; assert(n==26);
    if(op==0x81) {p[18]=0;p[19]=0x60;p[20]=9;p[21]=0;p[22]=64;}
    return 0;
}
// ------------------------=
// FUNC: bulk
// DESC: Exercises transport outcomes without performing any real capture.
// ------------------=
static int bulk(void *ctx,uint8_t *p,size_t *n,unsigned timeout) {
    (void)ctx; assert(timeout==10); assert(*n==64); ++calls;
    const uint8_t packet[]={2,0,0xff,0xd8,0xff,0xd9};
    memcpy(p,packet,sizeof(packet)); p[1]=flags; *n=sizeof(packet);
    return bulk_result;
}
// ------------------------=
// FUNC: main
// DESC: Verifies bounded assembly, cancellation, deadline, device failure and malformed-packet rejection.
// ------------------=
int main(void) {
    static uint8_t payload[UVC_PAYLOAD_LIMIT],frame[UVC_FRAME_LIMIT];
    UvcIo io={0,control,bulk}; UvcStream s;
    assert(uvc_start(&s,io,0)==UVC_RUNNING);
    assert(uvc_step(&s,io,0,0,payload,frame)==UVC_FRAME);
    assert(s.length==4 && frame[0]==0xff && frame[3]==0xd9);
    unsigned before=calls;
    assert(uvc_step(&s,io,0,1,payload,frame)==UVC_CANCELLED && s.length==0);
    assert(uvc_step(&s,io,0,0,payload,frame)==UVC_CANCELLED && calls==before);
    assert(uvc_start(&s,io,0)==UVC_RUNNING);
    assert(uvc_step(&s,io,5000,0,payload,frame)==UVC_TIMEOUT && calls==before);
    assert(uvc_start(&s,io,0)==UVC_RUNNING); bulk_result=-1;
    assert(uvc_step(&s,io,0,0,payload,frame)==UVC_DISCONNECTED);
    assert(uvc_start(&s,io,0)==UVC_RUNNING); bulk_result=1;
    assert(uvc_step(&s,io,10,0,payload,frame)==UVC_RUNNING);
    bulk_result=0; flags=0x40;
    assert(uvc_step(&s,io,20,0,payload,frame)==UVC_PROTOCOL_ERROR && s.length==0);
    assert(uvc_start(&s,io,0)==UVC_RUNNING); flags=0; s.length=UVC_FRAME_LIMIT;s.synchronized=1;
    assert(uvc_step(&s,io,0,0,payload,frame)==UVC_PROTOCOL_ERROR && s.length==0);
    assert(uvc_start(&s,io,0)==UVC_RUNNING); flags=0;
    assert(uvc_step(&s,io,0,0,payload,frame)==UVC_RUNNING && s.length==4);
    bulk_result=1;
    assert(uvc_step(&s,io,10,0,payload,frame)==UVC_RUNNING && s.length==0);
    bulk_result=0; flags=2;
    assert(uvc_step(&s,io,20,0,payload,frame)==UVC_RUNNING && s.length==0);
    flags=3;
    assert(uvc_step(&s,io,30,0,payload,frame)==UVC_FRAME && s.length==4);
    return 0;
}
