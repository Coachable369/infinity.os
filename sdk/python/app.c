#define PY_SSIZE_T_CLEAN
#include "Python.h"
#include <infinity/app.h>

// ------------------------=
// FUNC: checked
// DESC: Preserves native authorization and contract failures as Python exceptions.
// ------------------=
static PyObject *checked(int value) {
    if(value>=0) return PyLong_FromLong(value);
    PyErr_Format(PyExc_OSError,"Native application operation rejected (%d)",value);
    return NULL;
}
// ------------------------=
// FUNC: window
// DESC: Opens the native themed application window with SDK bounds and session ownership.
// ------------------=
static PyObject *window(PyObject *self,PyObject *args) {
    (void)self;const char *title;int width,height;
    if(!PyArg_ParseTuple(args,"sii",&title,&width,&height))return NULL;
    return checked(app_window(title,width,height));
}
// ------------------------=
// FUNC: element
// DESC: Creates a real UIKit label, display or button with a stable application ID.
// ------------------=
static PyObject *element(PyObject *self,PyObject *args) {
    (void)self;const char *label;int kind,id,x,y,w,h;
    if(!PyArg_ParseTuple(args,"iiiiiis",&kind,&id,&x,&y,&w,&h,&label))return NULL;
    if(kind<1||kind>3){PyErr_SetString(PyExc_ValueError,"Unsupported UIKit element kind");return NULL;}
    return checked(infinity_app_element(id,kind,x,y,w,h,label));
}
// ------------------------=
// FUNC: state
// DESC: Stages an observable app state without granting autonomous AI execution.
// ------------------=
static PyObject *state(PyObject *self,PyObject *args) {
    (void)self;int ai_turn,terminal;const char *text;
    if(!PyArg_ParseTuple(args,"pps",&ai_turn,&terminal,&text))return NULL;
    return checked(app_state(ai_turn,terminal,text));
}
// ------------------------=
// FUNC: choice
// DESC: Publishes a reversible legal action which still requires native capability authorization on execution.
// ------------------=
static PyObject *choice(PyObject *self,PyObject *args) {
    (void)self;int id,a=0,b=0,c=0,d=0;const char *action,*label;
    if(!PyArg_ParseTuple(args,"iss|iiii",&id,&action,&label,&a,&b,&c,&d))return NULL;
    return checked(app_choice(id,action,label,a,b,c,d));
}
// ------------------------=
// FUNC: text_action
// DESC: Offers the same bounded typed text-edit action contract as native C apps.
// ------------------=
static PyObject *text_action(PyObject *self,PyObject *args) {
    (void)self;int id,minimum,maximum;const char *action,*label,*parameter,*text_name;
    if(!PyArg_ParseTuple(args,"isssiis",&id,&action,&label,&parameter,&minimum,&maximum,&text_name))return NULL;
    return checked(app_text_action(id,action,label,parameter,minimum,maximum,text_name));
}
// ------------------------=
// FUNC: publish
// DESC: Commits one coherent state and action revision to the existing discoverable application registry.
// ------------------=
static PyObject *publish(PyObject *self,PyObject *unused) {
    (void)self;(void)unused;return checked(app_publish());
}
// ------------------------=
// FUNC: event
// DESC: Delivers real pointer or authorized AI events using the shared checked native event ABI.
// ------------------=
static PyObject *event(PyObject *self,PyObject *unused) {
    (void)self;(void)unused;AppEvent e;int value=app_event(&e);
    if(value<0)return checked(value);if(!value)Py_RETURN_NONE;
    return Py_BuildValue("{s:i,s:i,s:L,s:L,s:L,s:L,s:K,s:s}","kind",e.kind,"id",e.id,
        "a",(long long)e.a,"b",(long long)e.b,"c",(long long)e.c,"d",(long long)e.d,
        "token",(unsigned long long)e.token,"text",e.text);
}
// ------------------------=
// FUNC: done
// DESC: Acknowledges an actual AI receipt only after the caller has published its updated state.
// ------------------=
static PyObject *done(PyObject *self,PyObject *args) {
    (void)self;unsigned long long token;int applied;
    if(!PyArg_ParseTuple(args,"Kp",&token,&applied))return NULL;
    AppEvent e={0};e.kind=APP_AI;e.token=token;return checked(app_done(&e,applied));
}
// ------------------------=
// FUNC: yield_native
// DESC: Yields to the existing native scheduler without blocking the voice or desktop event loop.
// ------------------=
static PyObject *yield_native(PyObject *self,PyObject *unused) {
    (void)self;(void)unused;app_yield();Py_RETURN_NONE;
}
static PyMethodDef methods[]={
    {"window",window,METH_VARARGS,NULL},{"element",element,METH_VARARGS,NULL},
    {"state",state,METH_VARARGS,NULL},{"choice",choice,METH_VARARGS,NULL},
    {"text_action",text_action,METH_VARARGS,NULL},{"publish",publish,METH_NOARGS,NULL},
    {"event",event,METH_NOARGS,NULL},{"done",done,METH_VARARGS,NULL},
    {"yield_native",yield_native,METH_NOARGS,NULL},{NULL,NULL,0,NULL}
};
static struct PyModuleDef module={PyModuleDef_HEAD_INIT,"_infinity_app",NULL,-1,methods};
// ------------------------=
// FUNC: PyInit__infinity_app
// DESC: Exposes genuine UIKit and application interaction hooks without a second execution framework.
// ------------------=
PyMODINIT_FUNC PyInit__infinity_app(void){return PyModule_Create(&module);}
