#include "tcl.h"
#include <stdio.h>
#include <string.h>
extern int InfinityTclRegisterResources(void);

// ------------------------=
// FUNC: check_integer
// DESC: Evaluates real Tcl bytecode and verifies its typed integer outcome.
// ------------------=
static int check_integer(Tcl_Interp *interp, const char *script, int expected) {
    int value = 0;
    if (Tcl_EvalEx(interp, script, -1, TCL_EVAL_GLOBAL) != TCL_OK) {
        fprintf(stderr, "%s\n", Tcl_GetStringResult(interp)); return 0;
    }
    return Tcl_GetIntFromObj(interp, Tcl_GetObjResult(interp), &value) == TCL_OK && value == expected;
}
// ------------------------=
// FUNC: main
// DESC: Verifies actual Tcl initialization, libraries, procedures, Unicode data, bignums, events and fail-closed process authority.
// ------------------=
int main(int argc, char **argv) {
    (void)argc; (void)argv;
    Tcl_FindExecutable("/system/python/python.elf");
    if (InfinityTclRegisterResources() != TCL_OK) return 1;
    Tcl_Interp *interp = Tcl_CreateInterp();
    if (!interp) return 2;
    if (Tcl_Init(interp) != TCL_OK) {
        fprintf(stderr, "Tcl_Init: %s\n", Tcl_GetStringResult(interp)); return 3;
    }
    if (!check_integer(interp, "proc square {x} {expr {$x*$x}}; square 7", 49)) return 4;
    if (!check_integer(interp, "set d [dict create answer 42]; dict get $d answer", 42)) return 5;
    if (!check_integer(interp, "expr {(2**100 + 1) % 3}", 2)) return 6;
    if (!check_integer(interp, "string length \u4e2d\u6587", 2)) return 7;
    if (!check_integer(interp, "set ready 0; after 2 {set ready 1}; vwait ready; set ready", 1)) return 8;
    if (!check_integer(interp, "package require msgcat; string equal [::msgcat::mc hello] hello", 1)) return 9;
    if (!check_integer(interp, "catch {exec unauthorized-program}", 1)) return 10;
    if (!check_integer(interp, "catch {open /system/python/tcl8.6/init.tcl w}", 1)) return 11;
    Tcl_DeleteInterp(interp);
    Tcl_Finalize();
    puts("Native Tcl behavior completed");
    return 54;
}
