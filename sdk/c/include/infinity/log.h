#ifndef INFINITY_LOG_H
#define INFINITY_LOG_H
#include <infinity/syscall.h>
#include <stddef.h>
enum { INFINITY_LOG_DEBUG=0, INFINITY_LOG_INFO=1, INFINITY_LOG_WARNING=2, INFINITY_LOG_ERROR=3 };
// ------------------------=
// FUNC: infinity_log_escape
// DESC: Encodes a bounded JSON string, preserving UTF-8 bytes and escaping record-breaking control characters.
// ------------------=
static inline int infinity_log_escape(char *out, const char *value, size_t limit) {
    const char hex[]="0123456789abcdef"; size_t n=0;
    if (!value) return -22;
    out[n++]='"';
    for (size_t i=0;value[i];++i) {
        if (i>=limit) return -22;
        unsigned char b=(unsigned char)value[i];
        if (b<32) {out[n++]='\\';out[n++]='u';out[n++]='0';out[n++]='0';out[n++]=hex[b>>4];out[n++]=hex[b&15];}
        else {if (b=='"'||b=='\\') out[n++]='\\';out[n++]=(char)b;}
    }
    out[n++]='"'; return (int)n;
}
// ------------------------=
// FUNC: infinity_log
// DESC: Writes one versioned bounded JSON diagnostic record to native stderr, propagating errors and short writes.
// ------------------=
static inline int64_t infinity_log(unsigned level, const char *component, const char *message) {
    if (level>3) return -22;
    char out[2048]; size_t n=0;
    const char *prefix="{\"version\":1,\"level\":";
    while (*prefix) out[n++]=*prefix++;
    out[n++]=(char)('0'+level);
    prefix=",\"component\":";while (*prefix) out[n++]=*prefix++;
    int length=infinity_log_escape(out+n,component,32);if(length<0)return length;n+=(size_t)length;
    prefix=",\"message\":";while (*prefix) out[n++]=*prefix++;
    length=infinity_log_escape(out+n,message,256);if(length<0)return length;n+=(size_t)length;
    out[n++]='}';out[n++]='\n';
    int64_t result=infinity_system_call(INFINITY_SYS_CONSOLE_WRITE,(uintptr_t)out,n,2,0,0,0);
    return result<0 ? result : result==(int64_t)n ? 0 : -5;
}
#endif
