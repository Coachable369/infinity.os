/* Executes the exact model-visible example with the real SDK and intercepted ABI. */
#define main sdk_probe_main
#include "native-app-sdk-test.c"
#undef main
#define main foundation_main
#include "../sdk/c/examples/generation-counter.c"
#undef main

// ------------------------=
// FUNC: main
// DESC: Checks real source state transitions, overflow rejection and SDK-owned lifetime without host window services.
// ------------------=
int main(void) {
    CHECK(sdk_probe_main()==0);
    AppEvent e={0};e.kind=APP_CLICK;e.id=1;
    value=INT_MAX;CHECK(handle(&e)==0 && value==INT_MAX);
    e.kind=APP_AI;e.id=2;value=INT_MIN;
    CHECK(handle(&e)==0 && value==INT_MIN);
    e.id=99;value=10;CHECK(handle(&e)==0 && value==10);
    e.kind=APP_CLOSE;e.id=1;CHECK(handle(&e)==0 && value==10);
    const int kinds[]={APP_CLICK,APP_AI,APP_CLICK,APP_CLOSE};
    const int ids[]={1,2,99,0};
    begin_loop(kinds,ids,4);value=0;
    uint64_t revision=infinity_app_revision;
    CHECK(foundation_main()==0);
    CHECK(value==0 && loop_at==4 && finishes==1 && yields==4);
    CHECK(receipt_packet.outcome==INFINITY_AI_APPLIED);
    CHECK(infinity_app_revision==revision+3);
    begin_loop(kinds,ids,4);write_result=-5;
    CHECK(foundation_main()!=0 && loop_at==0 && finishes==0);
    return 0;
}
