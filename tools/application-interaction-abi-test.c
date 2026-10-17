#include <stddef.h>
#include <infinity/interaction.h>
_Static_assert(sizeof(InfinityAiState)==2072,"state ABI");
_Static_assert(sizeof(InfinityAiChoice)==192,"choice ABI");
_Static_assert(sizeof(InfinityAiRequest)==72,"request ABI");
_Static_assert(sizeof(InfinityAiResult)==16,"result ABI");
_Static_assert(offsetof(InfinityAiState,text)==24,"state text offset");
_Static_assert(offsetof(InfinityAiChoice,arguments)==160,"choice arguments offset");
_Static_assert(offsetof(InfinityAiRequest,arguments)==40,"request arguments offset");
// ------------------------=
// FUNC: probe
// DESC: Compiles all SDK entry points using the actual target-specific system-call adapter.
// ------------------=
int64_t probe(InfinityAiState *state,InfinityAiChoice *choice,InfinityAiRequest *request) {
    int64_t result=infinity_ai_begin(state);
    result+=infinity_ai_offer(choice);
    result+=infinity_ai_commit();
    result+=infinity_ai_poll(request);
    return result+infinity_ai_finish(request->token,INFINITY_AI_REJECTED);
}
