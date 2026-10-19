#include "Python.h"

// ------------------------=
// FUNC: add
// DESC: Exercises CPython argument parsing and integer result allocation from a separately loaded native extension.
// ------------------=
static PyObject *add(PyObject *self, PyObject *args) {
    (void)self;
    long left, right;
    if (!PyArg_ParseTuple(args, "ll", &left, &right)) return NULL;
    long result;
    if (__builtin_add_overflow(left, right, &result)) {
        PyErr_SetString(PyExc_OverflowError, "addition overflow");
        return NULL;
    }
    return PyLong_FromLong(result);
}

static PyMethodDef methods[] = {
    {"add", add, METH_VARARGS, "Add two native integers with checked overflow."},
    {NULL, NULL, 0, NULL}
};
static struct PyModuleDef definition = {
    PyModuleDef_HEAD_INIT, "infinity_native_test", NULL, -1, methods
};

// ------------------------=
// FUNC: PyInit_infinity_native_test
// DESC: Exports the standard CPython extension entry point rather than a private substitute ABI.
// ------------------=
PyMODINIT_FUNC PyInit_infinity_native_test(void) {
    return PyModule_Create(&definition);
}
