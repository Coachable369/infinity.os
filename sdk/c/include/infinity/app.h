#ifndef INFINITY_APP_H
#define INFINITY_APP_H
#include <stddef.h>
#include <stdint.h>
#define INFINITY_APP_ABI 2u
typedef struct InfinityObjectApi {
    int (*open)(void *, const unsigned char *, size_t, uint32_t);
    ptrdiff_t (*read)(void *, int, unsigned char *, size_t);
    ptrdiff_t (*write)(void *, int, const unsigned char *, size_t);
    int64_t (*seek)(void *, int, int64_t, uint32_t);
    int (*flush)(void *, int);
    int (*close)(void *, int);
} InfinityObjectApi;
typedef struct InfinityAppApi {
    uint32_t version;
    uint32_t size;
    void *context;
    int (*write_stdout)(void *context, const unsigned char *bytes, size_t length);
    const InfinityObjectApi *objects;
} InfinityAppApi;
#include <string.h>
#include <infinity/uikit.h>
#include <infinity/interaction.h>
#include <infinity/app_api.h>
static uint64_t infinity_app_revision = 0;
static uint64_t infinity_app_staged_revision = 0;
static int infinity_app_staging_error = 0;
static int infinity_app_grid_rows, infinity_app_grid_columns, infinity_app_grid_width;
static int infinity_app_grid_cells[95];

// ------------------------=
// FUNC: infinity_app_stage_result
// DESC: Keeps an incomplete action contract unpublishable until a successful new observation restarts staging.
// ------------------=
static inline int infinity_app_stage_result(int result) {
    if(result<0 && infinity_app_staged_revision) infinity_app_staging_error=1;
    return result;
}

// ------------------------=
// FUNC: infinity_app_copy
// DESC: Copies a complete bounded field without silently truncating an app contract.
// ------------------=
static inline int infinity_app_copy(char *out, size_t capacity, const char *text) {
    if (!out || !text || !capacity) return -1;
    size_t length = strlen(text);
    if (length >= capacity) return -1;
    memcpy(out, text, length + 1);
    return (int)length;
}
// ------------------------=
// FUNC: infinity_app_identifier
// DESC: Matches the native registry's alphabetic-first dotted semantic identifier contract.
// ------------------=
static inline int infinity_app_identifier(const char *text) {
    if (!text || !((*text >= 'A' && *text <= 'Z') || (*text >= 'a' && *text <= 'z'))) return 0;
    for (; *text; ++text) {
        if (!((*text >= 'A' && *text <= 'Z') || (*text >= 'a' && *text <= 'z') ||
            (*text >= '0' && *text <= '9') || *text == '.' || *text == '_')) return 0;
    }
    return 1;
}
// ------------------------=
// FUNC: app_window
// DESC: Opens the existing themed UIKit window with checked dimensions and title.
// ------------------=
static inline int app_window(const char *title, int width, int height) {
    InfinityUiWindow window = {0};
    int length = infinity_app_copy(window.title, sizeof(window.title), title);
    if (length < 0 || width < 320 || width > 960 || height < 240 || height > 720) return -1;
    window.version=1; window.width=width; window.height=height; window.title_length=length;
    int result=(int)infinity_ui_open(&window);
    if(result>=0) {
        infinity_app_grid_rows=infinity_app_grid_columns=infinity_app_grid_width=0;
        memset(infinity_app_grid_cells,0,sizeof(infinity_app_grid_cells));
    }
    return result;
}
// ------------------------=
// FUNC: infinity_app_element
// DESC: Updates one existing UIKit element with a stable ID and bounded label.
// ------------------=
static inline int infinity_app_element(int id,int kind,int x,int y,int w,int h,const char *label) {
    InfinityUiElement element={0};
    int length=infinity_app_copy(element.label,sizeof(element.label),label);
    if(length<0 || id<=0 || x<0 || y<0 || w<=0 || h<=0) return -1;
    element.version=1; element.id=id; element.kind=kind;
    element.x=x; element.y=y; element.width=w; element.height=h; element.label_length=length;
    return (int)infinity_ui_upsert(&element);
}
// ------------------------=
// FUNC: app_button
// DESC: Uses the shared theme-aware native button recipe, including pointer and focus states.
// ------------------=
static inline int app_button(int id,int x,int y,int w,int h,const char *label) {
    return infinity_app_element(id,INFINITY_UI_BUTTON,x,y,w,h,label);
}
// ------------------------=
// FUNC: app_label
// DESC: Uses the shared native label and typography roles.
// ------------------=
static inline int app_label(int id,int x,int y,int w,int h,const char *label) {
    return infinity_app_element(id,INFINITY_UI_LABEL,x,y,w,h,label);
}
// ------------------------=
// FUNC: app_display
// DESC: Uses the existing inset UIKit display surface.
// ------------------=
static inline int app_display(int id,int x,int y,int w,int h,const char *label) {
    return infinity_app_element(id,INFINITY_UI_DISPLAY,x,y,w,h,label);
}
// ------------------------=
// FUNC: app_grid
// DESC: Opens a bounded themed window with reserved status and button geometry using the shared native kit spacing.
// ------------------=
static inline int app_grid(const char *title,int rows,int columns) {
    if(rows<1 || rows>9 || columns<1 || columns>12 || rows*columns>95)
        return infinity_app_stage_result(-1);
    int width=68*columns+28, height=68*rows+72;
    if(width<320) width=320;
    if(height<240) height=240;
    int result=app_window(title,width,height);
    if(result>=0) {
        infinity_app_grid_rows=rows;infinity_app_grid_columns=columns;infinity_app_grid_width=width;
    }
    return infinity_app_stage_result(result);
}
// ------------------------=
// FUNC: app_cell
// DESC: Upserts one themed grid button with a stable positive cell ID, registering it only after native success.
// ------------------=
static inline int app_cell(int id,int row,int column,const char *label) {
    if(id<=0 || id==INT32_MAX || row<0 || row>=infinity_app_grid_rows ||
        column<0 || column>=infinity_app_grid_columns) return infinity_app_stage_result(-1);
    int index=row*infinity_app_grid_columns+column;
    if(infinity_app_grid_cells[index] && infinity_app_grid_cells[index]!=id)
        return infinity_app_stage_result(-1);
    for(int i=0;i<infinity_app_grid_rows*infinity_app_grid_columns;i++)
        if(i!=index && infinity_app_grid_cells[i]==id) return infinity_app_stage_result(-1);
    int result=app_button(id,20+68*column,64+68*row,56,56,label);
    if(result>=0) infinity_app_grid_cells[index]=id;
    return infinity_app_stage_result(result);
}
// ------------------------=
// FUNC: app_status
// DESC: Updates the grid's reserved native status label without consuming a caller-assigned cell ID.
// ------------------=
static inline int app_status(const char *text) {
    if(!infinity_app_grid_rows) return infinity_app_stage_result(-1);
    return infinity_app_stage_result(app_label(INT32_MAX,20,20,infinity_app_grid_width-40,32,text));
}
// ------------------------=
// FUNC: app_state
// DESC: Stages a new revision through the existing authority-checked interaction ABI, without autonomous AI authority.
// ------------------=
static inline int app_state(int ai_turn,int terminal,const char *text) {
    InfinityAiState state={0};
    int length=infinity_app_copy(state.text,sizeof(state.text),text);
    if(length<=0 || infinity_app_revision==UINT64_MAX) return infinity_app_stage_result(-1);
    state.version=1; state.revision=infinity_app_revision+1; state.length=length;
    state.flags=(ai_turn?INFINITY_AI_TURN:0)|(terminal?INFINITY_AI_TERMINAL:0);
    int result=(int)infinity_ai_begin(&state);
    if(result>=0) {infinity_app_staged_revision=state.revision;infinity_app_staging_error=0;}
    return infinity_app_stage_result(result);
}
// ------------------------=
// FUNC: app_choice
// DESC: Advertises one reversible legal action; apps still validate the shared operation on delivery.
// ------------------=
static inline int app_choice(int id,const char *action,const char *label,int a,int b,int c,int d) {
    InfinityAiChoice choice={0};
    if(!infinity_app_staged_revision || infinity_app_staging_error) return -1;
    if(id<=0 || !infinity_app_identifier(action) ||
        infinity_app_copy(choice.action,sizeof(choice.action),action)<0 ||
        infinity_app_copy(choice.label,sizeof(choice.label),label)<=0) return infinity_app_stage_result(-1);
    choice.version=1; choice.id=id; choice.risk=INFINITY_AI_REVERSIBLE;
    choice.arguments[0]=a; choice.arguments[1]=b; choice.arguments[2]=c; choice.arguments[3]=d;
    return infinity_app_stage_result((int)infinity_ai_offer(&choice));
}
// ------------------------=
// FUNC: app_offer
// DESC: Offers the shared reversible activation action only for a successfully created grid cell; app code still decides legality.
// ------------------=
static inline int app_offer(int id,const char *label) {
    if(id<=0 || id==INT32_MAX) return infinity_app_stage_result(-1);
    for(int i=0;i<infinity_app_grid_rows*infinity_app_grid_columns;i++)
        if(infinity_app_grid_cells[i]==id) return app_choice(id,"app.activate",label,0,0,0,0);
    return infinity_app_stage_result(-1);
}
// ------------------------=
// FUNC: app_text_action
// DESC: Publishes one bounded reversible text edit plus an integer parameter through the native typed-action schema.
// ------------------=
static inline int app_text_action(int id,const char *action,const char *label,const char *parameter,int minimum,int maximum,const char *text_name) {
    InfinityAiAction proposal={0};
    if(!infinity_app_staged_revision || infinity_app_staging_error) return -1;
    if(id<=0 || minimum>maximum || !infinity_app_identifier(action) ||
        !infinity_app_identifier(parameter) || !infinity_app_identifier(text_name) ||
        strcmp(parameter,text_name)==0 || infinity_app_copy(proposal.choice.action,48,action)<0 ||
        infinity_app_copy(proposal.choice.label,96,label)<=0 || infinity_app_copy(proposal.parameters[0].name,24,parameter)<0 ||
        infinity_app_copy(proposal.text_name,24,text_name)<0) return infinity_app_stage_result(-1);
    proposal.choice.version=1; proposal.choice.id=id; proposal.choice.risk=INFINITY_AI_REVERSIBLE;
    proposal.choice.arguments[0]=minimum; proposal.parameters[0].kind=INFINITY_AI_INTEGER;
    proposal.parameters[0].minimum=minimum; proposal.parameters[0].maximum=maximum;
    for(int i=1;i<4;i++) {
        for(int candidate=0;candidate<5;candidate++) {
            char name[]="unused0";name[6]=(char)('0'+candidate);
            int distinct=strcmp(name,parameter)!=0 && strcmp(name,text_name)!=0;
            for(int j=1;j<i;j++) if(strcmp(name,proposal.parameters[j].name)==0) distinct=0;
            if(distinct) {memcpy(proposal.parameters[i].name,name,sizeof(name));break;}
        }
    }
    proposal.text_limit=512;
    return infinity_app_stage_result((int)infinity_ai_action(&proposal));
}
// ------------------------=
// FUNC: app_publish
// DESC: Atomically commits the observation and legal actions using the existing registry.
// ------------------=
static inline int app_publish(void) {
    if(!infinity_app_staged_revision || infinity_app_staging_error) return -1;
    int result=(int)infinity_ai_commit();
    if(result>=0) {infinity_app_revision=infinity_app_staged_revision;infinity_app_staged_revision=0;}
    return result;
}
// ------------------------=
// FUNC: app_event
// DESC: Delivers native UI or authorized AI input without blocking, rejecting stale AI revisions before application dispatch.
// ------------------=
static inline int app_event(AppEvent *event) {
    if(!event) return -1;
    memset(event,0,sizeof(*event));
    InfinityUiEvent ui={0}; ui.version=1;
    int result=(int)infinity_ui_poll(&ui);
    if(result<0) return result;
    if(result>0) {
        if(result!=1 || ui.version!=1 || (ui.kind!=APP_CLICK && ui.kind!=APP_CLOSE)) return -1;
        event->kind=ui.kind;event->id=ui.id;return 1;
    }
    InfinityAiTypedRequest ai={0};
    result=(int)infinity_ai_poll_typed(&ai);
    if(result<=0) return result;
    if(result!=(int)sizeof(ai) || ai.request.version!=1) return -1;
    if(ai.request.revision!=infinity_app_revision || ai.text_length>sizeof(ai.text) || ai.typed>1) {
        infinity_ai_finish(ai.request.token,INFINITY_AI_REJECTED);return 0;
    }
    event->kind=APP_AI;event->id=ai.request.choice;event->token=ai.request.token;
    event->a=ai.request.arguments[0];event->b=ai.request.arguments[1];
    event->c=ai.request.arguments[2];event->d=ai.request.arguments[3];
    memcpy(event->text,ai.text,ai.text_length);event->text[ai.text_length]=0;
    return 1;
}
// ------------------------=
// FUNC: app_done
// DESC: Acknowledges a native AI receipt; the kernel independently requires a newer committed revision for success.
// ------------------=
static inline int app_done(const AppEvent *event,int applied) {
    if(!event || event->kind!=APP_AI) return 0;
    return (int)infinity_ai_finish(event->token,applied?INFINITY_AI_APPLIED:INFINITY_AI_REJECTED);
}
// ------------------------=
// FUNC: app_yield
// DESC: Cooperatively yields through the existing native application scheduler.
// ------------------=
static inline void app_yield(void) {infinity_ui_yield();}
// ------------------------=
// FUNC: app_run
// DESC: Shares validated UI and AI handling, commits each rendered change before its receipt, and yields without retrying failed operations.
// ------------------=
static inline int app_run(int (*render)(void), int (*handle)(const AppEvent *)) {
    if(!render || !handle) return -1;
    uint64_t revision=infinity_app_revision;
    int result=render();
    if(result<0 || infinity_app_revision<=revision || infinity_app_staged_revision || infinity_app_staging_error)
        return result<0?result:-1;
    for(;;) {
        AppEvent event;
        result=app_event(&event);
        if(result<0) {app_yield();return result;}
        if(!result) {app_yield();continue;}
        if(event.kind==APP_CLOSE) {app_yield();return 0;}
        int accepted=handle(&event);
        if(accepted>0) {
            revision=infinity_app_revision;
            result=render();
            if(result<0 || infinity_app_revision<=revision || infinity_app_staged_revision || infinity_app_staging_error) {
                (void)app_done(&event,0);
                app_yield();return result<0?result:-1;
            }
            result=app_done(&event,1);
        } else {
            result=app_done(&event,0);
            if(result>=0 && accepted<0) result=accepted;
        }
        app_yield();
        if(result<0) return result;
    }
}
#endif
