#ifndef INFINITY_ASSERT_H
#define INFINITY_ASSERT_H

#ifdef NDEBUG
#define assert(expression) ((void)0)
#else
#define assert(expression) ((expression) ? (void)0 : __builtin_trap())
#endif

#endif
