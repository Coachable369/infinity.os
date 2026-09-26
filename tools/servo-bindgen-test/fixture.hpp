#include <type_traits>
struct Bits { unsigned a : 3; unsigned b : 5; unsigned c : 9; };
struct __attribute__((packed)) Packed { unsigned a : 3; unsigned b : 5; unsigned c : 9; };
struct Plain { char tag; unsigned value; };
