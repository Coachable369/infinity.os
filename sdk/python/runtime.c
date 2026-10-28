#define PY_SSIZE_T_CLEAN
#include "Python.h"
#include "frameobject.h"
#include <stdint.h>
#include <string.h>
#include <infinity/syscall.h>

// ------------------------=
// FUNC: word
// DESC: Encodes one stable little-endian diagnostic field independently of native structure padding.
// ------------------=
static void word(unsigned char *record, unsigned index, uint32_t value) {
    for (unsigned i = 0; i < 4; ++i) record[index * 4 + i] = (unsigned char)(value >> (i * 8));
}

// ------------------------=
// FUNC: capture_exception
// DESC: Extracts genuine exception type and granted-source traceback location without evaluating user attributes or parsing traceback text.
// ------------------=
static PyObject *capture_exception(PyObject *self, PyObject *args) {
    (void)self;
    PyObject *path, *exception;
    if (!PyArg_ParseTuple(args, "UO", &path, &exception)) return NULL;
    if (!PyExceptionInstance_Check(exception)) {
        PyErr_SetString(PyExc_TypeError, "An exception instance is required"); return NULL;
    }
    if (PyErr_GivenExceptionMatches((PyObject *)Py_TYPE(exception), PyExc_SystemExit)
            || PyErr_GivenExceptionMatches((PyObject *)Py_TYPE(exception), PyExc_KeyboardInterrupt))
        Py_RETURN_NONE;
    Py_ssize_t path_length;
    const char *path_bytes = PyUnicode_AsUTF8AndSize(path, &path_length);
    if (!path_bytes) return NULL;
    if (path_length < 1 || path_length > 256 || path_bytes[0] != '/'
            || memchr(path_bytes, 0, (size_t)path_length)) Py_RETURN_NONE;
    uint32_t category = 4, kind = 14, line = 0;
    PyObject *types[] = {PyExc_ValueError, PyExc_TypeError, PyExc_IndexError, PyExc_KeyError,
        PyExc_AttributeError, PyExc_NameError, PyExc_ZeroDivisionError, PyExc_AssertionError,
        PyExc_RuntimeError, PyExc_SyntaxError, PyExc_OSError, PyExc_MemoryError, PyExc_RecursionError};
    /* Specific resource subclasses precede their general RuntimeError base. */
    if (PyErr_GivenExceptionMatches((PyObject *)Py_TYPE(exception), PyExc_RecursionError)) { category = 3; kind = 13; }
    else {
        for (unsigned i = 0; i < sizeof types / sizeof types[0]; ++i) {
            if (PyErr_GivenExceptionMatches((PyObject *)Py_TYPE(exception), types[i])) {
                kind = i + 1; category = kind <= 10 ? 1 : kind == 11 ? 2 : 3; break;
            }
        }
    }
    PyObject *trace = PyException_GetTraceback(exception);
    for (PyTracebackObject *cursor = (PyTracebackObject *)trace; cursor; cursor = cursor->tb_next) {
        PyCodeObject *code = PyFrame_GetCode(cursor->tb_frame);
        if (!code) { Py_XDECREF(trace); return NULL; }
        int same = PyUnicode_Compare(code->co_filename, path);
        if (same == -1 && PyErr_Occurred()) { Py_DECREF(code); Py_XDECREF(trace); return NULL; }
        if (same == 0) {
            int location = cursor->tb_lineno;
            if (location < 0) location = PyCode_Addr2Line(code, cursor->tb_lasti);
            if (location > 0) line = (uint32_t)location;
        }
        Py_DECREF(code);
    }
    Py_XDECREF(trace);
    if (!line) Py_RETURN_NONE;
    const char *name = Py_TYPE(exception)->tp_name;
    size_t name_length = strlen(name);
    if (!name_length || name_length > 64) Py_RETURN_NONE;
    for (size_t i = 0; i < name_length; ++i) {
        unsigned char c = (unsigned char)name[i];
        if (!((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z')
                || (c >= '0' && c <= '9') || c == '_' || c == '.')) Py_RETURN_NONE;
    }
    unsigned char record[352] = {0};
    word(record, 0, 0x49505845); word(record, 1, 1);
    word(record, 2, category); word(record, 3, kind); word(record, 4, line);
    word(record, 5, (uint32_t)path_length); word(record, 6, (uint32_t)name_length);
    memcpy(record + 32, path_bytes, (size_t)path_length);
    memcpy(record + 288, name, name_length);
    (void)infinity_system_call(0x1290, (uintptr_t)record, sizeof record, 0, 0, 0, 0);
    Py_RETURN_NONE;
}

static PyMethodDef methods[] = {
    {"capture_exception", capture_exception, METH_VARARGS, NULL}, {NULL, NULL, 0, NULL}
};
static struct PyModuleDef module = {PyModuleDef_HEAD_INIT, "_infinity_runtime", NULL, -1, methods};
// ------------------------=
// FUNC: PyInit__infinity_runtime
// DESC: Exposes bounded diagnostic transport only; the native job still owns phase, source identity and repair authorization.
// ------------------=
PyMODINIT_FUNC PyInit__infinity_runtime(void) { return PyModule_Create(&module); }
