#ifndef INFINITY_INTERACTION_H
#define INFINITY_INTERACTION_H
#include <stdint.h>
#include <infinity/syscall.h>

/* Publish ONLY information visible to the agent's assigned role, never hidden
 * cards or private documents. The OS supplies identity and permission. A choice
 * is one currently legal action with four signed integer arguments. Its label
 * describes their meaning. Republish choices after every user or agent action.
 * The app must validate each action again against its own current rules. */
#define INFINITY_AI_TURN 1u
#define INFINITY_AI_TERMINAL 2u
#define INFINITY_AI_READ_ONLY 0u
#define INFINITY_AI_REVERSIBLE 1u
#define INFINITY_AI_SENSITIVE 2u
#define INFINITY_AI_APPLIED 0u
#define INFINITY_AI_REJECTED 1u
typedef struct InfinityAiState {
    uint32_t version, flags;
    uint64_t revision;
    uint32_t length, reserved;
    char text[2048];
} InfinityAiState;
typedef struct InfinityAiChoice {
    uint32_t version, id, risk, reserved;
    char action[48], label[96];
    int64_t arguments[4];
} InfinityAiChoice;
typedef struct InfinityAiRequest {
    uint32_t version, choice;
    uint64_t token, instance, revision, deadline;
    int64_t arguments[4];
} InfinityAiRequest;
typedef struct InfinityAiResult {
    uint32_t version, outcome;
    uint64_t token;
} InfinityAiResult;
// ------------------------=
// FUNC: infinity_ai_begin
// DESC: Stages a new observation with a strictly increasing revision.
// ------------------=
static inline int64_t infinity_ai_begin(const InfinityAiState *state) {
    return infinity_system_call(0x1150,(uintptr_t)state,sizeof(*state),0,0,0,0);
}
// ------------------------=
// FUNC: infinity_ai_offer
// DESC: Stages one legal semantic action with a unique nonzero choice ID.
// ------------------=
static inline int64_t infinity_ai_offer(const InfinityAiChoice *choice) {
    return infinity_system_call(0x1151,(uintptr_t)choice,sizeof(*choice),0,0,0,0);
}
// ------------------------=
// FUNC: infinity_ai_commit
// DESC: Publishes the complete observation and choice set atomically.
// ------------------=
static inline int64_t infinity_ai_commit(void) {
    const uint32_t packet[2]={1,0};
    return infinity_system_call(0x1152,(uintptr_t)packet,sizeof(packet),0,0,0,0);
}
// ------------------------=
// FUNC: infinity_ai_poll
// DESC: Receives an agent request once; returns 72 bytes, zero if idle, or a negative error.
// ------------------=
static inline int64_t infinity_ai_poll(InfinityAiRequest *request) {
    request->version=1;
    return infinity_system_call(0x1153,(uintptr_t)request,sizeof(*request),0,0,0,0);
}
// ------------------------=
// FUNC: infinity_ai_finish
// DESC: Acknowledges a validated action; successful actions require publishing the new state first.
// ------------------=
static inline int64_t infinity_ai_finish(uint64_t token,uint32_t outcome) {
    const InfinityAiResult result={1,outcome,token};
    return infinity_system_call(0x1154,(uintptr_t)&result,sizeof(result),0,0,0,0);
}
#endif
