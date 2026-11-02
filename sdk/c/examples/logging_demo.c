#include <infinity/log.h>
// ------------------------=
// FUNC: main
// DESC: Emits one native structured diagnostic and returns failure if delivery fails.
// ------------------=
int main(void) { return infinity_log(INFINITY_LOG_INFO,"demo","Native application started") < 0; }
