#include "tcl.h"
#include <stdio.h>
extern int Tk_Init(Tcl_Interp *interp);
extern int InfinityTclRegisterResources(void);
extern int InfinityTkImageAcceptance(void);
extern int InfinityTkDrawAcceptance(void);

// ------------------------=
// FUNC: check_integer
// DESC: Verifies typed results of actual upstream Tk commands and widget state transitions.
// ------------------=
static int check_integer(Tcl_Interp *interp,const char *source,int expected){
    int value=0;if(Tcl_EvalEx(interp,source,-1,TCL_EVAL_GLOBAL)!=TCL_OK){fprintf(stderr,"%s\n",Tcl_GetStringResult(interp));return 0;}
    return Tcl_GetIntFromObj(interp,Tcl_GetObjResult(interp),&value)==TCL_OK&&value==expected;
}
// ------------------------=
// FUNC: main
// DESC: Exercises real upstream Tk initialization, widgets, geometry, callbacks, text edits, timers and destruction in an authorized native context.
// ------------------=
int main(int argc,char **argv){
    (void)argc;(void)argv;Tcl_FindExecutable("/system/python/python.elf");
    if(InfinityTclRegisterResources()!=TCL_OK)return 1;Tcl_Interp *interp=Tcl_CreateInterp();if(!interp)return 2;
    if(Tcl_Init(interp)!=TCL_OK||Tk_Init(interp)!=TCL_OK){fprintf(stderr,"Tk_Init: %s\n",Tcl_GetStringResult(interp));return 3;}
    int pixels=InfinityTkImageAcceptance();if(pixels)return 100+pixels;
    pixels=InfinityTkDrawAcceptance();if(pixels)return 140+pixels;
    if(!check_integer(interp,"wm title . {Native Tk acceptance}; wm geometry . 480x320+40+60; set clicks 0; ttk::button .b -text {Run} -command {incr clicks}; pack .b; update; .b invoke; set clicks",1))return 4;
    if(!check_integer(interp,"entry .e; pack .e; .e insert 0 {hello}; .e delete 1 3; string equal [.e get] hlo",1))return 5;
    if(!check_integer(interp,"text .t -height 3; pack .t; .t insert end {a\nb\nc}; string equal [.t get 2.0 2.end] b",1))return 6;
    if(!check_integer(interp,"set timer 0; after 4 {set timer 7}; vwait timer; set timer",7))return 7;
    if(!check_integer(interp,"update; expr {[winfo width .] == 480 && [winfo height .] == 320 && [winfo exists .b]}",1))return 8;
    if(!check_integer(interp,"destroy .b; expr {![winfo exists .b]}",1))return 9;
    Tcl_EvalEx(interp,"destroy .",-1,TCL_EVAL_GLOBAL);Tcl_DeleteInterp(interp);Tcl_Finalize();return 54;
}
