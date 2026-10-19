static long bias = 7;
static long *bias_address = &bias;
/* A real relocated const pointer must remain read-only after the loader writes it. */
long *const infinity_extension_readonly_address = &bias;

// ------------------------=
// FUNC: infinity_extension_value
// DESC: Exercises a real position-independent data relocation without host libraries.
// ------------------=
long infinity_extension_value(long value) { return value + *bias_address; }

// ------------------------=
// FUNC: infinity_extension_bias_address
// DESC: Exposes the relocated data pointer for structured loader verification.
// ------------------=
long *infinity_extension_bias_address(void) { return bias_address; }
