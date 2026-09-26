#include "fixture.hpp"
// ------------------------=
// FUNC: check_bits
// DESC: Validates Rust-generated normal bitfield writes using the C++ compiler's actual layout.
// ------------------=
extern "C" bool check_bits(const Bits* p) { return p->a == 5 && p->b == 19 && p->c == 301; }
// ------------------------=
// FUNC: check_packed
// DESC: Validates Rust-generated packed bitfield writes without assuming allocation-unit size.
// ------------------=
extern "C" bool check_packed(const Packed* p) { return p->a == 6 && p->b == 17 && p->c == 400; }
