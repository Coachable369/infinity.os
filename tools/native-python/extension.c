#include "Python.h"
#include <infinity/font.h>

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

// ------------------------=
// FUNC: glyph
// DESC: Exercises the real native FontKit syscall and private-user copy boundary from a loaded Python extension.
// ------------------=
static PyObject *glyph(PyObject *self, PyObject *args) {
    (void)self;
    unsigned int scalar, height, flags;
    if (!PyArg_ParseTuple(args, "III", &scalar, &height, &flags)) return NULL;
    InfinityFontRequest request = {1, scalar, height, flags};
    InfinityFontMetrics metrics;
    unsigned char coverage[96 * 96];
    int64_t status = infinity_font_glyph(&request, &metrics, coverage, sizeof(coverage));
    if (status < 0) return PyLong_FromLongLong(status);
    if (metrics.version != 1 || metrics.bytes_written < 0 || metrics.bytes_written > (int)sizeof(coverage)) {
        PyErr_SetString(PyExc_RuntimeError, "Invalid native font response");
        return NULL;
    }
    return Py_BuildValue("(iiiiiy#)", metrics.width, metrics.height, metrics.advance_x,
        metrics.ascent, metrics.descent, coverage, (Py_ssize_t)metrics.bytes_written);
}

// ------------------------=
// FUNC: glyph_bad_addresses
// DESC: Checks native pointer rejection without dereferencing invalid addresses or accepting partial metadata writes.
// ------------------=
static PyObject *glyph_bad_addresses(PyObject *self, PyObject *ignored) {
    (void)self; (void)ignored;
    InfinityFontRequest request = {1, 65, 28, 0};
    InfinityFontMetrics metrics;
    memset(&metrics, 0x5a, sizeof(metrics));
    unsigned char coverage[96 * 96];
    int64_t read_error = infinity_font_glyph((const InfinityFontRequest *)1, &metrics, coverage, sizeof(coverage));
    int64_t metric_error = infinity_font_glyph(&request, (InfinityFontMetrics *)1, coverage, sizeof(coverage));
    int64_t pixel_error = infinity_font_glyph(&request, &metrics, (unsigned char *)1, sizeof(coverage));
    int unchanged = 1;
    for (size_t i = 0; i < sizeof(metrics); ++i) if (((unsigned char *)&metrics)[i] != 0x5a) unchanged = 0;
    return Py_BuildValue("(LLLi)", (long long)read_error, (long long)metric_error, (long long)pixel_error, unchanged);
}

static PyMethodDef methods[] = {
    {"add", add, METH_VARARGS, "Add two native integers with checked overflow."},
    {"glyph", glyph, METH_VARARGS, "Read a real installed native font glyph."},
    {"glyph_bad_addresses", glyph_bad_addresses, METH_NOARGS, "Verify checked native glyph transport."},
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
