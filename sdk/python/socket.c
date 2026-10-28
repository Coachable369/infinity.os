#define PY_SSIZE_T_CLEAN
#include "Python.h"
#include <stdint.h>
#include <infinity/syscall.h>
#include "network_errno.h"

// ------------------------=
// FUNC: result
// DESC: Preserves native permission, certificate and transport errors as real Python OSError errno values.
// ------------------=
static PyObject *result(PyObject *module, int64_t value) {
    if (value >= 0) return PyLong_FromLongLong(value);
    if (value == -129) {
        PyObject *error = PyObject_GetAttrString(module, "CertificateError");
        if (error) { PyErr_SetString(error, "Native TLS certificate verification failed"); Py_DECREF(error); }
        return NULL;
    }
    PyObject *args = Py_BuildValue("(is)", infinity_network_errno(value), "Native socket operation failed");
    if (args) { PyErr_SetObject(PyExc_OSError, args); Py_DECREF(args); }
    return NULL;
}
// ------------------------=
// FUNC: begin
// DESC: Opens a bounded native DNS query or TCP conversation under this invocation's explicit network grant.
// ------------------=
static PyObject *begin(PyObject *self, PyObject *args) {
    (void)self; int mode, port; const char *host; Py_ssize_t length;
    if (!PyArg_ParseTuple(args, "iy#i", &mode, &host, &length, &port)) return NULL;
    return result(self, infinity_system_call(0x1270, mode, (uintptr_t)host, length, port, 0, 0));
}
// ------------------------=
// FUNC: command
// DESC: Copies one read, write or TLS-upgrade request into checked kernel-owned transport storage.
// ------------------=
static PyObject *command(PyObject *self, PyObject *args) {
    (void)self; unsigned long long id; int op; const char *data; Py_ssize_t length, count;
    if (!PyArg_ParseTuple(args, "Kiy#n", &id, &op, &data, &length, &count)) return NULL;
    if (op != 0 && count != length) { PyErr_SetString(PyExc_ValueError, "Socket write size mismatch"); return NULL; }
    return result(self, infinity_system_call(0x1271, id, op, (uintptr_t)data, count, 0, 0));
}
// ------------------------=
// FUNC: poll_socket
// DESC: Returns typed native progress and actual byte counts without blocking the interpreter's scheduler slice.
// ------------------=
static PyObject *poll_socket(PyObject *self, PyObject *args) {
    (void)self; unsigned long long id; uint64_t data[3] = {0};
    if (!PyArg_ParseTuple(args, "K", &id)) return NULL;
    int64_t value = infinity_system_call(0x1272, id, (uintptr_t)data, 0, 0, 0, 0);
    if (value == -11) Py_RETURN_NONE;
    if (value < 0) return result(self, value);
    return Py_BuildValue("(KKy#)", (unsigned long long)data[0],
        (unsigned long long)data[1], (char *)&data[2], (Py_ssize_t)4);
}
// ------------------------=
// FUNC: receive
// DESC: Copies the actual completed plaintext byte range from the authorized channel into a new Python bytes object.
// ------------------=
static PyObject *receive(PyObject *self, PyObject *args) {
    (void)self; unsigned long long id; Py_ssize_t size;
    if (!PyArg_ParseTuple(args, "Kn", &id, &size)) return NULL;
    if (size < 0 || size > 16384) { PyErr_SetString(PyExc_ValueError, "Socket receive exceeds 16 KiB"); return NULL; }
    PyObject *bytes = PyBytes_FromStringAndSize(NULL, size);
    if (!bytes) return NULL;
    int64_t value = infinity_system_call(0x1273, id, (uintptr_t)PyBytes_AS_STRING(bytes), size, 0, 0, 0);
    if (value < 0) { Py_DECREF(bytes); return result(self, value); }
    if (_PyBytes_Resize(&bytes, value) < 0) return NULL;
    return bytes;
}
// ------------------------=
// FUNC: close_socket
// DESC: Cancels and erases only the supplied generation-fenced channel.
// ------------------=
static PyObject *close_socket(PyObject *self, PyObject *args) {
    (void)self; unsigned long long id;
    if (!PyArg_ParseTuple(args, "K", &id)) return NULL;
    return result(self, infinity_system_call(0x1274, id, 0, 0, 0, 0, 0));
}
static PyMethodDef methods[] = {
    {"begin", begin, METH_VARARGS, NULL}, {"command", command, METH_VARARGS, NULL},
    {"poll", poll_socket, METH_VARARGS, NULL}, {"receive", receive, METH_VARARGS, NULL},
    {"close", close_socket, METH_VARARGS, NULL}, {NULL, NULL, 0, NULL}
};
static struct PyModuleDef module = {PyModuleDef_HEAD_INIT, "_infinity_socket", NULL, -1, methods};
// ------------------------=
// FUNC: PyInit__infinity_socket
// DESC: Exposes the real native DNS, TCP and TLS byte-stream actor without host sockets or user-selectable insecure verification.
// ------------------=
PyMODINIT_FUNC PyInit__infinity_socket(void) {
    PyObject *value = PyModule_Create(&module);
    if (!value) return NULL;
    PyObject *error = PyErr_NewException("_infinity_socket.CertificateError", PyExc_OSError, NULL);
    if (!error || PyModule_AddObjectRef(value, "CertificateError", error) < 0) {
        Py_XDECREF(error); Py_DECREF(value); return NULL;
    }
    Py_DECREF(error); return value;
}
