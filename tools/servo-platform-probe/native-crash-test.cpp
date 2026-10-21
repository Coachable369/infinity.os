#include "mozilla/Assertions.h"
#include <stdlib.h>

// ------------------------=
// FUNC: abort
// DESC: Stands in for the native supervisor's fatal callback; the exit code proves no null-store trap intercepted it.
// ------------------=
extern "C" void abort(void) { _Exit(73); }

class ShadowedAbort {
    // ------------------------=
    // FUNC: abort
    // DESC: Matches engine classes whose task-abort method must not shadow the global fatal callback.
    // ------------------=
    void abort(int) { _Exit(74); }
public:
    // ------------------------=
    // FUNC: crash
    // DESC: Requires the macro to resolve global native abort even inside a class with its own abort method.
    // ------------------=
    void crash() { MOZ_REALLY_CRASH(__LINE__); }
};

// ------------------------=
// FUNC: main
// DESC: Exercises the staged SpiderMonkey fatal macro instead of inspecting its source spelling.
// ------------------=
int main() { ShadowedAbort test; test.crash(); }
