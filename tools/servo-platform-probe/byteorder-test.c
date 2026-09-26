#include <arpa/inet.h>
// ------------------------=
// FUNC: main
// DESC: Verifies actual network-order bytes, roundtrips and single argument evaluation.
// ------------------=
int main(void) {
    uint32_t value = 0x12345678;
    uint32_t network = htonl(value++);
    const unsigned char *bytes = (const unsigned char *)&network;
    if (value != 0x12345679 || bytes[0] != 0x12 || bytes[1] != 0x34 ||
        bytes[2] != 0x56 || bytes[3] != 0x78 || ntohl(network) != 0x12345678) return 1;
    for (uint32_t index = 0; index <= 65535; ++index) {
        uint16_t wire = htons((uint16_t)index);
        bytes = (const unsigned char *)&wire;
        if (bytes[0] != index / 256 || bytes[1] != index % 256 || ntohs(wire) != index) return 2;
    }
    return 0;
}
