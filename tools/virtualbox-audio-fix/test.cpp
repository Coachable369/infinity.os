// Behavioral harness for the actual upstream capture functions, not a reimplementation.
#include <algorithm>
#include <cassert>
#include <cstdint>
#include <cstring>
#include <mutex>
#include <string>
#include <thread>
#define DECLCALLBACK(t) t
#define Log4Func(x) ((void)0)
#define Log3Func(x) ((void)0)
#define LogFunc(x) ((void)0)
#define LogFlowFunc(x) ((void)0)
#define LogRelMax(n,x) ((void)0)
#define STAM_PROFILE_ADD_PERIOD(a,b) ((void)0)
#define RT_NOREF(x) ((void)x)
#define RT_MIN(a,b) std::min(a,b)
#define RT_BZERO(p,n) std::memset(p,0,n)
#define RT_SUCCESS(x) ((x)>=0)
#define RT_FAILURE(x) ((x)<0)
#define Assert(x) assert(x)
#define AssertPtrReturn(x,r) if(!(x)) return r
#define AssertReturn(x,r) if(!(x)) return r
#define AssertReturnStmt(x,s,r) if(!(x)) {s;return r;}
#define AssertMsgReturnStmt(x,m,s,r) if(!(x)) {s;return r;}
#define AssertStmt(x,s) if(!(x)) {s;}
#define AssertBreakStmt(x,s) if(!(x)) {s;break;}
#define AssertPtrBreakStmt(x,s) if(!(x)) {s;break;}
constexpr int VINF_SUCCESS=0,VERR_INVALID_POINTER=-1,VERR_INVALID_PARAMETER=-2,
    VERR_AUDIO_STREAM_NOT_READY=-3,VERR_INTERNAL_ERROR_2=-4,COREAUDIOINITSTATE_INIT=1,
    COREAUDIO_MIN_BUFFERS=2,COREAUDIO_MAX_BUFFERS=32,noErr=0;
using OSStatus=int;
struct Props {uint8_t frame=4;};
struct Cfg {::Props Props;};
struct Buffer {uint32_t mAudioDataBytesCapacity,mAudioDataByteSize;void *mAudioData;bool queued=false;};
using AudioQueueBufferRef=Buffer*;
struct COREAUDIOBUF {Buffer *pBuf;uint32_t offRead=0;};
using PCOREAUDIOBUF=COREAUDIOBUF*;
struct COREAUDIOSTREAM {::Cfg Cfg;std::mutex CritSect;int enmInitState=1;bool fEnabled=true;
    COREAUDIOBUF *paBuffers;uint32_t cBuffers=2,idxBuffer=0;void *hAudioQueue=(void*)1;
    uint64_t offInternal=0,msLastTransfer=0;};
using PCOREAUDIOSTREAM=COREAUDIOSTREAM*;
using PPDMAUDIOBACKENDSTREAM=void*;
struct Host;
using PPDMIHOSTAUDIO=Host*;
struct Host {uint32_t(*pfnStreamGetReadable)(Host*,void*);int(*pfnStreamCapture)(Host*,void*,void*,uint32_t,uint32_t*);};
struct DRVAUDIO {Host *pHostDrvAudio;};using PDRVAUDIO=DRVAUDIO*;
struct DRVAUDIOSTREAM {struct {::Cfg Cfg;} Core;void *pBackend=nullptr;
    struct {struct {uint32_t cbBackendReadableBefore=0,cbBackendReadableAfter=0;} Stats;} In;
    uint64_t offInternal=0,nsLastPlayedCaptured=0;};using PDRVAUDIOSTREAM=DRVAUDIOSTREAM*;
// ------------------------=
// FUNC: PDMAudioPropsFrameSize
// DESC: Supplies PCM frame geometry to the upstream code.
// ------------------=
uint8_t PDMAudioPropsFrameSize(Props*p){return p->frame;}
// ------------------------=
// FUNC: PDMAudioPropsFloorBytesToFrame
// DESC: Aligns request bytes to complete frames.
// ------------------=
uint32_t PDMAudioPropsFloorBytesToFrame(Props*p,uint32_t n){return n/p->frame*p->frame;}
// ------------------------=
// FUNC: PDMAudioPropsIsSizeAligned
// DESC: Checks fixture PCM alignment.
// ------------------=
bool PDMAudioPropsIsSizeAligned(Props*p,uint32_t n){return n%p->frame==0;}
// ------------------------=
// FUNC: RTCritSectEnter
// DESC: Uses a real mutex so lock release is observable.
// ------------------=
void RTCritSectEnter(std::mutex*m){m->lock();}
// ------------------------=
// FUNC: RTCritSectLeave
// DESC: Releases the backend fixture mutex.
// ------------------=
void RTCritSectLeave(std::mutex*m){m->unlock();}
// ------------------------=
// FUNC: RTTimeNanoTS
// DESC: Supplies a deterministic successful-transfer timestamp.
// ------------------=
uint64_t RTTimeNanoTS(){return 123;}
// ------------------------=
// FUNC: RTTimeMilliTS
// DESC: Supplies the fixture CoreAudio timestamp.
// ------------------=
uint64_t RTTimeMilliTS(){return 1;}
// ------------------------=
// FUNC: drvHstAudCaIsBufferQueued
// DESC: Reports actual simulated AudioQueue buffer ownership.
// ------------------=
bool drvHstAudCaIsBufferQueued(Buffer*b){return b->queued;}
// ------------------------=
// FUNC: drvHstAudCaSetBufferQueued
// DESC: Records transfer back to AudioQueue.
// ------------------=
void drvHstAudCaSetBufferQueued(Buffer*b,bool q){b->queued=q;}
// ------------------------=
// FUNC: AudioQueueEnqueueBuffer
// DESC: Acknowledges native buffer submission without capturing a host microphone.
// ------------------=
int AudioQueueEnqueueBuffer(void*,Buffer*,int,void*){return 0;}
#include "functions.inc"
static unsigned calls=0,mode=0;
// ------------------------=
// FUNC: readable
// DESC: Models a backend which continues advertising data during an underrun.
// ------------------=
uint32_t readable(Host*,void*){return 16;}
// ------------------------=
// FUNC: capture
// DESC: Injects no-progress, partial-progress and backend-error results.
// ------------------=
int capture(Host*,void*,void*p,uint32_t,uint32_t*n){
    ++calls;*n=0;if(mode==2)return -9;
    if(mode==1&&calls==1){*n=4;std::memset(p,42,4);}return 0;
}
// ------------------------=
// FUNC: available
// DESC: Reports the CoreAudio fixture's actual unread, unqueued bytes.
// ------------------=
uint32_t available(Host*,void*p){
    auto*s=static_cast<COREAUDIOSTREAM*>(p);uint32_t n=0;
    for(unsigned i=0;i<s->cBuffers;++i)
        if(!s->paBuffers[i].pBuf->queued)n+=s->paBuffers[i].pBuf->mAudioDataByteSize-s->paBuffers[i].offRead;
    return n;
}
// ------------------------=
// FUNC: main
// DESC: Verifies termination, partial-read preservation, one-frame reads and released locks.
// ------------------=
int main(int argc,char**argv){
    assert(argc==2);std::string test=argv[1];
    if(test=="zero"||test=="partial"||test=="error"){
        mode=test=="partial"?1:test=="error"?2:0;
        Host host{readable,capture};DRVAUDIO driver{&host};DRVAUDIOSTREAM stream;
        uint8_t output[16]={};uint32_t got=99;
        int rc=drvAudioStreamCaptureLocked(&driver,&stream,output,16,&got);
        assert(rc==(mode==2?-9:0));assert(got==(mode==1?4u:0u));
        assert(calls==(mode==1?2u:1u));assert(stream.offInternal==got);
        if(mode==1)assert(output[0]==42&&output[3]==42&&output[4]==0);
    }else{
        uint8_t a[16]={1,2,3,4,5,6,7,8},b[16]={9,10,11,12};
        Buffer buffers[2]={{16,8,a},{16,4,b}};COREAUDIOBUF slots[2]={{&buffers[0]},{&buffers[1]}};
        COREAUDIOSTREAM stream;stream.paBuffers=slots;
        uint8_t out[16]={};uint32_t got=99;
        if(test=="empty")buffers[0].queued=true;
        if(test=="disabled")stream.fEnabled=false;
        unsigned wanted=test=="tail"?12:4;
        int rc;
        if(test=="integrated") {
            Host host{available,drvHstAudCaHA_StreamCapture};DRVAUDIO driver{&host};DRVAUDIOSTREAM outer;
            outer.pBackend=&stream;
            rc=drvAudioStreamCaptureLocked(&driver,&outer,out,wanted,&got);
            assert(outer.offInternal==got);
        } else {rc=drvHstAudCaHA_StreamCapture(nullptr,&stream,out,wanted,&got);}
        assert(rc==0);assert(got==((test=="empty"||test=="disabled")?0:wanted));
        for(unsigned i=0;i<got;++i)assert(out[i]==i+1);
        bool unlocked=false;
        std::thread other([&]{unlocked=stream.CritSect.try_lock();if(unlocked)stream.CritSect.unlock();});
        other.join();assert(unlocked);
        assert(stream.offInternal==got);
    }
}
