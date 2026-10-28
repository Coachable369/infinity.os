#define PY_SSIZE_T_CLEAN
#include "Python.h"
#include <stdint.h>
#include <infinity/syscall.h>

// ------------------------=
// FUNC: result
// DESC: Converts native child admission failures without hiding security denial or missing execution support.
// ------------------=
static PyObject *result(int64_t value) {
    if (value >= 0) return PyLong_FromLongLong(value);
    PyObject *args = Py_BuildValue("(is)", (int)-value, "Native build-child operation failed");
    if (args) { PyErr_SetObject(PyExc_OSError, args); Py_DECREF(args); }
    return NULL;
}
// ------------------------=
// FUNC: start
// DESC: Starts a real bounded native Python child from a private guest launch record.
// ------------------=
static PyObject *start(PyObject *self, PyObject *args) {
    (void)self; const char *path; Py_ssize_t length;
    if (!PyArg_ParseTuple(args, "y#", &path, &length)) return NULL;
    return result(infinity_system_call(0x1280, (uintptr_t)path, length, 0, 0, 0, 0));
}
// ------------------------=
// FUNC: poll_child
// DESC: Reads a typed real-exit and captured-length receipt after the child has reclaimed its native process memory.
// ------------------=
static PyObject *poll_child(PyObject *self, PyObject *args) {
    (void)self; unsigned long long id; uint64_t receipt[3];
    if (!PyArg_ParseTuple(args, "K", &id)) return NULL;
    int64_t value = infinity_system_call(0x1281, id, (uintptr_t)receipt, 0, 0, 0, 0);
    if (value == -11) Py_RETURN_NONE;
    if (value < 0) return result(value);
    return Py_BuildValue("(LKK)", (long long)receipt[0],
        (unsigned long long)receipt[1], (unsigned long long)receipt[2]);
}
// ------------------------=
// FUNC: read_child
// DESC: Reads a bounded captured stdout or stderr range through checked process memory.
// ------------------=
static PyObject *read_child(PyObject *self, PyObject *args) {
    (void)self; unsigned long long id, stream, offset; Py_ssize_t length;
    if (!PyArg_ParseTuple(args, "KKKn", &id, &stream, &offset, &length)) return NULL;
    if (length < 0 || length > 65536) { PyErr_SetString(PyExc_ValueError, "Child output read exceeds limit"); return NULL; }
    PyObject *bytes = PyBytes_FromStringAndSize(NULL, length);
    if (!bytes) return NULL;
    int64_t value = infinity_system_call(0x1282, id, stream, offset,
        (uintptr_t)PyBytes_AS_STRING(bytes), length, 0);
    if (value < 0) { Py_DECREF(bytes); return result(value); }
    if (_PyBytes_Resize(&bytes, value) < 0) return NULL;
    return bytes;
}
// ------------------------=
// FUNC: cancel_child
// DESC: Requests cancellation of the exact owned native child.
// ------------------=
static PyObject *cancel_child(PyObject *self, PyObject *args) {
    (void)self; unsigned long long id;
    if (!PyArg_ParseTuple(args, "K", &id)) return NULL;
    return result(infinity_system_call(0x1283, id, 0, 0, 0, 0, 0));
}
// ------------------------=
// FUNC: release_child
// DESC: Acknowledges a completed receipt and erases private child output before another child may start.
// ------------------=
static PyObject *release_child(PyObject *self, PyObject *args) {
    (void)self; unsigned long long id;
    if (!PyArg_ParseTuple(args, "K", &id)) return NULL;
    return result(infinity_system_call(0x1283, id, 1, 0, 0, 0, 0));
}
static PyMethodDef methods[] = {
    {"start", start, METH_VARARGS, NULL}, {"poll", poll_child, METH_VARARGS, NULL},
    {"read", read_child, METH_VARARGS, NULL}, {"cancel", cancel_child, METH_VARARGS, NULL},
    {"release", release_child, METH_VARARGS, NULL}, {NULL, NULL, 0, NULL}
};
static struct PyModuleDef module = {PyModuleDef_HEAD_INIT, "_infinity_process", NULL, -1, methods};
// ------------------------=
// FUNC: PyInit__infinity_process
// DESC: Exposes the native child lifecycle without host execution, shell authority or recursive process creation.
// ------------------=
PyMODINIT_FUNC PyInit__infinity_process(void) { return PyModule_Create(&module); }
