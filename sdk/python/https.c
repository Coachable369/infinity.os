#define PY_SSIZE_T_CLEAN
#include "Python.h"
#include <stdint.h>
#include <infinity/syscall.h>
#include "network_errno.h"

// ------------------------=
// FUNC: result
// DESC: Converts checked kernel errors without disguising certificate rejection or inventing a successful transfer.
// ------------------=
static PyObject *result(PyObject *module, int64_t value) {
    if (value >= 0) return PyLong_FromLongLong(value);
    if (value == -129) {
        PyObject *error = PyObject_GetAttrString(module, "CertificateError");
        if (error) { PyErr_SetString(error, "Native TLS certificate verification failed"); Py_DECREF(error); }
        return NULL;
    }
    PyObject *arguments = Py_BuildValue("(is)", infinity_network_errno(value), "Native HTTPS operation failed");
    if (arguments) { PyErr_SetObject(PyExc_OSError, arguments); Py_DECREF(arguments); }
    return NULL;
}

// ------------------------=
// FUNC: begin
// DESC: Starts a kernel-validated HTTPS GET using an opaque transaction generation rather than a kernel pointer.
// ------------------=
static PyObject *begin(PyObject *self, PyObject *args) {
    (void)self;
    const char *url, *headers; Py_ssize_t url_len, header_len;
    if (!PyArg_ParseTuple(args, "y#y#", &url, &url_len, &headers, &header_len)) return NULL;
    return result(self, infinity_system_call(0x1240, (uintptr_t)url, url_len,
        (uintptr_t)headers, header_len, 0, 0));
}

// ------------------------=
// FUNC: poll_response
// DESC: Reads structured completed-response metadata or returns None while the native actor is pending.
// ------------------=
static PyObject *poll_response(PyObject *self, PyObject *args) {
    (void)self; unsigned long long id; uint64_t metadata[3];
    if (!PyArg_ParseTuple(args, "K", &id)) return NULL;
    int64_t value = infinity_system_call(0x1241, id, (uintptr_t)metadata, 0, 0, 0, 0);
    if (value == -11) Py_RETURN_NONE;
    if (value < 0) return result(self, value);
    return Py_BuildValue("(KKK)", (unsigned long long)metadata[0],
        (unsigned long long)metadata[1], (unsigned long long)metadata[2]);
}

// ------------------------=
// FUNC: read_response
// DESC: Copies one bounded response region through checked native memory marshaling.
// ------------------=
static PyObject *read_response(PyObject *self, PyObject *args) {
    (void)self; unsigned long long id, region, offset, size;
    if (!PyArg_ParseTuple(args, "KKKK", &id, &region, &offset, &size)) return NULL;
    if (size > 65536) { PyErr_SetString(PyExc_ValueError, "HTTPS read exceeds 64 KiB"); return NULL; }
    PyObject *bytes = PyBytes_FromStringAndSize(NULL, size);
    if (!bytes) return NULL;
    int64_t value = infinity_system_call(0x1242, id, region, offset,
        (uintptr_t)PyBytes_AS_STRING(bytes), size, 0);
    if (value < 0) { Py_DECREF(bytes); return result(self, value); }
    if (_PyBytes_Resize(&bytes, value) < 0) return NULL;
    return bytes;
}

// ------------------------=
// FUNC: close_response
// DESC: Releases or cancels only the caller's exact HTTPS transaction.
// ------------------=
static PyObject *close_response(PyObject *self, PyObject *args) {
    (void)self; unsigned long long id;
    if (!PyArg_ParseTuple(args, "K", &id)) return NULL;
    return result(self, infinity_system_call(0x1243, id, 0, 0, 0, 0, 0));
}

static PyMethodDef methods[] = {
    {"begin", begin, METH_VARARGS, NULL},
    {"poll", poll_response, METH_VARARGS, NULL},
    {"read", read_response, METH_VARARGS, NULL},
    {"close", close_response, METH_VARARGS, NULL},
    {NULL, NULL, 0, NULL}
};
static struct PyModuleDef module = {PyModuleDef_HEAD_INIT, "_infinity_https", NULL, -1, methods};

// ------------------------=
// FUNC: PyInit__infinity_https
// DESC: Exposes the real native authenticated HTTPS actor, not a substitute SSL socket implementation.
// ------------------=
PyMODINIT_FUNC PyInit__infinity_https(void) {
    PyObject *value = PyModule_Create(&module);
    if (!value) return NULL;
    PyObject *error = PyErr_NewException("_infinity_https.CertificateError", PyExc_OSError, NULL);
    if (!error || PyModule_AddObjectRef(value, "CertificateError", error) < 0) {
        Py_XDECREF(error); Py_DECREF(value); return NULL;
    }
    Py_DECREF(error); return value;
}
