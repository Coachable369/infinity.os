#include <infinity/app.h>
#include <stdio.h>
#include <limits.h>
static int value;
// ------------------------=
// FUNC: render
// DESC: Publishes current data and only legal actions through the shared native SDK.
// ------------------=
static int render(void) {
    char s[32]; snprintf(s,sizeof(s),"%d",value);
    if(app_state(0,0,s)<0) return -1;
    if(app_status(s)<0 || app_cell(1,0,0,"+")<0 || app_cell(2,0,1,"-")<0) return -1;
    if(value<INT_MAX && app_offer(1,"Increase")<0) return -1;
    if(value>INT_MIN && app_offer(2,"Decrease")<0) return -1;
    return app_publish();
}
// ------------------------=
// FUNC: handle
// DESC: Validates both human and AI actions before changing application state.
// ------------------=
static int handle(const AppEvent *e) {
    if(e->kind!=APP_CLICK && e->kind!=APP_AI) return 0;
    if(e->id==1 && value<INT_MAX) value++;
    else if(e->id==2 && value>INT_MIN) value--;
    else return 0;
    return 1;
}
// ------------------------=
// FUNC: main
// DESC: Opens the existing native window and delegates close, yielding and acknowledgement to the SDK.
// ------------------=
int main(void) {
    if(app_grid("Counter",1,2)<0) return 1;
    return app_run(render,handle);
}
