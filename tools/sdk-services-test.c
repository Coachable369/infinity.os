#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <infinity/syscall.h>
static int64_t intercept(uint64_t,uint64_t,uint64_t,uint64_t,uint64_t,uint64_t,uint64_t);
#define infinity_system_call intercept
#include <infinity/log.h>
#include <infinity/network.h>
#undef infinity_system_call
static unsigned calls;
static int mode;
static uint64_t operation, arguments[4];
static char record[2048];
// ------------------------=
// FUNC: intercept
// DESC: Records typed native requests and supplies bounded service responses for actual public SDK helpers.
// ------------------=
static int64_t intercept(uint64_t op,uint64_t a,uint64_t b,uint64_t c,uint64_t d,uint64_t e,uint64_t f) {
    if(e||f) __builtin_trap();
    ++calls;operation=op;arguments[0]=a;arguments[1]=b;arguments[2]=c;arguments[3]=d;
    if(mode==1)return -13;
    if(op==INFINITY_SYS_CONSOLE_WRITE) {
        if(b>=sizeof(record)||c!=2)__builtin_trap();
        memcpy(record,(void*)(uintptr_t)a,(size_t)b);record[b]=0;
        return (int64_t)b-(mode==2);
    }
    if(op==0x1272) {InfinityNetworkStatus *s=(void*)(uintptr_t)b;memset(s,0,sizeof(*s));s->state=4;s->count=3;}
    if(op==0x1273) {if(c<3)__builtin_trap();memcpy((void*)(uintptr_t)b,"abc",3);return 3;}
    return op==0x1270?7:0;
}
#define CHECK(x) do {if(!(x))__builtin_trap();}while(0)
// ------------------------=
// FUNC: main
// DESC: Exercises structured logs, failure propagation, native network arguments and response state.
// ------------------=
int main(void) {
    CHECK(sizeof(InfinityNetworkStatus)==24);
    CHECK(infinity_log(1,"demo","line\n\"quoted\"\\end")==0);printf("%s",record);
    unsigned before=calls;CHECK(infinity_log(5,"x","x")==-22);CHECK(calls==before);
    char huge[258];memset(huge,'x',257);huge[257]=0;
    CHECK(infinity_log(1,"x",huge)==-22);CHECK(calls==before);
    mode=1;CHECK(infinity_log(1,"x","x")==-13);
    mode=2;CHECK(infinity_log(1,"x","x")==-5);mode=0;
    CHECK(infinity_network_open("example.com",11,443,0)==7);
    CHECK(operation==0x1270&&arguments[0]==0&&arguments[2]==11&&arguments[3]==443);
    before=calls;CHECK(infinity_network_open("x",1,0,0)==-22);CHECK(infinity_network_command(7,2,"x",254)==-22);CHECK(calls==before);
    CHECK(infinity_network_command(7,1,"abc",3)==0);CHECK(operation==0x1271&&arguments[1]==1&&arguments[3]==3);
    InfinityNetworkStatus status;CHECK(infinity_network_poll(7,&status)==0);CHECK(status.state==4&&status.count==3);
    char bytes[4]={0};CHECK(infinity_network_receive(7,bytes,4)==3);CHECK(memcmp(bytes,"abc",3)==0);
    mode=1;CHECK(infinity_network_poll(7,&status)==-13);mode=0;
    CHECK(infinity_network_close(7)==0);CHECK(operation==0x1274&&arguments[0]==7);
    return 0;
}
