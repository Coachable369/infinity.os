#include <infinity/network.h>
#include <infinity/console.h>
#include <stdio.h>
#include <string.h>
// ------------------------=
// FUNC: wait_state
// DESC: Polls bounded native progress while yielding, preserving errors and rejecting terminal closure.
// ------------------=
static int wait_state(uint64_t handle, unsigned wanted, InfinityNetworkStatus *status) {
    for (unsigned i=0;i<20000;++i) {
        int64_t result=infinity_network_poll(handle,status);
        if (result<0 && result!=-11) return (int)result;
        if (result>=0 && status->state==wanted) return 0;
        if (result>=0 && (status->state==5 || status->state==6)) return -5;
        infinity_system_call(INFINITY_SYS_YIELD,0,0,0,0,0,0);
    }
    return -110;
}
// ------------------------=
// FUNC: main
// DESC: Sends one authorized HTTPS HEAD request using the actual asynchronous native TCP and TLS API.
// ------------------=
int main(void) {
    char host[254], request[512], response[1025];
    puts("Authorized HTTPS hostname:");
    int length=infinity_read_line(host,sizeof(host));
    if (length<=0) return 1;
    for(int i=0;i<length;++i) if(!((host[i]>='a'&&host[i]<='z') || (host[i]>='A'&&host[i]<='Z') || (host[i]>='0'&&host[i]<='9') || host[i]=='.' || host[i]=='-')) return 1;
    int64_t handle=infinity_network_open(host,(size_t)length,443,0);
    if(handle<0) {puts("Network denied or unavailable; use run --allow-network with an enabled network profile.");return 1;}
    InfinityNetworkStatus status; int result=1;
    if(wait_state(handle,INFINITY_NET_CONNECTED,&status)<0) goto done;
    if(infinity_network_command(handle,2,host,(size_t)length)<0 || wait_state(handle,INFINITY_NET_COMPLETE,&status)<0) goto done;
    length=snprintf(request,sizeof(request),"HEAD / HTTP/1.1\r\nHost: %s\r\nConnection: close\r\n\r\n",host);
    if(length<0 || (size_t)length>=sizeof(request)) goto done;
    for(size_t offset=0;offset<(size_t)length;) {
        size_t count=(size_t)length-offset;
        if(infinity_network_command(handle,1,request+offset,count)<0 || wait_state(handle,INFINITY_NET_COMPLETE,&status)<0 || !status.count || status.count>count) goto done;
        offset+=(size_t)status.count;
    }
    if(infinity_network_command(handle,0,0,1024)<0 || wait_state(handle,INFINITY_NET_COMPLETE,&status)<0) goto done;
    int64_t count=infinity_network_receive(handle,response,1024);
    if(count<0 || count>1024) goto done;
    response[count]=0;puts(response);result=0;
done:
    infinity_network_close(handle);return result;
}
