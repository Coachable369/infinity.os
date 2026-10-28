#include <assert.h>
#include "../../sdk/python/network_errno.h"

// ------------------------=
// FUNC: main
// DESC: Verifies stable broker failures map to actual target errno values and never classify a numeric public errno as certificate evidence.
// ------------------=
int main(void) {
    assert(infinity_network_errno(-11) == EAGAIN);
    assert(infinity_network_errno(-13) == EACCES);
    assert(infinity_network_errno(-110) == ETIMEDOUT);
    assert(infinity_network_errno(-113) == EHOSTUNREACH);
    assert(infinity_network_errno(-100) == ENETDOWN);
    assert(infinity_network_errno(-125) == ECANCELED);
    assert(infinity_network_errno(-129) == EIO);
    return 0;
}
