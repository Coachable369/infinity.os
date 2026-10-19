/* Behavioral host probe: real public SDK helpers, intercepted native syscall boundary. */
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <infinity/syscall.h>

// ------------------------=
// FUNC: intercepted_system_call
// DESC: Intercepts the actual SDK syscall boundary without executing a host system call.
// ------------------=
static int64_t intercepted_system_call(uint64_t operation, uint64_t a0, uint64_t a1,
    uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5);
#define infinity_system_call intercepted_system_call
#include <infinity/app.h>
#undef infinity_system_call

_Static_assert(INFINITY_APP_ABI==2u,"native application entry ABI version");
_Static_assert(sizeof(InfinityObjectApi)==48,"native object callbacks ABI");
_Static_assert(sizeof(InfinityAppApi)==32,"native application entry ABI");
_Static_assert(offsetof(InfinityAppApi,context)==8,"native application context offset");
_Static_assert(offsetof(InfinityAppApi,write_stdout)==16,"native console callback offset");
_Static_assert(offsetof(InfinityAppApi,objects)==24,"native object callbacks offset");
_Static_assert(sizeof(InfinityUiWindow)==64,"native window ABI");
_Static_assert(sizeof(InfinityUiElement)==64,"native element ABI");
_Static_assert(sizeof(InfinityUiEvent)==16,"native UI event ABI");
_Static_assert(sizeof(InfinityAiState)==2072,"native state ABI");
_Static_assert(sizeof(InfinityAiChoice)==192,"native choice ABI");
_Static_assert(sizeof(InfinityAiAction)==416,"native typed action ABI");
_Static_assert(sizeof(InfinityAiTypedRequest)==592,"native typed request ABI");

static unsigned calls, ui_polls, ai_polls, finishes, yields;
static int64_t write_result, ui_result, ai_result;
static uint64_t last_operation;
static InfinityUiWindow window_packet;
static InfinityUiElement element_packet;
static InfinityAiState state_packet;
static InfinityAiChoice choice_packet;
static InfinityAiAction action_packet;
static InfinityAiResult receipt_packet;
static InfinityUiEvent next_ui;
static InfinityAiTypedRequest next_ai;
static int loop_enabled, loop_kinds[8], loop_ids[8], loop_poll_error, loop_finish_error;
static unsigned loop_at, loop_length, loop_renders, loop_handles, loop_fail_render, loop_render_mode;
static unsigned loop_trace[64], loop_trace_length;
enum { LOOP_RENDER=1, LOOP_HANDLE, LOOP_APPLIED, LOOP_REJECTED, LOOP_YIELD };

// ------------------------=
// FUNC: check
// DESC: Fails the executable on a behavioral mismatch while keeping diagnostic prose outside the acceptance oracle.
// ------------------=
static void check(int condition,int line) {
    if(!condition) {printf("Native app SDK assertion failed at line %d\n",line);fflush(NULL);__builtin_trap();}
}
#define CHECK(value) check((value),__LINE__)

// ------------------------=
// FUNC: record_loop
// DESC: Records bounded typed lifecycle events to verify publication-before-acknowledgment and scheduler yielding.
// ------------------=
static void record_loop(unsigned event) {
    CHECK(loop_trace_length<sizeof(loop_trace)/sizeof(loop_trace[0]));
    loop_trace[loop_trace_length++]=event;
}

// ------------------------=
// FUNC: intercepted_system_call
// DESC: Validates packet size, version, and reserved arguments before returning controlled service results or event bytes.
// ------------------=
static int64_t intercepted_system_call(uint64_t operation, uint64_t a0, uint64_t a1,
    uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5) {
    calls++;last_operation=operation;
    CHECK(a2==0 && a3==0 && a4==0 && a5==0);
    if(operation==INFINITY_SYS_YIELD) {
        CHECK(a0==0 && a1==0);yields++;
        if(loop_enabled) {record_loop(LOOP_YIELD);loop_at++;}
        return 0;
    }
    CHECK(a0!=0);
    const void *packet=(const void *)(uintptr_t)a0;
    uint32_t version=0;memcpy(&version,packet,sizeof(version));CHECK(version==1);
    switch(operation) {
        case INFINITY_SYS_UI_OPEN:
            CHECK(a1==sizeof(window_packet));memcpy(&window_packet,packet,sizeof(window_packet));break;
        case INFINITY_SYS_UI_UPSERT:
            CHECK(a1==sizeof(element_packet));memcpy(&element_packet,packet,sizeof(element_packet));break;
        case INFINITY_SYS_UI_POLL:
            CHECK(a1==sizeof(next_ui));ui_polls++;
            if(loop_enabled) {
                CHECK(loop_at<loop_length);
                if(loop_poll_error) return loop_poll_error;
                memset(&next_ui,0,sizeof(next_ui));next_ui.version=1;
                int kind=loop_kinds[loop_at];
                ui_result=kind==APP_CLICK || kind==APP_CLOSE;
                next_ui.kind=kind;next_ui.id=loop_ids[loop_at];
            }
            if(ui_result>0) memcpy((void *)(uintptr_t)a0,&next_ui,sizeof(next_ui));
            return ui_result;
        case 0x1150:
            CHECK(a1==sizeof(state_packet));memcpy(&state_packet,packet,sizeof(state_packet));
            CHECK(state_packet.reserved==0);break;
        case 0x1151:
            CHECK(a1==sizeof(choice_packet));memcpy(&choice_packet,packet,sizeof(choice_packet));
            CHECK(choice_packet.reserved==0);break;
        case 0x1152: {
            uint32_t words[2];CHECK(a1==sizeof(words));memcpy(words,packet,sizeof(words));
            CHECK(words[1]==0);break;
        }
        case 0x1154:
            CHECK(a1==sizeof(receipt_packet));memcpy(&receipt_packet,packet,sizeof(receipt_packet));
            finishes++;
            if(loop_enabled) {
                CHECK(receipt_packet.token==100+loop_at);
                record_loop(receipt_packet.outcome==INFINITY_AI_APPLIED?LOOP_APPLIED:LOOP_REJECTED);
                return loop_finish_error;
            }
            break;
        case 0x1155:
            CHECK(a1==sizeof(action_packet));memcpy(&action_packet,packet,sizeof(action_packet));
            CHECK(action_packet.choice.reserved==0 && action_packet.reserved==0);break;
        case 0x1156:
            CHECK(a1==sizeof(next_ai));ai_polls++;
            if(loop_enabled) {
                CHECK(loop_at<loop_length);
                memset(&next_ai,0,sizeof(next_ai));next_ai.request.version=1;
                next_ai.request.choice=loop_ids[loop_at];next_ai.request.token=100+loop_at;
                next_ai.request.instance=7;next_ai.request.revision=infinity_app_revision;
                ai_result=loop_kinds[loop_at]==APP_AI?(int64_t)sizeof(next_ai):0;
            }
            if(ai_result>0) memcpy((void *)(uintptr_t)a0,&next_ai,sizeof(next_ai));
            return ai_result;
        default: CHECK(0);return -1;
    }
    return write_result;
}

// ------------------------=
// FUNC: reset_boundary
// DESC: Clears only fake service responses and observed packet counts, leaving production SDK revision state intact.
// ------------------=
static void reset_boundary(void) {
    calls=ui_polls=ai_polls=finishes=yields=0;
    write_result=ui_result=ai_result=0;last_operation=0;
    loop_enabled=0;
    memset(&next_ui,0,sizeof(next_ui));memset(&next_ai,0,sizeof(next_ai));
}

// ------------------------=
// FUNC: same_bytes
// DESC: Compares binary payload bytes and their exact length rather than using rendered output as evidence.
// ------------------=
static void same_bytes(const void *left,const void *right,size_t length) {
    CHECK(memcmp(left,right,length)==0);
}

// ------------------------=
// FUNC: test_ui_packets
// DESC: Exercises bounded titles and geometry, themed element routing, error propagation, and scheduler yield packets.
// ------------------=
static void test_ui_packets(void) {
    reset_boundary();
    CHECK(app_window("Workspace",320,240)==0);
    CHECK(last_operation==INFINITY_SYS_UI_OPEN);
    CHECK(window_packet.width==320 && window_packet.height==240 && window_packet.title_length==9);
    same_bytes(window_packet.title,"Workspace",10);
    CHECK(app_window("Workspace",960,720)==0);
    unsigned before=calls;
    CHECK(app_window("Too narrow",319,240)<0);
    CHECK(app_window("Too wide",961,240)<0);
    CHECK(app_window("Too short",320,239)<0);
    CHECK(app_window("Too high",320,721)<0);
    CHECK(app_window(NULL,320,240)<0);
    CHECK(calls==before);
    char title[49];memset(title,'T',sizeof(title));title[47]=0;
    CHECK(app_window(title,480,320)==0);CHECK(window_packet.title_length==47);
    title[47]='T';title[48]=0;before=calls;
    CHECK(app_window(title,480,320)<0);CHECK(calls==before);

    CHECK(app_button(7,8,16,96,40,"Run")==0);
    CHECK(element_packet.id==7 && element_packet.kind==INFINITY_UI_BUTTON);
    CHECK(element_packet.x==8 && element_packet.y==16 && element_packet.width==96 && element_packet.height==40);
    CHECK(element_packet.label_length==3);same_bytes(element_packet.label,"Run",4);
    CHECK(app_label(8,0,0,128,24,"")==0);CHECK(element_packet.kind==INFINITY_UI_LABEL);
    CHECK(element_packet.label_length==0);
    CHECK(app_display(9,0,24,128,32,"42")==0);CHECK(element_packet.kind==INFINITY_UI_DISPLAY);
    before=calls;
    CHECK(app_button(0,0,0,1,1,"x")<0);CHECK(app_button(-1,0,0,1,1,"x")<0);
    CHECK(app_button(1,-1,0,1,1,"x")<0);CHECK(app_button(1,0,-1,1,1,"x")<0);
    CHECK(app_button(1,0,0,0,1,"x")<0);CHECK(app_button(1,0,0,1,-1,"x")<0);
    CHECK(app_button(1,0,0,1,1,NULL)<0);CHECK(calls==before);
    char label[33];memset(label,'L',sizeof(label));label[31]=0;
    CHECK(app_button(1,0,0,1,1,label)==0);CHECK(element_packet.label_length==31);
    label[31]='L';label[32]=0;before=calls;CHECK(app_button(1,0,0,1,1,label)<0);CHECK(calls==before);
    write_result=-13;CHECK(app_window("Denied",480,320)==-13);
    CHECK(app_button(1,0,0,10,10,"Denied")==-13);write_result=0;
    app_yield();CHECK(yields==1 && last_operation==INFINITY_SYS_YIELD);
}

// ------------------------=
// FUNC: deliver_ai
// DESC: Queues one concrete versioned action packet through the SDK's actual UI-first event polling path.
// ------------------=
static int deliver_ai(uint64_t revision,AppEvent *event) {
    memset(&next_ai,0,sizeof(next_ai));ui_result=0;ai_result=sizeof(next_ai);
    next_ai.request.version=1;next_ai.request.revision=revision;
    next_ai.request.choice=12;next_ai.request.token=99;next_ai.request.instance=7;
    next_ai.request.arguments[0]=-4;next_ai.request.arguments[1]=8;
    next_ai.request.arguments[2]=INT64_MAX;next_ai.request.arguments[3]=INT64_MIN;
    return app_event(event);
}

// ------------------------=
// FUNC: test_publication
// DESC: Verifies staged versus committed revisions, failed publication retry, flags, exact state boundaries, and overflow rejection.
// ------------------=
static void test_publication(void) {
    reset_boundary();AppEvent event;
    CHECK(app_publish()<0);CHECK(calls==0);
    CHECK(app_state(1,0,"User-visible state")==0);
    CHECK(state_packet.revision==1 && state_packet.flags==INFINITY_AI_TURN && state_packet.length==18);
    same_bytes(state_packet.text,"User-visible state",19);
    CHECK(app_publish()==0);CHECK(last_operation==0x1152);
    CHECK(deliver_ai(1,&event)==1);CHECK(event.kind==APP_AI && event.token==99);

    write_result=-13;CHECK(app_state(0,0,"Denied replacement")==-13);
    CHECK(deliver_ai(1,&event)==1);write_result=0;
    CHECK(app_state(0,1,"Finished")==0);
    CHECK(state_packet.revision==2 && state_packet.flags==INFINITY_AI_TERMINAL);
    CHECK(deliver_ai(1,&event)==1); /* Staging must not retire published authority. */
    write_result=-16;CHECK(app_publish()==-16);
    CHECK(deliver_ai(1,&event)==1);write_result=0;
    CHECK(app_publish()==0);CHECK(deliver_ai(2,&event)==1);
    unsigned before=finishes;CHECK(deliver_ai(1,&event)==0);CHECK(finishes==before+1);
    CHECK(receipt_packet.token==99 && receipt_packet.outcome==INFINITY_AI_REJECTED);

    before=calls;CHECK(app_state(0,0,NULL)<0);CHECK(app_state(0,0,"")<0);CHECK(calls==before);
    char state[2049];memset(state,'S',sizeof(state));state[2047]=0;
    CHECK(app_state(1,1,state)==0);CHECK(state_packet.length==2047 && state_packet.flags==3);
    CHECK(app_publish()==0);
    state[2047]='S';state[2048]=0;before=calls;
    CHECK(app_state(0,0,state)<0);CHECK(calls==before);
    uint64_t saved=infinity_app_revision;infinity_app_revision=UINT64_MAX;
    CHECK(app_state(0,0,"Overflow")<0);CHECK(calls==before);infinity_app_revision=saved;
}

// ------------------------=
// FUNC: test_state_status_not_data
// DESC: Proves observation calls return native status while application-owned data and initialized description remain independent, with publication still explicit.
// ------------------=
static void test_state_status_not_data(void) {
    reset_boundary();
    int value=37;char description[32];
    int length=snprintf(description,sizeof(description),"%d",value);
    CHECK(length==2);
    uint64_t revision=infinity_app_revision;
    CHECK(app_state(0,0,description)==0);
    CHECK(value==37 && infinity_app_revision==revision);
    CHECK(state_packet.length==(uint32_t)length);
    same_bytes(state_packet.text,description,(size_t)length+1);
    CHECK(app_publish()==0 && infinity_app_revision==revision+1);
    value=-9;length=snprintf(description,sizeof(description),"%d",value);
    CHECK(length==2);write_result=-13;
    CHECK(app_state(0,0,description)==-13);
    CHECK(value==-9 && infinity_app_revision==revision+1);
    write_result=0;
}

// ------------------------=
// FUNC: registry_identifier
// DESC: Independently checks the published native registry's alphabetic-first identifier contract at the packet boundary.
// ------------------=
static int registry_identifier(const char *value,size_t capacity) {
    size_t length=0;
    while(length<capacity && value[length]) length++;
    if(length==0 || length==capacity || !((value[0]>='a' && value[0]<='z') || (value[0]>='A' && value[0]<='Z'))) return 0;
    for(size_t i=0;i<length;i++) {
        char c=value[i];
        if(!((c>='a' && c<='z') || (c>='A' && c<='Z') || (c>='0' && c<='9') || c=='.' || c=='_')) return 0;
    }
    return 1;
}

// ------------------------=
// FUNC: test_choices
// DESC: Verifies reversible fixed and typed action packets, signed bounds, collision-free parameter names, and rejection without truncation.
// ------------------=
static void test_choices(void) {
    reset_boundary();
    CHECK(app_choice(12,"document.move","Move item",0,0,0,0)<0);CHECK(calls==0);
    CHECK(app_text_action(20,"document.edit","Edit","offset",0,1,"text")<0);CHECK(calls==0);
    CHECK(app_state(1,0,"Choice contract")==0);
    CHECK(app_choice(12,"document.move","Move item",-9,2,3,4)==0);
    CHECK(choice_packet.id==12 && choice_packet.risk==INFINITY_AI_REVERSIBLE);
    CHECK(choice_packet.arguments[0]==-9 && choice_packet.arguments[3]==4);
    same_bytes(choice_packet.action,"document.move",14);same_bytes(choice_packet.label,"Move item",10);
    unsigned before=calls;CHECK(app_choice(0,"document.move","Move",0,0,0,0)<0);
    CHECK(app_choice(1,NULL,"Move",0,0,0,0)<0);CHECK(calls==before);
    char action[49];memset(action,'a',sizeof(action));action[48]=0;
    CHECK(app_choice(1,action,"Move",0,0,0,0)<0);CHECK(calls==before);
    char label[97];memset(label,'L',sizeof(label));label[96]=0;
    CHECK(app_choice(1,"document.move",label,0,0,0,0)<0);CHECK(calls==before);
    CHECK(app_state(1,0,"Choice contract")==0);
    write_result=-16;CHECK(app_choice(12,"document.move","Move",0,0,0,0)==-16);write_result=0;
    CHECK(app_state(1,0,"Typed contract")==0);

    const char *parameters[]={"offset","unused0","unused1","unused2","unused3","unused4"};
    for(size_t case_id=0;case_id<sizeof(parameters)/sizeof(parameters[0]);case_id++) {
        const char *name=parameters[case_id];const char *text_name=strcmp(name,"unused4")==0?"unused0":"unused4";
        CHECK(app_text_action(20,"document.edit","Edit text",name,-3,77,text_name)==0);
        CHECK(last_operation==0x1155 && action_packet.choice.id==20);
        CHECK(action_packet.choice.risk==INFINITY_AI_REVERSIBLE && action_packet.text_limit==512);
        CHECK(action_packet.parameters[0].kind==INFINITY_AI_INTEGER);
        CHECK(action_packet.parameters[0].minimum==-3 && action_packet.parameters[0].maximum==77);
        CHECK(action_packet.choice.arguments[0]==-3);
        for(int i=0;i<4;i++) {
            InfinityAiParameter *parameter=&action_packet.parameters[i];
            CHECK(registry_identifier(parameter->name,sizeof(parameter->name)));
            CHECK(strcmp(parameter->name,action_packet.text_name)!=0 && parameter->reserved==0);
            for(int j=0;j<i;j++) CHECK(strcmp(parameter->name,action_packet.parameters[j].name)!=0);
            if(i) CHECK(parameter->kind==INFINITY_AI_FIXED && parameter->minimum==0 && parameter->maximum==0 && action_packet.choice.arguments[i]==0);
        }
    }
    char name[25];memset(name,'n',sizeof(name));name[24]=0;
    for(int invalid=0;invalid<6;invalid++) {
        CHECK(app_state(1,0,"Typed contract")==0);before=calls;
        switch(invalid) {
            case 0:CHECK(app_text_action(1,"document.edit","Edit","offset",2,1,"text")<0);break;
            case 1:CHECK(app_text_action(1,"document.edit","Edit","same",0,1,"same")<0);break;
            case 2:CHECK(app_text_action(1,"document.edit","Edit","_offset",0,1,"text")<0);break;
            case 3:CHECK(app_text_action(1,"document.edit","Edit","offset",0,1,NULL)<0);break;
            case 4:CHECK(app_text_action(1,"document.edit","Edit",name,0,1,"text")<0);break;
            default:CHECK(app_text_action(1,"document.edit","","offset",0,1,"text")<0);break;
        }
        CHECK(calls==before);CHECK(app_publish()<0);CHECK(calls==before);
    }
}

// ------------------------=
// FUNC: test_incomplete_contracts
// DESC: Prevents partial action publication after local or native staging failures and allows only a new observation to recover.
// ------------------=
static void test_incomplete_contracts(void) {
    reset_boundary();AppEvent event;
    CHECK(app_state(1,0,"Published contract")==0);
    CHECK(app_choice(1,"document.inspect","Inspect",0,0,0,0)==0);
    CHECK(app_publish()==0);
    uint64_t revision=infinity_app_revision;
    for(int failure=0;failure<11;failure++) {
        CHECK(app_state(1,0,"Candidate replacement")==0);
        CHECK(app_choice(1,"document.inspect","Inspect",0,0,0,0)==0);
        unsigned before=calls;
        switch(failure) {
            case 0:CHECK(app_choice(2,"_invalid","Edit",0,0,0,0)<0);CHECK(calls==before);break;
            case 1:CHECK(app_choice(2,"document-edit","Edit",0,0,0,0)<0);CHECK(calls==before);break;
            case 2:CHECK(app_choice(2,"document.edit","",0,0,0,0)<0);CHECK(calls==before);break;
            case 3:CHECK(app_choice(0,"document.edit","Edit",0,0,0,0)<0);CHECK(calls==before);break;
            case 4:CHECK(app_choice(2,NULL,"Edit",0,0,0,0)<0);CHECK(calls==before);break;
            case 5:write_result=-28;CHECK(app_choice(2,"document.edit","Edit",0,0,0,0)==-28);CHECK(calls==before+1);break;
            case 6:write_result=-22;CHECK(app_choice(1,"document.inspect","Duplicate",0,0,0,0)==-22);CHECK(calls==before+1);break;
            case 7:write_result=-13;CHECK(app_text_action(2,"document.edit","Edit","offset",0,1,"text")==-13);CHECK(calls==before+1);break;
            case 8:CHECK(app_text_action(2,"document.edit","Edit","same",0,1,"same")<0);CHECK(calls==before);break;
            case 9:CHECK(app_state(1,0,"")<0);CHECK(calls==before);break;
            default:write_result=-13;CHECK(app_state(1,0,"Denied replacement")==-13);CHECK(calls==before+1);break;
        }
        write_result=0;before=calls;
        CHECK(app_publish()<0);CHECK(calls==before);
        CHECK(app_choice(3,"document.recover","Cannot patch failed transaction",0,0,0,0)<0);
        CHECK(app_text_action(4,"document.edit","Cannot patch","offset",0,1,"text")<0);
        CHECK(calls==before);
        CHECK(deliver_ai(revision,&event)==1);CHECK(event.kind==APP_AI);
        CHECK(app_state(1,0,"Complete replacement")==0);
        CHECK(state_packet.revision==revision+1);
        CHECK(app_choice(2,"document.edit","Edit",0,0,0,0)==0);
        CHECK(app_publish()==0);revision++;
        CHECK(deliver_ai(revision,&event)==1);
        before=calls;CHECK(app_publish()<0);CHECK(calls==before);
    }
}

// ------------------------=
// FUNC: test_events
// DESC: Verifies UI precedence, signed AI arguments, full typed-text boundaries, malformed packet rejection, and exact completion receipts.
// ------------------=
static void test_events(void) {
    reset_boundary();AppEvent event;memset(&event,0x5a,sizeof(event));
    CHECK(app_event(NULL)<0);CHECK(calls==0);
    CHECK(app_event(&event)==0);CHECK(ui_polls==1 && ai_polls==1);
    AppEvent empty;memset(&empty,0,sizeof(empty));same_bytes(&event,&empty,sizeof(event));
    ui_result=-13;unsigned before=ai_polls;CHECK(app_event(&event)==-13);CHECK(ai_polls==before);
    ui_result=1;next_ui.version=1;next_ui.kind=APP_CLICK;next_ui.id=41;
    CHECK(app_event(&event)==1);CHECK(event.kind==APP_CLICK && event.id==41 && event.token==0);
    CHECK(ai_polls==before);unsigned receipts=finishes;CHECK(app_done(&event,1)==0);CHECK(finishes==receipts);
    next_ui.kind=APP_CLOSE;CHECK(app_event(&event)==1);CHECK(event.kind==APP_CLOSE);
    next_ui.version=2;CHECK(app_event(&event)<0);same_bytes(&event,&empty,sizeof(event));
    next_ui.version=1;next_ui.kind=99;CHECK(app_event(&event)<0);
    ui_result=0;ai_result=-16;CHECK(app_event(&event)==-16);
    CHECK(deliver_ai(infinity_app_revision,&event)==1);
    CHECK(event.kind==APP_AI && event.id==12 && event.token==99);
    CHECK(event.a==-4 && event.b==8 && event.c==INT64_MAX && event.d==INT64_MIN);
    CHECK(event.text[0]==0);
    next_ai.typed=1;next_ai.text_length=512;
    for(size_t i=0;i<sizeof(next_ai.text);i++) next_ai.text[i]=(char)('a'+i%26);
    CHECK(app_event(&event)==1);same_bytes(event.text,next_ai.text,512);CHECK(event.text[512]==0);
    CHECK(app_done(&event,0)==0);CHECK(receipt_packet.token==99 && receipt_packet.outcome==INFINITY_AI_REJECTED);
    write_result=-116;CHECK(app_done(&event,1)==-116);
    CHECK(receipt_packet.outcome==INFINITY_AI_APPLIED);write_result=0;
    CHECK(app_done(&event,1)==0);CHECK(receipt_packet.outcome==INFINITY_AI_APPLIED);
    receipts=finishes;CHECK(app_done(NULL,1)==0);CHECK(finishes==receipts);

    next_ai.text_length=513;CHECK(app_event(&event)==0);CHECK(finishes==receipts+1);
    CHECK(receipt_packet.outcome==INFINITY_AI_REJECTED);same_bytes(&event,&empty,sizeof(event));
    next_ai.text_length=0;next_ai.typed=2;CHECK(app_event(&event)==0);
    next_ai.typed=1;next_ai.request.version=2;receipts=finishes;CHECK(app_event(&event)<0);CHECK(finishes==receipts);
    next_ai.request.version=1;ai_result=sizeof(next_ai)-1;CHECK(app_event(&event)<0);CHECK(finishes==receipts);
}

// ------------------------=
// FUNC: begin_loop
// DESC: Installs a finite deterministic input script; polling beyond its end fails instead of hanging the test.
// ------------------=
static void begin_loop(const int *kinds,const int *ids,unsigned length) {
    reset_boundary();CHECK(length<=8);
    memcpy(loop_kinds,kinds,length*sizeof(*kinds));memcpy(loop_ids,ids,length*sizeof(*ids));
    loop_at=loop_renders=loop_handles=loop_fail_render=loop_render_mode=loop_trace_length=0;
    loop_poll_error=loop_finish_error=0;loop_length=length;loop_enabled=1;
}

// ------------------------=
// FUNC: render_loop
// DESC: Publishes through the real SDK or deliberately fails a chosen render to exercise fail-closed loop behavior.
// ------------------=
static int render_loop(void) {
    loop_renders++;record_loop(LOOP_RENDER);
    int fail=loop_renders==loop_fail_render;
    if(fail && loop_render_mode==0) return -5;
    if(fail && loop_render_mode==1) return 0;
    int result=app_state(1,0,"Event-loop observation");
    if(result<0) return result;
    if(fail && loop_render_mode==2) return 0;
    if(fail && loop_render_mode==3) {CHECK(app_choice(0,"action.invalid","Invalid",0,0,0,0)<0);return 0;}
    result=app_choice(41,"item.change","Change",0,0,0,0);
    if(result<0) return result;
    if(fail && loop_render_mode==4) write_result=-28;
    result=app_publish();write_result=0;return result;
}

// ------------------------=
// FUNC: handle_loop
// DESC: Uses one handler for real UI and AI events with accepted, rejected and fatal outcomes.
// ------------------=
static int handle_loop(const AppEvent *event) {
    CHECK(event && (event->kind==APP_CLICK || event->kind==APP_AI));
    loop_handles++;record_loop(LOOP_HANDLE);
    if(event->id==43) return -7;
    return event->id==41?2:0;
}

// ------------------------=
// FUNC: test_event_loop
// DESC: Exercises the SDK loop's finite lifecycle, shared handling, publication-gated receipts, failure propagation and no-replay behavior.
// ------------------=
static void test_event_loop(void) {
    reset_boundary();unsigned before=calls;
    CHECK(app_run(NULL,handle_loop)<0);CHECK(app_run(render_loop,NULL)<0);CHECK(calls==before);
    const int kinds[]={0,APP_CLICK,APP_AI,APP_AI,APP_CLOSE};
    const int ids[]={0,41,41,42,0};
    begin_loop(kinds,ids,5);uint64_t revision=infinity_app_revision;
    CHECK(app_run(render_loop,handle_loop)==0);
    CHECK(loop_at==5 && yields==5 && loop_renders==3 && loop_handles==3 && finishes==2);
    CHECK(infinity_app_revision==revision+3);
    const unsigned expected[]={LOOP_RENDER,LOOP_YIELD,LOOP_HANDLE,LOOP_RENDER,LOOP_YIELD,
        LOOP_HANDLE,LOOP_RENDER,LOOP_APPLIED,LOOP_YIELD,LOOP_HANDLE,LOOP_REJECTED,LOOP_YIELD,LOOP_YIELD};
    CHECK(loop_trace_length==sizeof(expected)/sizeof(expected[0]));same_bytes(loop_trace,expected,sizeof(expected));

    const int ai_close[]={APP_AI,APP_CLOSE};const int accepted[]={41,0};
    for(unsigned mode=0;mode<5;mode++) {
        begin_loop(ai_close,accepted,2);revision=infinity_app_revision;
        loop_fail_render=2;loop_render_mode=mode;
        int result=app_run(render_loop,handle_loop);
        CHECK(result==(mode==0?-5:mode==4?-28:-1));
        CHECK(loop_at==1 && yields==1 && loop_handles==1 && loop_renders==2 && finishes==1);
        CHECK(receipt_packet.outcome==INFINITY_AI_REJECTED && infinity_app_revision==revision+1);
        const unsigned failed[]={LOOP_RENDER,LOOP_HANDLE,LOOP_RENDER,LOOP_REJECTED,LOOP_YIELD};
        CHECK(loop_trace_length==sizeof(failed)/sizeof(failed[0]));same_bytes(loop_trace,failed,sizeof(failed));
    }
    begin_loop(ai_close,accepted,2);loop_fail_render=1;
    CHECK(app_run(render_loop,handle_loop)==-5);CHECK(yields==0 && loop_handles==0 && finishes==0);
    begin_loop(ai_close,accepted,2);loop_fail_render=1;loop_render_mode=1;
    CHECK(app_run(render_loop,handle_loop)<0);CHECK(yields==0 && loop_handles==0 && finishes==0);

    begin_loop(ai_close,accepted,2);loop_finish_error=-116;
    CHECK(app_run(render_loop,handle_loop)==-116);
    CHECK(loop_at==1 && yields==1 && loop_handles==1 && loop_renders==2 && finishes==1);
    begin_loop(ai_close,accepted,2);loop_poll_error=-13;
    CHECK(app_run(render_loop,handle_loop)==-13);
    CHECK(loop_at==1 && yields==1 && loop_handles==0 && loop_renders==1 && finishes==0);
    const int rejected[]={43,0};begin_loop(ai_close,rejected,2);
    CHECK(app_run(render_loop,handle_loop)==-7);
    CHECK(loop_at==1 && yields==1 && loop_handles==1 && loop_renders==1 && finishes==1);
    CHECK(receipt_packet.outcome==INFINITY_AI_REJECTED);
    const int click_close[]={APP_CLICK,APP_CLOSE};begin_loop(click_close,accepted,2);loop_fail_render=2;
    CHECK(app_run(render_loop,handle_loop)==-5);
    CHECK(loop_at==1 && yields==1 && loop_handles==1 && loop_renders==2 && finishes==0);
    loop_enabled=0;
}

// ------------------------=
// FUNC: test_grid_helpers
// DESC: Exercises native geometry, stable IDs, capacity, publication boundaries and error recovery for generic grid helpers.
// ------------------=
static void test_grid_helpers(void) {
    reset_boundary();
    CHECK(app_window("Manual",320,240)==0);
    unsigned before=calls;
    CHECK(app_cell(1,0,0,"A")<0 && app_status("Ready")<0 && app_offer(1,"A")<0);
    CHECK(calls==before);
    const int invalid[][2]={{0,1},{1,0},{10,1},{1,13},{8,12},{9,11},{INT32_MAX,INT32_MAX},{-1,2}};
    for(unsigned i=0;i<sizeof(invalid)/sizeof(invalid[0]);i++)
        CHECK(app_grid("Grid",invalid[i][0],invalid[i][1])<0);
    CHECK(calls==before);
    for(int rows=1;rows<=9;rows++) for(int columns=1;columns<=12;columns++) {
        if(rows*columns>95) continue;
        CHECK(app_grid("Grid",rows,columns)==0);
        CHECK(window_packet.width==(uint32_t)(68*columns+28<320?320:68*columns+28));
        CHECK(window_packet.height==(uint32_t)(68*rows+72<240?240:68*rows+72));
        CHECK(window_packet.width<=960 && window_packet.height<=720);
        CHECK(app_status("Ready")==0);
        CHECK(element_packet.id==INT32_MAX && element_packet.kind==INFINITY_UI_LABEL);
        CHECK(element_packet.x==20 && element_packet.y==20 && element_packet.height==32);
        CHECK(element_packet.width==window_packet.width-40);
        for(int row=0;row<rows;row++) for(int column=0;column<columns;column++) {
            int id=1+row*columns+column;
            CHECK(app_cell(id,row,column,"Cell")==0);
            CHECK(element_packet.id==(uint32_t)id && element_packet.kind==INFINITY_UI_BUTTON);
            CHECK(element_packet.x==(uint32_t)(20+68*column) && element_packet.y==(uint32_t)(64+68*row));
            CHECK(element_packet.width==56 && element_packet.height==56);
            CHECK(element_packet.x+element_packet.width+20<=window_packet.width);
            CHECK(element_packet.y+element_packet.height+20<=window_packet.height);
        }
    }
    CHECK(app_grid("Tall grid",9,8)==0);
    CHECK(window_packet.width==572 && window_packet.height==684);
    CHECK(app_cell(72,8,7,"Last cell")==0);
    CHECK(element_packet.y+element_packet.height+20==window_packet.height);
    before=calls;CHECK(app_cell(73,9,0,"Outside")<0 && calls==before);
    CHECK(app_grid("Grid",2,2)==0 && app_cell(17,0,0,"A")==0);
    CHECK(app_cell(17,0,0,"Updated")==0);
    before=calls;
    CHECK(app_cell(0,0,1,"A")<0 && app_cell(-1,0,1,"A")<0 && app_cell(INT32_MAX,0,1,"A")<0);
    CHECK(app_cell(17,0,1,"A")<0 && app_cell(18,0,0,"A")<0);
    CHECK(app_cell(18,-1,0,"A")<0 && app_cell(18,0,-1,"A")<0);
    CHECK(app_cell(18,2,0,"A")<0 && app_cell(18,0,2,"A")<0);
    CHECK(calls==before);
    CHECK(app_state(1,0,"Grid state")==0 && app_offer(17,"Activate A")==0);
    CHECK(choice_packet.id==17 && choice_packet.risk==INFINITY_AI_REVERSIBLE);
    CHECK(memcmp(choice_packet.action,"app.activate",13)==0);
    for(int i=0;i<4;i++) CHECK(choice_packet.arguments[i]==0);
    CHECK(app_publish()==0);
    uint64_t revision=infinity_app_revision;
    CHECK(app_state(1,0,"Grid state")==0);
    before=calls;
    CHECK(app_offer(18,"Missing")<0 && app_publish()<0 && calls==before);
    CHECK(infinity_app_revision==revision);
    CHECK(app_state(1,0,"Grid state")==0);
    write_result=-5;CHECK(app_cell(18,0,1,"B")==-5);write_result=0;
    CHECK(app_publish()<0 && infinity_app_revision==revision);
    CHECK(app_state(1,0,"Grid state")==0);
    before=calls;CHECK(app_offer(18,"Never created")<0 && calls==before);
    CHECK(app_state(1,0,"Grid state")==0 && app_cell(18,0,1,"B")==0);
    CHECK(app_offer(18,"Activate B")==0 && app_publish()==0);
    CHECK(app_state(1,0,"Grid state")==0);
    char oversized[33];memset(oversized,'x',sizeof(oversized));oversized[32]=0;
    before=calls;CHECK(app_status(oversized)<0 && app_publish()<0 && calls==before);
    CHECK(app_state(1,0,"Grid state")==0);
    write_result=-6;CHECK(app_window("Failed",320,240)==-6);write_result=0;
    CHECK(app_offer(17,"Still exists")==0 && app_publish()==0);
    CHECK(app_grid("Reset",1,1)==0 && app_state(1,0,"Grid state")==0);
    before=calls;CHECK(app_offer(17,"Old grid")<0 && calls==before);
    CHECK(app_cell(17,0,0,"Reused")==0 && app_state(1,0,"Grid state")==0);
    CHECK(app_offer(17,"New grid")==0 && app_publish()==0);
    CHECK(app_window("Manual",320,240)==0 && app_state(0,0,"Manual state")==0);
    before=calls;CHECK(app_offer(17,"Stale cell")<0 && app_publish()<0 && calls==before);
    CHECK(app_state(0,0,"Manual state")==0 && app_publish()==0);
}

// ------------------------=
// FUNC: main
// DESC: Executes real SDK helpers against deterministic native service results and exits nonzero on any behavioral failure.
// ------------------=
int main(void) {
    test_ui_packets();test_publication();test_state_status_not_data();test_choices();test_incomplete_contracts();test_events();test_event_loop();test_grid_helpers();return 0;
}
