"""Actual guest FFI behavior against an explicitly mounted target-native .so."""
import ctypes as C
import sys

assert sys.platform == "infinityos"
assert C.sizeof(C.c_void_p) == C.sizeof(C.c_long) == 8
assert C.sizeof(C.c_wchar) == 4
assert C.pythonapi is None


class Pair(C.Structure):
    _fields_ = [("integer", C.c_int32), ("decimal", C.c_double)]


class Vector(C.Structure):
    _fields_ = [("values", C.c_double * 4)]


class Large(C.Structure):
    _fields_ = [("values", C.c_int64 * 5)]


class Bits(C.Union):
    _fields_ = [("integer", C.c_uint32), ("bytes", C.c_ubyte * 4)]


assert (C.sizeof(Pair), C.alignment(Pair), Pair.decimal.offset) == (16, 8, 8)
bits = Bits(integer=0x04030201)
assert list(bits.bytes) == [1, 2, 3, 4]
wide = C.create_unicode_buffer("native π")
assert wide.value == "native π"
source = C.create_string_buffer(b"private-memory")
destination = C.create_string_buffer(32)
C.memmove(destination, source, len(source))
assert C.string_at(destination) == b"private-memory"
C.memset(destination, 0x41, 3)
assert destination.raw[:4] == b"AAAv"

library = C.CDLL("/infinity_native_test.so", use_errno=True)
library.native_ffi_integer.argtypes = [C.c_int8, C.c_uint16, C.c_int64]
library.native_ffi_integer.restype = C.c_int64
assert library.native_ffi_integer(-3, 65530, 1 << 40) == (1 << 40) - 196590
library.native_ffi_mixed.argtypes = [C.c_int32, C.c_double, C.c_float, C.c_uint64]
library.native_ffi_mixed.restype = C.c_double
assert library.native_ffi_mixed(-7, 2.5, 4.0, 9) == 12.0
library.native_ffi_pair.argtypes = [Pair, C.c_int32]
library.native_ffi_pair.restype = Pair
pair = Pair(12, 1.25)
changed = library.native_ffi_pair(pair, -2)
assert (changed.integer, changed.decimal) == (10, 2.5)
assert (pair.integer, pair.decimal) == (12, 1.25)
library.native_ffi_vector.argtypes = [Vector]
library.native_ffi_vector.restype = Vector
assert list(library.native_ffi_vector(Vector((1, 2, 3, 4))).values) == [1, 3, 5, 7]
library.native_ffi_large.argtypes = [Large]
library.native_ffi_large.restype = Large
assert list(library.native_ffi_large(Large((2, 3, 4, 5, 6))).values) == [2, 6, 12, 20, 30]
library.native_ffi_variadic.argtypes = [C.c_uint]
library.native_ffi_variadic.restype = C.c_double
assert library.native_ffi_variadic(12, *(C.c_double(i + 0.25) for i in range(12))) == 69.0
library.native_ffi_many.argtypes = [C.c_int64] * 12
library.native_ffi_many.restype = C.c_int64
assert library.native_ffi_many(*range(12)) == 66
library.native_ffi_bytes.argtypes = [C.POINTER(C.c_ubyte), C.c_size_t]
library.native_ffi_bytes.restype = None
buffer = (C.c_ubyte * 4)(0, 1, 0x55, 0xff)
library.native_ffi_bytes(buffer, len(buffer))
assert list(buffer) == [0x5a, 0x5b, 0x0f, 0xa5]
library.native_ffi_counter.restype = C.POINTER(C.c_int64)
counter = library.native_ffi_counter()
assert counter.contents.value == 7
counter.contents.value = 54
assert library.native_ffi_counter().contents.value == 54
prototype = C.CFUNCTYPE(C.c_int64, C.c_int8, C.c_uint16, C.c_int64)
outbound = prototype(("native_ffi_integer", library))
assert outbound(2, 3, 48) == 54
C.set_errno(33)
assert library.native_ffi_many(*range(12)) == 66 and C.get_errno() == 33

for path in (None, "/missing.so", "/script.py", "/system/kernel.so", "../foreign.so"):
    try:
        C.CDLL(path)
    except OSError:
        pass
    else:
        raise AssertionError(("unauthorized or invalid library admitted", path))
try:
    library.nonexistent_export
except AttributeError:
    pass
else:
    raise AssertionError("missing export admitted")
try:
    prototype(lambda a, b, c: a + b + c)
except NotImplementedError:
    pass
else:
    raise AssertionError("executable callback allocated")
sys.exit(54)
