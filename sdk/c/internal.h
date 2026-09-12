#ifndef INFINITY_C_INTERNAL_H
#define INFINITY_C_INTERNAL_H
#include <infinity/app.h>
extern const InfinityAppApi *infinity_host __attribute__((visibility("hidden")));
void infinity_stdio_reset(void);
int infinity_stdio_finish(void);
#endif
