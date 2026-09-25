#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <time.h>

extern int infinity_flite_synthesize(const unsigned char *,size_t,int16_t *,size_t,size_t *,size_t *,int (*)(void),size_t);
extern int infinity_flite_clean(void);
static int16_t pcm[480000], previous[480000];
static int checks;
// ------------------------=
// FUNC: cancel
// DESC: Cancels deterministically after synthesis has allocated intermediate state.
// ------------------=
static int cancel(void){return ++checks>100;}
// ------------------------=
// FUNC: u16
// DESC: Writes little-endian WAV metadata in the host-only evidence harness.
// ------------------=
static void u16(FILE *f,uint16_t v){fputc(v&255,f);fputc(v>>8,f);}
// ------------------------=
// FUNC: u32
// DESC: Writes portable 32-bit WAV header fields.
// ------------------=
static void u32(FILE *f,uint32_t v){u16(f,v&65535);u16(f,v>>16);}
// ------------------------=
// FUNC: main
// DESC: Exercises genuine synthesis, output bounds, error recovery, cancellation and private-state cleanup.
// ------------------=
int main(int argc,char **argv){
    const unsigned char *first=(const unsigned char *)"Hello. I am the native voice of Infinity OS. How can I help you?";
    size_t frames=0,peak=0;
    clock_t start=clock();
    assert(infinity_flite_synthesize(first,strlen((const char *)first),pcm,480000,&frames,&peak,NULL,8*1024*1024)==0);
    double seconds=(double)(clock()-start)/CLOCKS_PER_SEC;
    assert(frames>16000&&frames<480000&&peak>0&&peak<=8*1024*1024);
    size_t nonzero=0;for(size_t i=0;i<frames;i++)nonzero+=pcm[i]!=0;
    assert(nonzero>frames/4);assert(infinity_flite_clean());
    printf("frames=%zu rate=16000 arena_peak=%zu synthesis_seconds=%.6f rtf=%.6f\n",frames,peak,seconds,seconds/(frames/16000.0));
    if(argc==2){FILE *f=fopen(argv[1],"wb");assert(f);fwrite("RIFF",1,4,f);u32(f,36+(uint32_t)frames*2);fwrite("WAVEfmt ",1,8,f);u32(f,16);u16(f,1);u16(f,1);u32(f,16000);u32(f,32000);u16(f,2);u16(f,16);fwrite("data",1,4,f);u32(f,(uint32_t)frames*2);for(size_t i=0;i<frames;i++)u16(f,(uint16_t)pcm[i]);assert(fclose(f)==0);}
    memcpy(previous,pcm,frames*2);size_t reference=frames;
    for(int i=0;i<10;i++){assert(infinity_flite_synthesize(first,strlen((const char *)first),pcm,480000,&frames,&peak,NULL,8*1024*1024)==0);assert(frames==reference);assert(!memcmp(pcm,previous,frames*2));assert(infinity_flite_clean());}
    assert(infinity_flite_synthesize(first,strlen((const char *)first),pcm,1,&frames,&peak,NULL,8*1024*1024)==3);assert(frames==0);assert(infinity_flite_clean());
    assert(infinity_flite_synthesize(first,strlen((const char *)first),pcm,480000,&frames,&peak,NULL,1024)==3);assert(frames==0);assert(infinity_flite_clean());
    assert(infinity_flite_synthesize(first,strlen((const char *)first),pcm,480000,&frames,&peak,cancel,8*1024*1024)==2);assert(frames==0);assert(checks==101);assert(infinity_flite_clean());
    const unsigned char *second=(const unsigned char *)"Your system is online. The time is twelve thirty.";
    assert(infinity_flite_synthesize(second,strlen((const char *)second),pcm,480000,&frames,&peak,NULL,8*1024*1024)==0);assert(frames!=reference||memcmp(previous,pcm,frames*2));assert(infinity_flite_clean());
    assert(infinity_flite_synthesize(first,161,pcm,480000,&frames,&peak,NULL,8*1024*1024)==1);
    const unsigned char invalid[]={0xc3,0xa9};
    assert(infinity_flite_synthesize(invalid,2,pcm,480000,&frames,&peak,NULL,8*1024*1024)==1);
    puts("Native compact TTS behavioral tests: PASS");
}
