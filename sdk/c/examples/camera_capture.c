/* Isolated native capture client. No simulated frames or host camera API. */
#include <infinity/camera.h>
#include <stdio.h>
static unsigned char jpeg[4096];
// ------------------------=
// FUNC: main
// DESC: Reads a sequence-consistent JPEG through bounded isolated chunks and reports its checksum; not a recognition implementation.
// ------------------=
int main(void) {
    if(infinity_camera_start()<0) {puts("Camera denied or unavailable. Grant camera permission before launching.");return 1;}
    int result=2;
    for(unsigned attempt=0;attempt<20000;++attempt) {
        InfinityCameraFrame frame;
        int64_t status=infinity_camera_frame(&frame);
        if(status==-11) {infinity_system_call(INFINITY_SYS_YIELD,0,0,0,0,0,0);continue;}
        if(status<0 || frame.bytes<4 || frame.bytes>INFINITY_CAMERA_FRAME_LIMIT) break;
        uint64_t offset=0;
        unsigned hash=2166136261u;
        unsigned char first[2]={0},last[2]={0};
        while(offset<frame.bytes) {
            uint64_t count=frame.bytes-offset;if(count>4096) count=4096;
            status=infinity_camera_read(frame.sequence,offset,jpeg,count);
            if(status!=(int64_t)count) break;
            for(uint64_t i=0;i<count;++i) {
                if(offset+i<2) first[offset+i]=jpeg[i];
                last[0]=last[1];last[1]=jpeg[i];
                hash=(hash^jpeg[i])*16777619u;
            }
            offset+=count;
        }
        if(status==-116) continue;
        if(offset!=frame.bytes || first[0]!=255 || first[1]!=216 || last[0]!=255 || last[1]!=217) break;
        printf("Captured %u JPEG bytes in isolated memory; checksum %u\n",(unsigned)offset,hash);
        result=0;break;
    }
    infinity_camera_stop();
    for(unsigned i=0;i<sizeof(jpeg);++i) ((volatile unsigned char *)jpeg)[i]=0;
    return result;
}
