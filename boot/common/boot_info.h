#ifndef INFINITY_BOOT_INFO_H
#define INFINITY_BOOT_INFO_H

#include <stdint.h>

#define INFINITY_BOOT_MAGIC UINT64_C(0x494e46424f4f5430)
#define INFINITY_BOOT_VERSION 5u
#define INFINITY_ARCH_X86_64 2u
#define INFINITY_ARCH_AARCH64 3u

typedef struct {
    uint64_t magic;
    uint32_t version;
    uint32_t architecture;
    uint64_t memory_map_address;
    uint64_t memory_map_size;
    uint64_t memory_descriptor_size;
    uint64_t firmware_revision;
    uint64_t boot_flags;
    uint64_t framebuffer_address;
    uint64_t framebuffer_size;
    uint32_t framebuffer_width;
    uint32_t framebuffer_height;
    uint32_t framebuffer_stride;
    uint32_t framebuffer_format;
    uint64_t firmware_input;
    uint64_t firmware_pointer;
    uint64_t firmware_runtime_services;
    uint64_t firmware_network;
    uint32_t network_device_count;
    uint32_t network_link_state;
    uint32_t network_mtu;
    uint32_t network_capabilities;
    uint32_t network_mac_length;
    uint32_t network_reserved;
    uint8_t network_mac[32];
} InfinityBootInfo;

_Static_assert(sizeof(InfinityBootInfo) == 176, "BootInfo ABI changed");

#endif
