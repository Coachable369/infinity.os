#include <stddef.h>
#include <stdint.h>
#include "boot_info.h"
#include "video_modes.h"

#define EFIAPI __attribute__((ms_abi))
#include "tpm_random.h"
#define EFI_SUCCESS 0
#define EFI_BUFFER_TOO_SMALL UINT64_C(0x8000000000000005)
#define EFI_LOADER_DATA 2
#define EFI_ALLOCATE_ANY_PAGES 0
#define EFI_ALLOCATE_MAX_ADDRESS 1
#define EFI_ALLOCATE_ADDRESS 2
#define EFI_OPEN_READ 1
#define EFI_FILE_MODE_READ 1
#define EFI_BY_PROTOCOL 2
#define EFI_ALL_HANDLES 0
#define PAGE_SIZE 4096u
#define PAGE_MASK (PAGE_SIZE - 1u)
#define KERNEL_STACK_PAGES 256u
#define NATIVE_RUNTIME_POOL_PAGES 65536u
#define IDENTITY_MAP_GIB 64u
#define PT_LOAD 1u
#if defined(INFINITY_AARCH64)
#define INFINITY_ELF_MACHINE 183u
#define INFINITY_ARCHITECTURE INFINITY_ARCH_AARCH64
#else
#define INFINITY_ELF_MACHINE 62u
#define INFINITY_ARCHITECTURE INFINITY_ARCH_X86_64
#endif

typedef uint64_t EFI_STATUS;
typedef void *EFI_HANDLE;
typedef uint16_t CHAR16;

typedef struct {
    uint32_t data1;
    uint16_t data2;
    uint16_t data3;
    uint8_t data4[8];
} EFI_GUID;

typedef struct {
    uint64_t signature;
    uint32_t revision;
    uint32_t header_size;
    uint32_t crc32;
    uint32_t reserved;
} EFI_TABLE_HEADER;

typedef struct EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL;
struct EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL {
    EFI_STATUS (EFIAPI *reset)(EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL *, uint8_t);
    EFI_STATUS (EFIAPI *output_string)(EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL *, const CHAR16 *);
};

typedef struct EFI_SIMPLE_TEXT_INPUT_PROTOCOL EFI_SIMPLE_TEXT_INPUT_PROTOCOL;
struct EFI_SIMPLE_TEXT_INPUT_PROTOCOL {
    EFI_STATUS (EFIAPI *reset)(EFI_SIMPLE_TEXT_INPUT_PROTOCOL *, uint8_t);
    EFI_STATUS (EFIAPI *read_key_stroke)(EFI_SIMPLE_TEXT_INPUT_PROTOCOL *, void *);
    void *wait_for_key;
};

typedef struct EFI_SIMPLE_POINTER_PROTOCOL EFI_SIMPLE_POINTER_PROTOCOL;
struct EFI_SIMPLE_POINTER_PROTOCOL {
    void *reset;
    void *get_state;
    void *wait_for_input;
    void *mode;
};

typedef struct EFI_ABSOLUTE_POINTER_PROTOCOL EFI_ABSOLUTE_POINTER_PROTOCOL;
struct EFI_ABSOLUTE_POINTER_PROTOCOL {
    void *reset;
    void *get_state;
    void *wait_for_input;
    void *mode;
};

typedef struct EFI_USB_IO_PROTOCOL EFI_USB_IO_PROTOCOL;
typedef struct EFI_BLOCK_IO_PROTOCOL EFI_BLOCK_IO_PROTOCOL;
typedef struct __attribute__((packed)) {
    uint8_t length, descriptor_type, interface_number, alternate_setting;
    uint8_t num_endpoints, interface_class, interface_subclass, interface_protocol, interface;
} EFI_USB_INTERFACE_DESCRIPTOR;
typedef struct __attribute__((packed)) {
    uint8_t length, descriptor_type, endpoint_address, attributes;
    uint16_t max_packet_size;
    uint8_t interval;
} EFI_USB_ENDPOINT_DESCRIPTOR;
typedef struct __attribute__((packed)) {
    uint8_t request_type, request;
    uint16_t value, index, length;
} EFI_USB_DEVICE_REQUEST;
typedef EFI_STATUS (EFIAPI *EFI_USB_CONTROL_TRANSFER)(EFI_USB_IO_PROTOCOL *,
    EFI_USB_DEVICE_REQUEST *, uint32_t, uint32_t, void *, size_t, uint32_t *);
typedef EFI_STATUS (EFIAPI *EFI_USB_INTERRUPT_CALLBACK)(void *, size_t, void *, uint32_t);
typedef EFI_STATUS (EFIAPI *EFI_USB_ASYNC_INTERRUPT_TRANSFER)(EFI_USB_IO_PROTOCOL *,
    uint8_t, uint8_t, size_t, size_t, EFI_USB_INTERRUPT_CALLBACK, void *);
typedef EFI_STATUS (EFIAPI *EFI_USB_GET_INTERFACE_DESCRIPTOR)(EFI_USB_IO_PROTOCOL *, EFI_USB_INTERFACE_DESCRIPTOR *);
typedef EFI_STATUS (EFIAPI *EFI_USB_GET_ENDPOINT_DESCRIPTOR)(EFI_USB_IO_PROTOCOL *, uint8_t, EFI_USB_ENDPOINT_DESCRIPTOR *);
struct EFI_USB_IO_PROTOCOL {
    EFI_USB_CONTROL_TRANSFER control_transfer;
    void *bulk_transfer;
    EFI_USB_ASYNC_INTERRUPT_TRANSFER async_interrupt_transfer;
    void *sync_interrupt_transfer;
    void *isochronous_transfer;
    void *async_isochronous_transfer;
    void *get_device_descriptor;
    void *get_config_descriptor;
    EFI_USB_GET_INTERFACE_DESCRIPTOR get_interface_descriptor;
    EFI_USB_GET_ENDPOINT_DESCRIPTOR get_endpoint_descriptor;
    void *port_reset;
};

#define INFINITY_POINTER_PROTOCOLS 8u
typedef struct {
    void *check_event;
    uint32_t absolute_count;
    uint32_t relative_count;
    uint32_t usb_mouse_count;
    uint32_t reserved;
    EFI_ABSOLUTE_POINTER_PROTOCOL *absolute[INFINITY_POINTER_PROTOCOLS];
    EFI_SIMPLE_POINTER_PROTOCOL *relative[INFINITY_POINTER_PROTOCOLS];
    EFI_USB_IO_PROTOCOL *usb_mouse[INFINITY_POINTER_PROTOCOLS];
    uint8_t usb_endpoint[INFINITY_POINTER_PROTOCOLS];
    uint8_t usb_mouse_absolute[INFINITY_POINTER_PROTOCOLS];
    uint32_t usb_keyboard_count;
    uint32_t reserved2;
    EFI_USB_IO_PROTOCOL *usb_keyboard[INFINITY_POINTER_PROTOCOLS];
    uint8_t usb_keyboard_endpoint[INFINITY_POINTER_PROTOCOLS];
    volatile uint32_t usb_mouse_async;
    volatile uint32_t usb_mouse_sequence;
    volatile uint32_t usb_mouse_length;
    volatile uint8_t usb_mouse_report[8];
    uint32_t block_count;
    uint32_t reserved3;
    EFI_BLOCK_IO_PROTOCOL *blocks[INFINITY_POINTER_PROTOCOLS];
} InfinityFirmwarePointers;

typedef EFI_STATUS (EFIAPI *EFI_ALLOCATE_PAGES)(uint32_t, uint32_t, size_t, uint64_t *);
typedef EFI_STATUS (EFIAPI *EFI_FREE_PAGES)(uint64_t, size_t);
typedef EFI_STATUS (EFIAPI *EFI_GET_MEMORY_MAP)(size_t *, void *, uint64_t *, size_t *, uint32_t *);
typedef EFI_STATUS (EFIAPI *EFI_ALLOCATE_POOL)(uint32_t, size_t, void **);
typedef EFI_STATUS (EFIAPI *EFI_FREE_POOL)(void *);
typedef EFI_STATUS (EFIAPI *EFI_HANDLE_PROTOCOL)(EFI_HANDLE, EFI_GUID *, void **);
typedef EFI_STATUS (EFIAPI *EFI_EXIT_BOOT_SERVICES)(EFI_HANDLE, uint64_t);
typedef EFI_STATUS (EFIAPI *EFI_SET_WATCHDOG_TIMER)(size_t, uint64_t, size_t, const CHAR16 *);
typedef EFI_STATUS (EFIAPI *EFI_LOCATE_PROTOCOL)(EFI_GUID *, void *, void **);
typedef EFI_STATUS (EFIAPI *EFI_LOCATE_HANDLE_BUFFER)(uint32_t, EFI_GUID *, void *, size_t *, EFI_HANDLE **);
typedef EFI_STATUS (EFIAPI *EFI_CONNECT_CONTROLLER)(EFI_HANDLE, EFI_HANDLE *, void *, uint8_t);
typedef struct EFI_RNG_PROTOCOL EFI_RNG_PROTOCOL;
typedef EFI_STATUS (EFIAPI *EFI_GET_RNG)(EFI_RNG_PROTOCOL *, EFI_GUID *, size_t, uint8_t *);
struct EFI_RNG_PROTOCOL {
    void *get_info;
    EFI_GET_RNG get_rng;
};

typedef struct {
    EFI_TABLE_HEADER header;
    void *raise_tpl;
    void *restore_tpl;
    EFI_ALLOCATE_PAGES allocate_pages;
    EFI_FREE_PAGES free_pages;
    EFI_GET_MEMORY_MAP get_memory_map;
    EFI_ALLOCATE_POOL allocate_pool;
    EFI_FREE_POOL free_pool;
    void *unused_07_15[9];
    EFI_HANDLE_PROTOCOL handle_protocol;
    void *unused_17_25[9];
    EFI_EXIT_BOOT_SERVICES exit_boot_services;
    void *unused_27_28[2];
    EFI_SET_WATCHDOG_TIMER set_watchdog_timer;
    EFI_CONNECT_CONTROLLER connect_controller;
    void *unused_31_35[5];
    EFI_LOCATE_HANDLE_BUFFER locate_handle_buffer;
    EFI_LOCATE_PROTOCOL locate_protocol;
} EFI_BOOT_SERVICES;

typedef struct {
    EFI_GUID vendor_guid;
    void *vendor_table;
} EFI_CONFIGURATION_TABLE;

typedef struct {
    EFI_TABLE_HEADER header;
    CHAR16 *firmware_vendor;
    uint32_t firmware_revision;
    uint32_t pad;
    EFI_HANDLE console_in_handle;
    EFI_SIMPLE_TEXT_INPUT_PROTOCOL *con_in;
    EFI_HANDLE console_out_handle;
    EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL *con_out;
    EFI_HANDLE stderr_handle;
    EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL *std_err;
    void *runtime_services;
    EFI_BOOT_SERVICES *boot_services;
    size_t number_of_table_entries;
    EFI_CONFIGURATION_TABLE *configuration_table;
} EFI_SYSTEM_TABLE;

#if defined(INFINITY_AARCH64)
#include "worker_bridge.h"
#else
#include "../x86_64/workers.h"
#endif

typedef struct {
    uint32_t revision;
    EFI_HANDLE parent_handle;
    EFI_SYSTEM_TABLE *system_table;
    EFI_HANDLE device_handle;
    void *file_path;
    void *reserved;
    uint32_t load_options_size;
    uint32_t pad;
    void *load_options;
    void *image_base;
    uint64_t image_size;
} EFI_LOADED_IMAGE_PROTOCOL;

typedef struct EFI_FILE_PROTOCOL EFI_FILE_PROTOCOL;
struct EFI_FILE_PROTOCOL {
    uint64_t revision;
    EFI_STATUS (EFIAPI *open)(EFI_FILE_PROTOCOL *, EFI_FILE_PROTOCOL **, CHAR16 *, uint64_t, uint64_t);
    EFI_STATUS (EFIAPI *close)(EFI_FILE_PROTOCOL *);
    void *delete_file;
    EFI_STATUS (EFIAPI *read)(EFI_FILE_PROTOCOL *, size_t *, void *);
    void *write;
    void *get_position;
    EFI_STATUS (EFIAPI *set_position)(EFI_FILE_PROTOCOL *, uint64_t);
    EFI_STATUS (EFIAPI *get_info)(EFI_FILE_PROTOCOL *, EFI_GUID *, size_t *, void *);
};

typedef struct EFI_SIMPLE_FILE_SYSTEM_PROTOCOL EFI_SIMPLE_FILE_SYSTEM_PROTOCOL;
struct EFI_SIMPLE_FILE_SYSTEM_PROTOCOL {
    uint64_t revision;
    EFI_STATUS (EFIAPI *open_volume)(EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *, EFI_FILE_PROTOCOL **);
};

typedef struct {
    uint32_t version;
    uint32_t horizontal_resolution;
    uint32_t vertical_resolution;
    uint32_t pixel_format;
    uint32_t pixel_information[4];
    uint32_t pixels_per_scan_line;
} EFI_GRAPHICS_OUTPUT_MODE_INFORMATION;

typedef struct {
    uint32_t max_mode;
    uint32_t mode;
    EFI_GRAPHICS_OUTPUT_MODE_INFORMATION *info;
    size_t size_of_info;
    uint64_t framebuffer_base;
    size_t framebuffer_size;
} EFI_GRAPHICS_OUTPUT_PROTOCOL_MODE;

typedef struct EFI_GRAPHICS_OUTPUT_PROTOCOL EFI_GRAPHICS_OUTPUT_PROTOCOL;
typedef EFI_STATUS (EFIAPI *EFI_GRAPHICS_QUERY_MODE)(EFI_GRAPHICS_OUTPUT_PROTOCOL *, uint32_t,
                                                      size_t *, EFI_GRAPHICS_OUTPUT_MODE_INFORMATION **);
typedef EFI_STATUS (EFIAPI *EFI_GRAPHICS_SET_MODE)(EFI_GRAPHICS_OUTPUT_PROTOCOL *, uint32_t);

struct EFI_GRAPHICS_OUTPUT_PROTOCOL {
    EFI_GRAPHICS_QUERY_MODE query_mode;
    EFI_GRAPHICS_SET_MODE set_mode;
    void *blt;
    EFI_GRAPHICS_OUTPUT_PROTOCOL_MODE *mode;
};

typedef struct {
    uint32_t media_id;
    uint8_t removable_media;
    uint8_t media_present;
    uint8_t logical_partition;
    uint8_t read_only;
    uint8_t write_caching;
    uint32_t block_size;
    uint32_t io_align;
    uint64_t last_block;
    uint64_t lowest_aligned_lba;
    uint32_t logical_blocks_per_physical_block;
    uint32_t optimal_transfer_length_granularity;
} EFI_BLOCK_IO_MEDIA;

typedef EFI_STATUS (EFIAPI *EFI_BLOCK_READ)(EFI_BLOCK_IO_PROTOCOL *, uint32_t, uint64_t, size_t, void *);
struct EFI_BLOCK_IO_PROTOCOL {
    uint64_t revision;
    EFI_BLOCK_IO_MEDIA *media;
    void *reset;
    EFI_BLOCK_READ read_blocks;
    void *write_blocks;
    void *flush_blocks;
};

typedef struct {
    uint8_t address[32];
} EFI_MAC_ADDRESS;

typedef struct {
    uint32_t state;
    uint32_t hw_address_size;
    uint32_t media_header_size;
    uint32_t max_packet_size;
    uint32_t nvram_size;
    uint32_t nvram_access_size;
    uint32_t receive_filter_mask;
    uint32_t receive_filter_setting;
    uint32_t max_mcast_filter_count;
    uint32_t mcast_filter_count;
    EFI_MAC_ADDRESS mcast_filter[16];
    EFI_MAC_ADDRESS current_address;
    EFI_MAC_ADDRESS broadcast_address;
    EFI_MAC_ADDRESS permanent_address;
    uint8_t if_type;
    uint8_t mac_address_changeable;
    uint8_t multiple_tx_supported;
    uint8_t media_present_supported;
    uint8_t media_present;
} EFI_SIMPLE_NETWORK_MODE;

typedef struct {
    uint64_t revision;
    void *start;
    void *stop;
    void *initialize;
    void *reset;
    void *shutdown;
    void *receive_filters;
    void *station_address;
    void *statistics;
    void *mcast_ip_to_mac;
    void *nv_data;
    void *get_status;
    void *transmit;
    void *receive;
    void *wait_for_packet;
    EFI_SIMPLE_NETWORK_MODE *mode;
} EFI_SIMPLE_NETWORK_PROTOCOL;

typedef struct EFI_PCI_IO_PROTOCOL EFI_PCI_IO_PROTOCOL;
typedef EFI_STATUS (EFIAPI *EFI_PCI_CONFIG_ACCESS)(EFI_PCI_IO_PROTOCOL *,
    uint32_t, uint64_t, size_t, void *);
typedef struct {
    EFI_PCI_CONFIG_ACCESS read;
    EFI_PCI_CONFIG_ACCESS write;
} EFI_PCI_CONFIG_ACCESS_PAIR;
struct EFI_PCI_IO_PROTOCOL {
    void *poll_mem;
    void *poll_io;
    void *mem_read;
    void *mem_write;
    void *io_read;
    void *io_write;
    EFI_PCI_CONFIG_ACCESS_PAIR pci;
};

typedef struct {
    uint8_t ident[16];
    uint16_t type;
    uint16_t machine;
    uint32_t version;
    uint64_t entry;
    uint64_t phoff;
    uint64_t shoff;
    uint32_t flags;
    uint16_t ehsize;
    uint16_t phentsize;
    uint16_t phnum;
    uint16_t shentsize;
    uint16_t shnum;
    uint16_t shstrndx;
} Elf64Header;

typedef struct {
    uint32_t type;
    uint32_t flags;
    uint64_t offset;
    uint64_t vaddr;
    uint64_t paddr;
    uint64_t filesz;
    uint64_t memsz;
    uint64_t align;
} Elf64ProgramHeader;

typedef struct {
    uint64_t entry;
    uint64_t low;
    uint64_t high;
    uint64_t text_low;
    uint64_t text_high;
    uint64_t rodata_low;
    uint64_t rodata_high;
    uint64_t data_low;
    uint64_t data_high;
} InfinityLoadedKernel;

static const EFI_GUID loaded_image_guid = {0x5b1b31a1, 0x9562, 0x11d2,
    {0x8e, 0x3f, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b}};
static const EFI_GUID simple_fs_guid = {0x964e5b22, 0x6459, 0x11d2,
    {0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b}};
static const EFI_GUID file_info_guid = {0x09576e92, 0x6d3f, 0x11d2,
    {0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b}};
static const EFI_GUID graphics_output_guid = {0x9042a9de, 0x23dc, 0x4a38,
    {0x96, 0xfb, 0x7a, 0xde, 0xd0, 0x80, 0x51, 0x6a}};
static const EFI_GUID block_io_guid = {0x964e5b21, 0x6459, 0x11d2,
    {0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b}};
static const EFI_GUID simple_network_guid = {0xa19832b9, 0xac25, 0x11d3,
    {0x9a, 0x2d, 0x00, 0x90, 0x27, 0x3f, 0xc1, 0x4d}};
static const EFI_GUID pci_io_guid = {0x4cf5b200, 0x68b8, 0x4ca5,
    {0x9e, 0xec, 0xb2, 0x3e, 0x3f, 0x50, 0x02, 0x9a}};
static const EFI_GUID rng_protocol_guid = {0x3152bca5, 0xeade, 0x433d,
    {0x86, 0x2e, 0xc0, 0x1c, 0xdc, 0x29, 0x1f, 0x44}};
static const EFI_GUID acpi20_table_guid = {0x8868e871, 0xe4f1, 0x11d3,
    {0xbc, 0x22, 0x00, 0x80, 0xc7, 0x3c, 0x88, 0x81}};
static const uint8_t infinity_partition_type[16] =
    {0x69,0x66,0x6e,0x49,0x69,0x6e,0x79,0x74,0x53,0x54,0x4f,0x52,0x41,0x47,0x45,0x31};

// ------------------------=
// FUNC: gather_firmware_entropy
// DESC: Obtains one boot-scoped seed from UEFI RNG or TPM2 GetRandom without inventing fallback randomness.
// ------------------=
static void gather_firmware_entropy(EFI_SYSTEM_TABLE *system, InfinityBootInfo *info) {
    EFI_RNG_PROTOCOL *rng = NULL;
    info->firmware_entropy_valid = 0;
    if (system->boot_services->locate_protocol((EFI_GUID *)&rng_protocol_guid, NULL,
            (void **)&rng) == EFI_SUCCESS && rng && rng->get_rng &&
        rng->get_rng(rng, NULL, sizeof(info->firmware_entropy), info->firmware_entropy) == EFI_SUCCESS) {
        info->firmware_entropy_valid = 1;
        return;
    }
    // VirtualBox ARM has no EFI_RNG_PROTOCOL, but exposes TCG2 when the VM's
    // TPM 2.0 is enabled. Use its real random source, not time/MAC/UUID hashes.
    EFI_GUID tcg2_guid = {0x607f766c, 0x7455, 0x42be,
        {0x93, 0x0b, 0xe4, 0xd7, 0x6d, 0xb2, 0x72, 0x0f}};
    InfinityTcg2 *tpm = NULL;
    if (system->boot_services->locate_protocol(&tcg2_guid, NULL, (void **)&tpm) == EFI_SUCCESS &&
        infinity_tpm_random(tpm, info->firmware_entropy)) info->firmware_entropy_valid = 1;
}

// ------------------------=
// FUNC: infinity_handoff
// DESC: Declares the architecture-specific transfer from UEFI into the kernel.
// ------------------=
extern void EFIAPI infinity_handoff(InfinityBootInfo *, uint64_t, uint64_t, uint64_t);

// ------------------------=
// FUNC: memcpy
// DESC: Copies a fixed number of bytes between memory regions.
// ------------------=
void *memcpy(void *destination, const void *source, size_t count) {
    uint8_t *to = destination;
    const uint8_t *from = source;
    while (count--) *to++ = *from++;
    return destination;
}

// ------------------------=
// FUNC: memset
// DESC: Fills a memory region with a byte value.
// ------------------=
void *memset(void *destination, int value, size_t count) {
    uint8_t *to = destination;
    while (count--) *to++ = (uint8_t)value;
    return destination;
}

#if !defined(INFINITY_AARCH64)
// ------------------------=
// FUNC: outb
// DESC: Writes outb to firmware or device state.
// ------------------=
static inline void outb(uint16_t port, uint8_t value) {
    __asm__ volatile ("outb %0, %1" : : "a"(value), "Nd"(port));
}

// ------------------------=
// FUNC: inb
// DESC: Reads inb from firmware or device state.
// ------------------=
static inline uint8_t inb(uint16_t port) {
    uint8_t value;
    __asm__ volatile ("inb %1, %0" : "=a"(value) : "Nd"(port));
    return value;
}
#endif

// ------------------------=
// FUNC: serial_initialize
// DESC: Initializes serial initialize.
// ------------------=
static void serial_initialize(void) {
#if defined(INFINITY_AARCH64)
    /* QEMU virt's PL011 is already configured by firmware. */
#else
    outb(0x3f9, 0x00); outb(0x3fb, 0x80); outb(0x3f8, 0x01);
    outb(0x3f9, 0x00); outb(0x3fb, 0x03); outb(0x3fa, 0xc7); outb(0x3fc, 0x0b);
#endif
}

// ------------------------=
// FUNC: serial_write
// DESC: Writes serial write to firmware or device state.
// ------------------=
static void serial_write(const char *text) {
#if defined(INFINITY_AARCH64)
    volatile uint32_t *uart_data = (volatile uint32_t *)(uintptr_t)0x09000000;
    volatile uint32_t *uart_flags = (volatile uint32_t *)(uintptr_t)0x09000018;
    for (; *text; ++text) {
        while ((*uart_flags & (1u << 5)) != 0) {}
        *uart_data = (uint8_t)*text;
    }
#else
    for (; *text; ++text) {
        while ((inb(0x3fd) & 0x20) == 0) {}
        outb(0x3f8, (uint8_t)*text);
    }
#endif
}

// ------------------------=
// FUNC: firmware_write
// DESC: Writes firmware write to firmware or device state.
// ------------------=
static void firmware_write(EFI_SYSTEM_TABLE *system, const CHAR16 *text) {
    if (system && system->con_out) system->con_out->output_string(system->con_out, text);
}

// ------------------------=
// FUNC: crc32_bytes
// DESC: Calculates crc32 bytes.
// ------------------=
static uint32_t crc32_bytes(const void *data, size_t count) {
    const uint8_t *bytes = data;
    uint32_t crc = UINT32_C(0xffffffff);
    while (count--) {
        crc ^= *bytes++;
        for (unsigned bit = 0; bit < 8; ++bit)
            crc = (crc >> 1) ^ ((crc & 1) ? UINT32_C(0xedb88320) : 0);
    }
    return ~crc;
}

// ------------------------=
// FUNC: read_u32
// DESC: Reads read u32 from firmware or device state.
// ------------------=
static uint32_t read_u32(const uint8_t *data) {
    return (uint32_t)data[0] | ((uint32_t)data[1] << 8) | ((uint32_t)data[2] << 16) | ((uint32_t)data[3] << 24);
}

// ------------------------=
// FUNC: read_u64
// DESC: Reads read u64 from firmware or device state.
// ------------------=
static uint64_t read_u64(const uint8_t *data) {
    return (uint64_t)read_u32(data) | ((uint64_t)read_u32(data + 4) << 32);
}

// ------------------------=
// FUNC: equal_bytes
// DESC: Validates equal bytes.
// ------------------=
static uint8_t equal_bytes(const uint8_t *left, const uint8_t *right, size_t count) {
    while (count--) if (*left++ != *right++) return 0;
    return 1;
}

// ------------------------=
// FUNC: valid_sector_record
// DESC: Validates valid sector record.
// ------------------=
static uint8_t valid_sector_record(uint8_t *sector, size_t size, size_t crc_offset) {
    uint32_t expected = read_u32(sector + crc_offset);
    memset(sector + crc_offset, 0, 4);
    uint32_t actual = crc32_bytes(sector, size);
    return expected == actual;
}

static uint8_t installed_generation_invalid;
static uint32_t installed_generation_stage;

// ------------------------=
// FUNC: read_blocks_bounded
// DESC: Reads a large block extent through firmware-sized chunks so device transfer limits cannot invalidate a verified System Generation.
// ------------------=
static EFI_STATUS read_blocks_bounded(
    EFI_BLOCK_IO_PROTOCOL *block,
    uint64_t first_lba,
    size_t byte_count,
    void *destination
) {
    const size_t maximum_transfer = 1024 * 1024;
    uint8_t *output = destination;
    while (byte_count != 0) {
        size_t transfer = byte_count > maximum_transfer ? maximum_transfer : byte_count;
        EFI_STATUS status = block->read_blocks(
            block,
            block->media->media_id,
            first_lba,
            transfer,
            output
        );
        if (status != EFI_SUCCESS) return status;
        first_lba += transfer / 512;
        output += transfer;
        byte_count -= transfer;
    }
    return EFI_SUCCESS;
}

// ------------------------=
// FUNC: installed_kernel_size_valid
// DESC: Bounds installed ELF payloads before allocation, matching the native generation reservation.
// ------------------=
static int installed_kernel_size_valid(uint64_t bytes) {
    return bytes >= sizeof(Elf64Header) && bytes <= UINT64_C(1024) * 1024 * 1024;
}

// ------------------------=
// FUNC: allocate_native_buffer
// DESC: Allocates verified high RAM within an explicit address ceiling, avoiding fixed low kernel ranges.
// ------------------=
static void *allocate_native_buffer(EFI_BOOT_SERVICES *boot, size_t bytes, uint64_t maximum) {
    if (!bytes || bytes > SIZE_MAX - PAGE_MASK) return NULL;
    size_t pages = (bytes + PAGE_MASK) / PAGE_SIZE;
    // Some firmware's MaxAddress policy still prefers low memory. Select a
    // conventional high-RAM descriptor explicitly before accepting that policy.
    static uint64_t map[4096];
    size_t map_bytes = sizeof(map), stride = 0;
    uint64_t key = 0, highest = 0; uint32_t version = 0;
    if (boot->get_memory_map && boot->get_memory_map(&map_bytes, map, &key, &stride, &version) == EFI_SUCCESS &&
        map_bytes <= sizeof(map) && stride >= 40 && map_bytes % stride == 0) {
        for (size_t at = 0; at < map_bytes; at += stride) {
            const uint8_t *entry = (const uint8_t *)map + at;
            uint32_t type; uint64_t start, count;
            memcpy(&type, entry, 4); memcpy(&start, entry + 8, 8); memcpy(&count, entry + 24, 8);
            if (type != 7 || count < pages || count > (UINT64_MAX-start)/PAGE_SIZE) continue;
            uint64_t end = start + count*PAGE_SIZE;
            uint64_t limit = maximum & ~(uint64_t)PAGE_MASK;
            if (limit <= UINT64_MAX-PAGE_SIZE) limit += PAGE_SIZE;
            if (end > limit) end = limit;
            if (end <= start || (end-start)/PAGE_SIZE < pages) continue;
            uint64_t candidate = end - pages*PAGE_SIZE;
            if (candidate >= UINT64_C(0x100000000) && candidate > highest) highest = candidate;
        }
    }
    if (highest && boot->allocate_pages(EFI_ALLOCATE_ADDRESS, EFI_LOADER_DATA, pages, &highest) == EFI_SUCCESS)
        return (void *)(uintptr_t)highest;
    uint64_t address = maximum;
    if (boot->allocate_pages(EFI_ALLOCATE_MAX_ADDRESS, EFI_LOADER_DATA, pages, &address) != EFI_SUCCESS)
        return NULL;
    return (void *)(uintptr_t)address;
}

// ------------------------=
// FUNC: allocate_kernel_image
// DESC: Stages ELF bytes outside fixed low kernel ranges while firmware mappings remain active.
// ------------------=
static void *allocate_kernel_image(EFI_BOOT_SERVICES *boot, size_t bytes) {
    return allocate_native_buffer(boot, bytes, UINT64_MAX);
}

// ------------------------=
// FUNC: free_kernel_image
// DESC: Releases the page-backed staging image after validation failure or successful ELF loading.
// ------------------=
static void free_kernel_image(EFI_BOOT_SERVICES *boot, void *image, size_t bytes) {
    if (image && bytes && bytes <= SIZE_MAX - PAGE_MASK)
        boot->free_pages((uint64_t)(uintptr_t)image, (bytes + PAGE_MASK) / PAGE_SIZE);
}

// ------------------------=
// FUNC: try_load_installed_kernel
// DESC: Reads try load installed kernel from firmware or device state.
// ------------------=
static void *try_load_installed_kernel(EFI_SYSTEM_TABLE *system, size_t *file_size) {
    EFI_BOOT_SERVICES *boot = system->boot_services;
    EFI_GUID guid = block_io_guid;
    EFI_HANDLE *handles = NULL;
    size_t handle_count = 0;
    if (!boot->locate_handle_buffer || boot->locate_handle_buffer(EFI_BY_PROTOCOL, &guid, NULL,
            &handle_count, &handles) != EFI_SUCCESS) return NULL;
    for (size_t handle_index = 0; handle_index < handle_count; ++handle_index) {
        EFI_BLOCK_IO_PROTOCOL *block = NULL;
        guid = block_io_guid;
        if (boot->handle_protocol(handles[handle_index], &guid, (void **)&block) != EFI_SUCCESS ||
            !block || !block->media || !block->media->media_present || block->media->logical_partition ||
            block->media->block_size != 512 || !block->read_blocks) continue;
        uint8_t sector[512];
        if (block->read_blocks(block, block->media->media_id, 1, sizeof(sector), sector) != EFI_SUCCESS ||
            !equal_bytes(sector, (const uint8_t *)"EFI PART", 8)) continue;
        if (installed_generation_stage < 1) installed_generation_stage = 1;
        uint32_t header_size = read_u32(sector + 12);
        if (header_size < 92 || header_size > 512 || !valid_sector_record(sector, header_size, 16)) continue;
        uint64_t entries_lba = read_u64(sector + 72);
        uint32_t entry_count = read_u32(sector + 80), entry_size = read_u32(sector + 84);
        if (entry_count != 128 || entry_size != 128) continue;
        uint64_t container_lba = 0;
        for (uint32_t entry_sector = 0; entry_sector < 32 && container_lba == 0; ++entry_sector) {
            if (block->read_blocks(block, block->media->media_id, entries_lba + entry_sector,
                    sizeof(sector), sector) != EFI_SUCCESS) break;
            for (unsigned entry = 0; entry < 4; ++entry) {
                uint8_t *record = sector + entry * 128;
                if (equal_bytes(record, infinity_partition_type, 16)) {
                    container_lba = read_u64(record + 32); break;
                }
            }
        }
        if (!container_lba || block->read_blocks(block, block->media->media_id, container_lba,
                sizeof(sector), sector) != EFI_SUCCESS || !equal_bytes(sector, (const uint8_t *)"INFCONT1", 8) ||
                read_u32(sector + 8) != 1 || read_u32(sector + 16) != 4 ||
                !valid_sector_record(sector, 512, 508)) continue;
        installed_generation_invalid = 1;
        if (installed_generation_stage < 2) installed_generation_stage = 2;
        uint64_t pool_relative_lba = read_u64(sector + 32);
        uint64_t spaces_relative_lba = read_u64(sector + 40);
        uint64_t boot_catalog_relative_lba = read_u64(sector + 96);
        uint8_t container_uuid[16], pool_uuid[16];
        memcpy(container_uuid, sector + 64, 16); memcpy(pool_uuid, sector + 80, 16);
        if (block->read_blocks(block, block->media->media_id, container_lba + pool_relative_lba,
                sizeof(sector), sector) != EFI_SUCCESS || !equal_bytes(sector, (const uint8_t *)"INFPOOL1", 8) ||
                read_u32(sector + 8) != 1 || !valid_sector_record(sector, 512, 508) ||
                !equal_bytes(sector + 16, pool_uuid, 16) || !equal_bytes(sector + 32, container_uuid, 16)) continue;
        if (installed_generation_stage < 3) installed_generation_stage = 3;
        if (block->read_blocks(block, block->media->media_id, container_lba + spaces_relative_lba,
                sizeof(sector), sector) != EFI_SUCCESS || !equal_bytes(sector, (const uint8_t *)"INFSPACE", 8) ||
                read_u32(sector + 8) != 1 || read_u32(sector + 16) != 4 ||
                !valid_sector_record(sector, 512, 508)) continue;
        if (installed_generation_stage < 4) installed_generation_stage = 4;
        uint64_t system_boot_relative_lba = 0, system_boot_bytes = 0;
        for (unsigned space = 0; space < 4; ++space) {
            uint8_t *entry = sector + 32 + space * 96;
            if (read_u32(entry + 32) == 1 && read_u32(entry + 56) == 1) {
                system_boot_relative_lba = read_u64(entry + 64);
                system_boot_bytes = read_u64(entry + 72);
                break;
            }
        }
        if (boot_catalog_relative_lba != 3 || system_boot_relative_lba != boot_catalog_relative_lba ||
            system_boot_bytes != 512 || block->read_blocks(block, block->media->media_id,
                container_lba + boot_catalog_relative_lba, sizeof(sector), sector) != EFI_SUCCESS ||
            !equal_bytes(sector, (const uint8_t *)"INFBOOT1", 8) || read_u32(sector + 8) != 1 ||
            read_u32(sector + 12) != 512 || read_u32(sector + 16) != 1 ||
            read_u32(sector + 20) != INFINITY_ARCHITECTURE || read_u64(sector + 24) == 0 ||
            read_u32(sector + 56) == 0 || read_u32(sector + 56) > 8 ||
            !equal_bytes(sector + 64, container_uuid, 16) || !valid_sector_record(sector, 512, 508)) continue;
        if (installed_generation_stage < 5) installed_generation_stage = 5;
        uint64_t active_generation = read_u64(sector + 24);
        uint64_t manifest_relative_lba = read_u64(sector + 32);
        if (manifest_relative_lba < 4 || manifest_relative_lba >= 2048 ||
            block->read_blocks(block, block->media->media_id, container_lba + manifest_relative_lba,
                sizeof(sector), sector) != EFI_SUCCESS || !equal_bytes(sector, (const uint8_t *)"INFSYSM1", 8) ||
            read_u32(sector + 8) != 1 || read_u32(sector + 12) != 512 || read_u32(sector + 16) != 3 ||
            read_u32(sector + 20) != INFINITY_ARCHITECTURE || read_u64(sector + 24) != active_generation ||
            read_u32(sector + 60) == 0 || read_u32(sector + 60) > 16 ||
            !equal_bytes(sector + 80, container_uuid, 16) || !valid_sector_record(sector, 512, 508)) continue;
        if (installed_generation_stage < 6) installed_generation_stage = 6;
        uint64_t kernel_relative_lba = read_u64(sector + 40), kernel_bytes = read_u64(sector + 48);
        uint32_t kernel_crc = read_u32(sector + 56), component_count = read_u32(sector + 60);
        uint64_t components_relative_lba = read_u64(sector + 64);
        uint64_t component_words[128];
        uint8_t *components = (uint8_t *)component_words;
        if (kernel_relative_lba < 2048 || components_relative_lba < 5 || components_relative_lba >= 2048 ||
            component_count == 0 || component_count > 20 ||
            block->read_blocks(block, block->media->media_id, container_lba + components_relative_lba,
                sizeof(component_words), components) != EFI_SUCCESS ||
            !equal_bytes(components, (const uint8_t *)"INFCOMP1", 8) ||
            read_u32(components + 8) != 2 || read_u32(components + 12) != sizeof(component_words) ||
            read_u32(components + 16) != component_count || read_u32(components + 20) != 48 ||
            read_u32(components + 24) != 2 || !valid_sector_record(components, sizeof(component_words), 1020)) continue;
        if (installed_generation_stage < 7) installed_generation_stage = 7;
        uint8_t kernel_declared = 0, core_valid = 1;
        for (uint32_t component = 0; component < component_count; ++component) {
            uint8_t *entry = components + 32 + component * 48;
            if (read_u32(entry + 16) != 1 || !(read_u32(entry + 20) & 1)) { core_valid = 0; break; }
            uint32_t component_arch = read_u32(entry + 12);
            if (component_arch != 0 && component_arch != INFINITY_ARCHITECTURE) { core_valid = 0; break; }
            if (read_u32(entry) == 1 && read_u64(entry + 32) == kernel_relative_lba &&
                read_u64(entry + 40) == kernel_bytes && read_u32(entry + 24) == kernel_crc) kernel_declared = 1;
        }
        if (!core_valid || !kernel_declared) continue;
        if (installed_generation_stage < 8) installed_generation_stage = 8;
        /* The installed image embeds the native System Generation payload and
         * can legitimately exceed 64 MiB as new CORE services are added. Keep
         * the bound explicit, but large enough for the architecture-neutral
         * installed image assembled by the current build. */
        if (!installed_kernel_size_valid(kernel_bytes)) continue;
        size_t transfer_size = (size_t)((kernel_bytes + 511) & ~UINT64_C(511));
        void *buffer = allocate_kernel_image(boot, transfer_size);
        if (!buffer) continue;
        if (read_blocks_bounded(block, container_lba + kernel_relative_lba,
                transfer_size, buffer) != EFI_SUCCESS) { free_kernel_image(boot, buffer, transfer_size); continue; }
        if (installed_generation_stage < 9) installed_generation_stage = 9;
        if (crc32_bytes(buffer, (size_t)kernel_bytes) != kernel_crc) { free_kernel_image(boot, buffer, transfer_size); continue; }
        installed_generation_stage = 10;
        *file_size = (size_t)kernel_bytes;
        installed_generation_invalid = 0;
        boot->free_pool(handles);
        serial_write("InfinityOS Native Boot\nBoot source: installed system\n"
            "Container           found\nPool                online\nSystem Space        online\n"
            "Generation          1\nGeneration state    ACTIVE\nSystem manifest     valid\nKernel              valid\n"
            "[BOOT] Infinity container located\n[BOOT] Infinity pool located\n[BOOT] System Space online\n"
            "[BOOT] Generation 1 ACTIVE\n[BOOT] System manifest valid\n[BOOT] Kernel valid\nStarting InfinityOS...\n");
        return buffer;
    }
    boot->free_pool(handles);
    return NULL;
}

// ------------------------=
// FUNC: fail
// DESC: Stops boot after reporting an unrecoverable error.
// ------------------=
__attribute__((noreturn)) static void fail(EFI_SYSTEM_TABLE *system, const CHAR16 *wide, const char *serial) {
    firmware_write(system, wide);
    serial_write(serial);
    for (;;) {
#if defined(INFINITY_AARCH64)
        __asm__ volatile ("msr daifset, #0xf; wfe");
#else
        __asm__ volatile ("cli; hlt");
#endif
    }
}

// ------------------------=
// FUNC: load_kernel_file
// DESC: Reads load kernel file from firmware or device state.
// ------------------=
static void *load_kernel_file(EFI_HANDLE image, EFI_SYSTEM_TABLE *system, size_t *file_size) {
    EFI_BOOT_SERVICES *boot = system->boot_services;
    EFI_LOADED_IMAGE_PROTOCOL *loaded = NULL;
    EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *filesystem = NULL;
    EFI_FILE_PROTOCOL *root = NULL;
    EFI_FILE_PROTOCOL *file = NULL;
    EFI_GUID guid = loaded_image_guid;
    if (boot->handle_protocol(image, &guid, (void **)&loaded) != EFI_SUCCESS)
        fail(system, L"ERROR: loaded-image protocol unavailable\r\n", "ERROR: loaded-image protocol unavailable\n");
    guid = simple_fs_guid;
    if (boot->handle_protocol(loaded->device_handle, &guid, (void **)&filesystem) != EFI_SUCCESS ||
        filesystem->open_volume(filesystem, &root) != EFI_SUCCESS)
        fail(system, L"ERROR: boot filesystem unavailable\r\n", "ERROR: boot filesystem unavailable\n");
    CHAR16 path[] = L"\\EFI\\INFINITY\\KERNEL.ELF";
    if (root->open(root, &file, path, EFI_FILE_MODE_READ, 0) != EFI_SUCCESS && installed_generation_invalid)
        fail(system, L"InfinityOS boot failed\r\nNo valid ACTIVE system generation found.\r\nBoot recovery media to repair InfinityOS.\r\n",
            "InfinityOS boot failed\nNo valid ACTIVE system generation found.\nBoot recovery media to repair InfinityOS.\n");
    if (!file)
        fail(system, L"ERROR: kernel image not found\r\n", "ERROR: kernel image not found\n");

    size_t info_size = 0;
    guid = file_info_guid;
    EFI_STATUS status = file->get_info(file, &guid, &info_size, NULL);
    if (status != EFI_BUFFER_TOO_SMALL || info_size < 24)
        fail(system, L"ERROR: kernel metadata unavailable\r\n", "ERROR: kernel metadata unavailable\n");
    void *info = NULL;
    if (boot->allocate_pool(EFI_LOADER_DATA, info_size, &info) != EFI_SUCCESS ||
        file->get_info(file, &guid, &info_size, info) != EFI_SUCCESS)
        fail(system, L"ERROR: kernel metadata allocation failed\r\n", "ERROR: kernel metadata allocation failed\n");
    *file_size = (size_t)((uint64_t *)info)[1];
    boot->free_pool(info);

    void *buffer = *file_size < sizeof(Elf64Header) ? NULL : allocate_kernel_image(boot, *file_size);
    if (!buffer)
        fail(system, L"ERROR: kernel buffer allocation failed\r\n", "ERROR: kernel buffer allocation failed\n");
    size_t read_size = *file_size;
    if (file->read(file, &read_size, buffer) != EFI_SUCCESS || read_size != *file_size)
        fail(system, L"ERROR: kernel read failed\r\n", "ERROR: kernel read failed\n");
    file->close(file);
    root->close(root);
    return buffer;
}

// ------------------------=
// FUNC: boot_media_has_kernel
// DESC: Keeps installer and recovery media paired with their own kernel instead of an older installed generation.
// ------------------=
static uint8_t boot_media_has_kernel(EFI_HANDLE image, EFI_SYSTEM_TABLE *system) {
    EFI_LOADED_IMAGE_PROTOCOL *loaded = NULL;
    EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *filesystem = NULL;
    EFI_FILE_PROTOCOL *root = NULL, *file = NULL;
    EFI_GUID guid = loaded_image_guid;
    if (system->boot_services->handle_protocol(image, &guid, (void **)&loaded) != EFI_SUCCESS) return 0;
    guid = simple_fs_guid;
    if (system->boot_services->handle_protocol(loaded->device_handle, &guid, (void **)&filesystem) != EFI_SUCCESS ||
        filesystem->open_volume(filesystem, &root) != EFI_SUCCESS) return 0;
    CHAR16 path[] = L"\\EFI\\INFINITY\\KERNEL.ELF";
    uint8_t present = root->open(root, &file, path, EFI_FILE_MODE_READ, 0) == EFI_SUCCESS && file != NULL;
    if (file) file->close(file);
    root->close(root);
    return present;
}

// ------------------------=
// FUNC: reserve_kernel_range
// DESC: Reserves only conventional RAM across firmware descriptor boundaries, rolling back partial reservations.
// ------------------=
static int reserve_kernel_range(EFI_BOOT_SERVICES *boot, uint64_t low, uint64_t high) {
    if (high <= low || (low & PAGE_MASK) || (high & PAGE_MASK)) return 0;
    uint64_t address = low;
    if (boot->allocate_pages(EFI_ALLOCATE_ADDRESS, EFI_LOADER_DATA,
            (size_t)((high-low)/PAGE_SIZE), &address) == EFI_SUCCESS) {
        if (address == low) return 1;
        if (boot->free_pages) boot->free_pages(address, (size_t)((high-low)/PAGE_SIZE));
        return 0;
    }
    if (!boot->get_memory_map || !boot->free_pages) return 0;
    /* Firmware entry is single-threaded. Keep bounded scratch in loader BSS,
       avoiding a platform stack-probe runtime dependency before kernel entry. */
    static uint64_t map[4096], reserved[128][2];
    uint64_t key = 0;
    size_t bytes = sizeof(map), stride = 0, count = 0;
    uint32_t version = 0;
    if (boot->get_memory_map(&bytes, map, &key, &stride, &version) != EFI_SUCCESS
        || bytes > sizeof(map) || stride < 40 || bytes % stride) return 0;
    uint64_t cursor = low;
    while (cursor < high && count < 128) {
        uint64_t end = cursor;
        for (size_t at = 0; at < bytes; at += stride) {
            const uint8_t *entry = (const uint8_t *)map + at;
            uint32_t type; uint64_t start, pages;
            memcpy(&type, entry, 4); memcpy(&start, entry+8, 8); memcpy(&pages, entry+24, 8);
            if (type != 7 || pages > (UINT64_MAX-start)/PAGE_SIZE) continue;
            uint64_t limit = start + pages*PAGE_SIZE;
            if (start <= cursor && cursor < limit) {end = limit < high ? limit : high; break;}
        }
        if (end == cursor || (end & PAGE_MASK)) break;
        address = cursor;
        size_t pages = (size_t)((end-cursor)/PAGE_SIZE);
        if (boot->allocate_pages(EFI_ALLOCATE_ADDRESS, EFI_LOADER_DATA, pages, &address) != EFI_SUCCESS) break;
        if (address != cursor) {boot->free_pages(address, pages); break;}
        reserved[count][0] = cursor; reserved[count++][1] = pages;
        cursor = end;
    }
    if (cursor == high) return 1;
    while (count) {--count; boot->free_pages(reserved[count][0], (size_t)reserved[count][1]);}
    return 0;
}

// ------------------------=
// FUNC: report_kernel_memory_conflict
// DESC: Emits bounded boot-only allocation diagnostics without changing memory ownership or normal runtime logging.
// ------------------=
static void report_kernel_memory_conflict(EFI_BOOT_SERVICES *boot, uint64_t low, uint64_t high) {
    static uint64_t map[4096];
    size_t bytes = sizeof(map), stride = 0;
    uint64_t key = 0; uint32_t version = 0;
    if (boot->get_memory_map(&bytes, map, &key, &stride, &version) != EFI_SUCCESS ||
        bytes > sizeof(map) || stride < 40 || bytes % stride) return;
    for (size_t at = 0; at < bytes; at += stride) {
        const uint8_t *entry = (const uint8_t *)map + at;
        uint32_t type; uint64_t start, pages;
        memcpy(&type, entry, 4); memcpy(&start, entry + 8, 8); memcpy(&pages, entry + 24, 8);
        if (pages > (UINT64_MAX-start)/PAGE_SIZE || start >= high || start + pages*PAGE_SIZE <= low) continue;
        uint64_t values[3] = {type, start, pages*PAGE_SIZE};
        serial_write("[BOOT] kernel memory type/start/bytes ");
        for (unsigned field = 0; field < 3; ++field) {
            char text[18];
            for (unsigned digit = 0; digit < 16; ++digit)
                text[digit] = "0123456789abcdef"[(values[field] >> ((15-digit)*4)) & 15];
            text[16] = field == 2 ? '\n' : ' '; text[17] = 0; serial_write(text);
        }
    }
}

// ------------------------=
// FUNC: load_elf
// DESC: Reads load elf from firmware or device state.
// ------------------=
static InfinityLoadedKernel load_elf(EFI_SYSTEM_TABLE *system, const void *image, size_t image_size) {
    EFI_BOOT_SERVICES *boot = system->boot_services;
    const Elf64Header *header = image;
    if (header->ident[0] != 0x7f || header->ident[1] != 'E' || header->ident[2] != 'L' ||
        header->ident[3] != 'F' || header->ident[4] != 2 || header->ident[5] != 1 ||
        header->machine != INFINITY_ELF_MACHINE || header->phentsize != sizeof(Elf64ProgramHeader) ||
        header->phoff + (uint64_t)header->phnum * header->phentsize > image_size)
        fail(system, L"ERROR: unsupported kernel format\r\n", "ERROR: unsupported kernel format\n");

    const Elf64ProgramHeader *segments = (const void *)((const uint8_t *)image + header->phoff);
    uint64_t low = UINT64_MAX, high = 0;
    InfinityLoadedKernel loaded = {0, UINT64_MAX, 0, UINT64_MAX, 0,
        UINT64_MAX, 0, UINT64_MAX, 0};
    for (uint16_t i = 0; i < header->phnum; ++i) {
        if (segments[i].type != PT_LOAD) continue;
        if (segments[i].filesz > segments[i].memsz || segments[i].offset + segments[i].filesz > image_size)
            fail(system, L"ERROR: malformed kernel segment\r\n", "ERROR: malformed kernel segment\n");
        uint64_t start = segments[i].paddr & ~(uint64_t)PAGE_MASK;
        uint64_t end = (segments[i].paddr + segments[i].memsz + PAGE_MASK) & ~(uint64_t)PAGE_MASK;
        if (start < low) low = start;
        if (end > high) high = end;
        uint64_t *kind_low = segments[i].flags & 1 ? &loaded.text_low :
            (segments[i].flags & 2 ? &loaded.data_low : &loaded.rodata_low);
        uint64_t *kind_high = segments[i].flags & 1 ? &loaded.text_high :
            (segments[i].flags & 2 ? &loaded.data_high : &loaded.rodata_high);
        if (start < *kind_low) *kind_low = start;
        if (end > *kind_high) *kind_high = end;
    }
    if (low == UINT64_MAX || high <= low || header->entry < low || header->entry >= high)
        fail(system, L"ERROR: kernel has no loadable entry\r\n", "ERROR: kernel has no loadable entry\n");
    if (!reserve_kernel_range(boot, low, high)) {
        report_kernel_memory_conflict(boot, low, high);
        fail(system, L"ERROR: unable to allocate kernel pages\r\n", "ERROR: unable to allocate kernel pages\n");
    }
    memset((void *)(uintptr_t)low, 0, (size_t)(high - low));
    for (uint16_t i = 0; i < header->phnum; ++i) {
        if (segments[i].type == PT_LOAD)
            memcpy((void *)(uintptr_t)segments[i].paddr, (const uint8_t *)image + segments[i].offset,
                   (size_t)segments[i].filesz);
    }
    loaded.entry = header->entry;
    loaded.low = low;
    loaded.high = high;
    if (loaded.text_low == UINT64_MAX) loaded.text_low = 0;
    if (loaded.rodata_low == UINT64_MAX) loaded.rodata_low = 0;
    if (loaded.data_low == UINT64_MAX) loaded.data_low = 0;
    return loaded;
}

// ------------------------=
// FUNC: reserve_native_runtime_pool
// DESC: Reserves a contiguous identity-mapped frame arena for isolated native process images, heaps, stacks, and page tables.
// ------------------=
static uint64_t reserve_native_runtime_pool(EFI_SYSTEM_TABLE *system) {
#if defined(INFINITY_AARCH64)
    (void)system;
    return 0;
#else
    uint64_t base = (uint64_t)(uintptr_t)allocate_native_buffer(system->boot_services,
        (size_t)NATIVE_RUNTIME_POOL_PAGES * PAGE_SIZE, (uint64_t)IDENTITY_MAP_GIB * 1024 * 1024 * 1024 - 1);
    if (!base)
        fail(system, L"ERROR: native runtime pool allocation failed\r\n",
            "ERROR: native runtime pool allocation failed\n");
    return base;
#endif
}

// ------------------------=
// FUNC: prepare_identity_map
// DESC: Implements the prepare identity map loader operation.
// ------------------=
static uint64_t prepare_identity_map(EFI_SYSTEM_TABLE *system) {
#if defined(INFINITY_AARCH64)
    /* UEFI's identity mappings remain active for this first handoff. */
    (void)system;
    return 0;
#else
    uint64_t base = UINT32_MAX;
    if (system->boot_services->allocate_pages(EFI_ALLOCATE_MAX_ADDRESS, EFI_LOADER_DATA,
            2 + IDENTITY_MAP_GIB, &base) != EFI_SUCCESS)
        fail(system, L"ERROR: page-table allocation failed\r\n", "ERROR: page-table allocation failed\n");
    memset((void *)(uintptr_t)base, 0, (2 + IDENTITY_MAP_GIB) * PAGE_SIZE);
    uint64_t *pml4 = (void *)(uintptr_t)base;
    uint64_t *pdpt = (void *)(uintptr_t)(base + PAGE_SIZE);
    pml4[0] = (base + PAGE_SIZE) | 3;
    for (uint64_t group = 0; group < IDENTITY_MAP_GIB; ++group) {
        uint64_t pd_address = base + (2 + group) * PAGE_SIZE;
        pdpt[group] = pd_address | 3;
        uint64_t *pd = (void *)(uintptr_t)pd_address;
        for (uint64_t page = 0; page < 512; ++page)
            pd[page] = (group * UINT64_C(0x40000000) + page * UINT64_C(0x200000)) | UINT64_C(0x83);
    }
    return base;
#endif
}

// ------------------------=
// FUNC: gather_framebuffer
// DESC: Reads gather framebuffer from firmware or device state.
// ------------------=
static void gather_framebuffer(EFI_SYSTEM_TABLE *system, InfinityBootInfo *info) {
    EFI_GRAPHICS_OUTPUT_PROTOCOL *graphics = NULL;
    EFI_GUID guid = graphics_output_guid;
    if (system->boot_services->locate_protocol(&guid, NULL, (void **)&graphics) != EFI_SUCCESS ||
        !graphics || !graphics->mode) return;
    if (!graphics->mode->info && graphics->set_mode)
        graphics->set_mode(graphics, 0);
    EFI_GRAPHICS_OUTPUT_PROTOCOL_MODE *mode = graphics->mode;
    if (!mode || !mode->info) return;
    uint32_t best_mode = mode->mode;
    uint64_t best_score = infinity_video_score(mode->info->horizontal_resolution,
        mode->info->vertical_resolution, mode->info->pixels_per_scan_line,
        mode->info->pixel_format, mode->info->pixel_information);
    if (graphics->query_mode && graphics->set_mode) {
        for (uint32_t candidate = 0; candidate < mode->max_mode && candidate < 4096; ++candidate) {
            EFI_GRAPHICS_OUTPUT_MODE_INFORMATION *candidate_info = NULL;
            size_t candidate_size = 0;
            EFI_STATUS status = graphics->query_mode(graphics, candidate, &candidate_size, &candidate_info);
            if (status == EFI_SUCCESS && candidate_info && candidate_size >= sizeof(*candidate_info)) {
                uint64_t score = infinity_video_score(candidate_info->horizontal_resolution,
                    candidate_info->vertical_resolution, candidate_info->pixels_per_scan_line,
                    candidate_info->pixel_format, candidate_info->pixel_information);
                if (score > best_score) { best_score = score; best_mode = candidate; }
            }
            if (candidate_info) system->boot_services->free_pool(candidate_info);
        }
        if (best_mode != mode->mode) graphics->set_mode(graphics, best_mode);
    }
    mode = graphics->mode;
    if (!mode || !mode->info || !infinity_video_score(mode->info->horizontal_resolution,
        mode->info->vertical_resolution, mode->info->pixels_per_scan_line,
        mode->info->pixel_format, mode->info->pixel_information) ||
        !infinity_video_memory_valid(mode->framebuffer_base, mode->framebuffer_size,
            mode->info->pixels_per_scan_line, mode->info->vertical_resolution)) return;
#if !defined(INFINITY_AARCH64)
    /* The current x86-64 bootstrap identity map covers the first 4 GiB. */
    if (mode->framebuffer_base + mode->framebuffer_size > UINT64_C(0x100000000)) return;
#endif
    info->framebuffer_address = mode->framebuffer_base;
    info->framebuffer_size = mode->framebuffer_size;
    info->framebuffer_width = mode->info->horizontal_resolution;
    info->framebuffer_height = mode->info->vertical_resolution;
    info->framebuffer_stride = mode->info->pixels_per_scan_line;
    info->framebuffer_format = infinity_video_format(mode->info->pixel_format, mode->info->pixel_information);
    serial_write("[BOOT] framebuffer ready\n");
}

// ------------------------=
// FUNC: starts_with_edk
// DESC: Validates starts with edk.
// ------------------=
static uint8_t starts_with_edk(const CHAR16 *vendor) {
    return vendor && vendor[0] == 'E' && vendor[1] == 'D' && vendor[2] == 'K';
}

#if defined(INFINITY_AARCH64)
// ------------------------=
// FUNC: acpi_serial_base
// DESC: Reads the firmware-described serial MMIO base without relying on the host operating system.
// ------------------=
static uint64_t acpi_serial_base(EFI_SYSTEM_TABLE *system) {
    if (!system || !system->configuration_table) return 0;
    const uint8_t rsdp_signature[8] = {'R','S','D',' ','P','T','R',' '};
    const uint8_t spcr_signature[4] = {'S','P','C','R'};
    for (size_t table_index = 0; table_index < system->number_of_table_entries; ++table_index) {
        EFI_CONFIGURATION_TABLE *entry = &system->configuration_table[table_index];
        if (!equal_bytes((const uint8_t *)&entry->vendor_guid,
                (const uint8_t *)&acpi20_table_guid, sizeof(EFI_GUID)) || !entry->vendor_table) continue;
        const uint8_t *rsdp = entry->vendor_table;
        if (!equal_bytes(rsdp, rsdp_signature, sizeof(rsdp_signature)) || rsdp[15] < 2) continue;
        const uint8_t *xsdt = (const uint8_t *)(uintptr_t)read_u64(rsdp + 24);
        if (!xsdt || read_u32(xsdt + 4) < 36u) continue;
        size_t count = (read_u32(xsdt + 4) - 36u) / 8u;
        for (size_t index = 0; index < count; ++index) {
            const uint8_t *table = (const uint8_t *)(uintptr_t)read_u64(xsdt + 36u + index * 8u);
            if (!table || !equal_bytes(table, spcr_signature, sizeof(spcr_signature)) ||
                    read_u32(table + 4) < 52u || table[40] != 0u) continue;
            return read_u64(table + 44u);
        }
    }
    return 0;
}
#endif

#if defined(INFINITY_AARCH64)
// ------------------------=
// FUNC: usb_mouse_report
// DESC: Copies the latest asynchronous boot-mouse report into the kernel handoff buffer.
// ------------------=
static EFI_STATUS EFIAPI usb_mouse_report(void *data, size_t length, void *context,
                                           uint32_t status) {
    InfinityFirmwarePointers *set = (InfinityFirmwarePointers *)context;
    if (!set || status != 0 || !data || length < 3) return EFI_SUCCESS;
    size_t count = length < sizeof(set->usb_mouse_report) ? length : sizeof(set->usb_mouse_report);
    const uint8_t *bytes = (const uint8_t *)data;
    for (size_t index = 0; index < count; ++index) set->usb_mouse_report[index] = bytes[index];
    set->usb_mouse_length = (uint32_t)count;
    set->usb_mouse_sequence++;
    return EFI_SUCCESS;
}

// ------------------------=
// FUNC: gather_firmware_pointers
// DESC: Reads gather firmware pointers from firmware or device state.
// ------------------=
static InfinityFirmwarePointers *gather_firmware_pointers(EFI_SYSTEM_TABLE *system) {
    InfinityFirmwarePointers *set = NULL;
    if (system->boot_services->allocate_pool(EFI_LOADER_DATA, sizeof(*set),
                                             (void **)&set) != EFI_SUCCESS) return NULL;
    memset(set, 0, sizeof(*set));
    set->check_event = system->boot_services->unused_07_15[5];

    /* UEFI is permitted to connect only the boot path. Connect controllers
     * before locating input protocols: on VirtualBox ARM the xHCI controller
     * is not necessarily connected merely because the optical boot device is.
     * Enumerating first left the kernel with no mouse handles until reboot. */
    if (system->boot_services->connect_controller) {
        EFI_HANDLE *all_handles = NULL;
        size_t all_handle_count = 0;
        if (system->boot_services->locate_handle_buffer(EFI_ALL_HANDLES, NULL, NULL,
                &all_handle_count, &all_handles) == EFI_SUCCESS) {
            for (size_t index = 0; index < all_handle_count; ++index)
                system->boot_services->connect_controller(all_handles[index], NULL, NULL, 1);
            system->boot_services->free_pool(all_handles);
        }
    }

    EFI_GUID absolute_guid = {0x8d59d32b, 0xc655, 0x4ae9,
                              {0x9b, 0x15, 0xf2, 0x59, 0x04, 0x99, 0x2a, 0x43}};
    EFI_GUID relative_guid = {0x31878c87, 0x0b75, 0x11d5,
                              {0x9a, 0x4f, 0x00, 0x90, 0x27, 0x3f, 0xc1, 0x4d}};
    EFI_GUID *guids[2] = {&absolute_guid, &relative_guid};
    for (uint32_t kind = 0; kind < 2; ++kind) {
        EFI_HANDLE *handles = NULL;
        size_t handle_count = 0;
        EFI_GUID guid = *guids[kind];
        if (!system->boot_services->locate_handle_buffer ||
            system->boot_services->locate_handle_buffer(EFI_BY_PROTOCOL, &guid, NULL,
                &handle_count, &handles) != EFI_SUCCESS) continue;
        for (size_t index = 0; index < handle_count; ++index) {
            void *protocol = NULL;
            guid = *guids[kind];
            if (system->boot_services->handle_protocol(handles[index], &guid, &protocol) != EFI_SUCCESS ||
                !protocol) continue;
            if (kind == 0 && set->absolute_count < INFINITY_POINTER_PROTOCOLS)
                set->absolute[set->absolute_count++] = protocol;
            if (kind == 1 && set->relative_count < INFINITY_POINTER_PROTOCOLS)
                set->relative[set->relative_count++] = protocol;
        }
        system->boot_services->free_pool(handles);
    }
    EFI_GUID usb_io_guid = {0x2b2f68d6, 0x0cd2, 0x44cf,
                            {0x8e, 0x8b, 0xbb, 0xa2, 0x0b, 0x1b, 0x5b, 0x75}};
    EFI_HANDLE *usb_handles = NULL;
    size_t usb_handle_count = 0;
    if (system->boot_services->locate_handle_buffer &&
        system->boot_services->locate_handle_buffer(EFI_BY_PROTOCOL, &usb_io_guid, NULL,
            &usb_handle_count, &usb_handles) == EFI_SUCCESS) {
        for (size_t index = 0; index < usb_handle_count; ++index) {
            EFI_USB_IO_PROTOCOL *usb = NULL;
            EFI_GUID guid = usb_io_guid;
            if (system->boot_services->handle_protocol(usb_handles[index], &guid,
                    (void **)&usb) != EFI_SUCCESS || !usb || !usb->get_interface_descriptor ||
                    !usb->get_endpoint_descriptor) continue;
            EFI_USB_INTERFACE_DESCRIPTOR interface;
            if (usb->get_interface_descriptor(usb, &interface) != EFI_SUCCESS ||
                interface.interface_class != 3 ||
                (interface.interface_protocol != 0 && interface.interface_protocol != 1 &&
                 interface.interface_protocol != 2)) continue;
            if (usb->control_transfer && interface.interface_protocol != 0) {
                uint32_t usb_status = 0;
                EFI_USB_DEVICE_REQUEST boot_protocol =
                    {0x21, 0x0b, 0, interface.interface_number, 0};
                usb->control_transfer(usb, &boot_protocol, 2, 100, NULL, 0, &usb_status);
            }
            for (uint8_t endpoint_index = 0; endpoint_index < interface.num_endpoints; ++endpoint_index) {
                EFI_USB_ENDPOINT_DESCRIPTOR endpoint;
                if (usb->get_endpoint_descriptor(usb, endpoint_index, &endpoint) == EFI_SUCCESS &&
                    (endpoint.endpoint_address & 0x80) && (endpoint.attributes & 3) == 3) {
                    if ((interface.interface_protocol == 0 || interface.interface_protocol == 2) &&
                        set->usb_mouse_count < INFINITY_POINTER_PROTOCOLS) {
                        uint32_t slot = set->usb_mouse_count++;
                        set->usb_mouse[slot] = usb;
                        set->usb_endpoint[slot] = endpoint.endpoint_address;
                        set->usb_mouse_absolute[slot] = interface.interface_protocol == 0;
                        /* Protocol-zero HID tablets carry absolute 16-bit axes.
                         * Keep them on the synchronous path so the kernel can
                         * decode the complete report instead of interpreting
                         * its low bytes as relative boot-mouse deltas. */
                        if (!set->usb_mouse_absolute[slot] && usb->async_interrupt_transfer) {
                            size_t packet_size = endpoint.max_packet_size;
                            if (packet_size > sizeof(set->usb_mouse_report))
                                packet_size = sizeof(set->usb_mouse_report);
                            if (usb->async_interrupt_transfer(usb, endpoint.endpoint_address,
                                    1, endpoint.interval, packet_size, usb_mouse_report,
                                    set) == EFI_SUCCESS)
                                set->usb_mouse_async = 1;
                        }
                    } else if (interface.interface_protocol == 1 &&
                        set->usb_keyboard_count < INFINITY_POINTER_PROTOCOLS) {
                        uint32_t slot = set->usb_keyboard_count++;
                        set->usb_keyboard[slot] = usb;
                        set->usb_keyboard_endpoint[slot] = endpoint.endpoint_address;
                    }
                    break;
                }
            }
        }
        system->boot_services->free_pool(usb_handles);
    }
    EFI_HANDLE *block_handles = NULL;
    size_t block_handle_count = 0;
    EFI_GUID block_guid = block_io_guid;
    if (system->boot_services->locate_handle_buffer &&
        system->boot_services->locate_handle_buffer(EFI_BY_PROTOCOL, &block_guid, NULL,
            &block_handle_count, &block_handles) == EFI_SUCCESS) {
        for (size_t index = 0; index < block_handle_count &&
                set->block_count < INFINITY_POINTER_PROTOCOLS; ++index) {
            EFI_BLOCK_IO_PROTOCOL *block = NULL;
            block_guid = block_io_guid;
            if (system->boot_services->handle_protocol(block_handles[index], &block_guid,
                    (void **)&block) != EFI_SUCCESS || !block || !block->media) continue;
            EFI_BLOCK_IO_MEDIA *media = block->media;
            if (!media->media_present || media->logical_partition || media->removable_media ||
                media->read_only || media->block_size != 512 || !block->read_blocks ||
                !block->write_blocks) continue;
            set->blocks[set->block_count++] = block;
        }
        system->boot_services->free_pool(block_handles);
    }
    if (set->absolute_count == 0 && set->relative_count == 0 && set->usb_mouse_count == 0 &&
        set->usb_keyboard_count == 0 && set->block_count == 0) {
        system->boot_services->free_pool(set);
        return NULL;
    }
    return set;
}
#endif

// ------------------------=
// FUNC: discover_acpi_network
// DESC: Finds an Ethernet-class PCI function through the ACPI MCFG table when firmware provides no network protocol.
// ------------------=
static uint8_t discover_acpi_network(EFI_SYSTEM_TABLE *system, InfinityBootInfo *info) {
    if (!system || !system->configuration_table) return 0;
    const uint8_t rsdp_signature[8] = {'R','S','D',' ','P','T','R',' '};
    const uint8_t mcfg_signature[4] = {'M','C','F','G'};
    for (size_t table_index = 0; table_index < system->number_of_table_entries; ++table_index) {
        EFI_CONFIGURATION_TABLE *entry = &system->configuration_table[table_index];
        if (!equal_bytes((const uint8_t *)&entry->vendor_guid,
                (const uint8_t *)&acpi20_table_guid, sizeof(EFI_GUID)) || !entry->vendor_table) continue;
        const uint8_t *rsdp = entry->vendor_table;
        if (!equal_bytes(rsdp, rsdp_signature, sizeof(rsdp_signature)) || rsdp[15] < 2) continue;
        const uint8_t *xsdt = (const uint8_t *)(uintptr_t)read_u64(rsdp + 24);
        if (!xsdt || read_u32(xsdt + 4) < 36u) continue;
        size_t xsdt_entries = (read_u32(xsdt + 4) - 36u) / 8u;
        for (size_t xsdt_index = 0; xsdt_index < xsdt_entries; ++xsdt_index) {
            const uint8_t *table = (const uint8_t *)(uintptr_t)read_u64(xsdt + 36u + xsdt_index * 8u);
            if (!table || !equal_bytes(table, mcfg_signature, sizeof(mcfg_signature)) ||
                read_u32(table + 4) < 60u) continue;
            size_t allocation_count = (read_u32(table + 4) - 44u) / 16u;
            for (size_t allocation = 0; allocation < allocation_count; ++allocation) {
                const uint8_t *descriptor = table + 44u + allocation * 16u;
                uint64_t ecam = read_u64(descriptor);
                uint8_t first_bus = descriptor[10];
                uint8_t last_bus = descriptor[11];
                for (uint32_t bus = first_bus; bus <= last_bus; ++bus) {
                    for (uint32_t device = 0; device < 32u; ++device) {
                        uintptr_t function = (uintptr_t)ecam +
                            ((uintptr_t)(bus - first_bus) << 20) + ((uintptr_t)device << 15);
                        uint32_t identity = *(volatile uint32_t *)function;
                        if ((identity & 0xffffu) == 0xffffu) continue;
                        uint32_t class_revision = *(volatile uint32_t *)(function + 8u);
                        if (((class_revision >> 24) & 0xffu) != 0x02u) continue;
                        info->firmware_network = UINT64_C(0x8000000000000000) |
                            ((uint64_t)(identity & 0xffffu) << 16) |
                            ((uint64_t)(identity >> 16) & 0xffffu);
                        info->network_device_count = 1;
                        info->network_link_state = 0u;
                        info->network_mtu = 0u;
                        info->network_capabilities = 0u;
                        info->network_mac_length = 0u;
                        info->network_reserved = 3u;
                        if (identity == UINT32_C(0x100e8086)) {
                            info->firmware_network = (uint64_t)function;
                            info->network_reserved = INFINITY_NETWORK_ECAM;
                        }
                        serial_write("[BOOT] ACPI PCI network adapter discovered\n");
                        return 1;
                    }
                }
            }
        }
    }
    return 0;
}

// ------------------------=
// FUNC: gather_firmware_network
// DESC: Discovers the first UEFI network adapter and copies observed link, MTU, MAC, and transport capability into the stable handoff.
// ------------------=
static void gather_firmware_network(EFI_SYSTEM_TABLE *system, InfinityBootInfo *info) {
    EFI_BOOT_SERVICES *boot = system->boot_services;
    if (!boot || !boot->locate_handle_buffer || !boot->handle_protocol) return;

    if (boot->connect_controller) {
        EFI_HANDLE *all_handles = NULL;
        size_t all_handle_count = 0;
        if (boot->locate_handle_buffer(EFI_ALL_HANDLES, NULL, NULL,
                &all_handle_count, &all_handles) == EFI_SUCCESS) {
            for (size_t index = 0; index < all_handle_count; ++index)
                boot->connect_controller(all_handles[index], NULL, NULL, 1);
            boot->free_pool(all_handles);
        }
    }

#if defined(INFINITY_AARCH64)
    /* Controller connection above assigns BARs before native ownership. */
    if (discover_acpi_network(system, info) &&
            info->network_reserved == INFINITY_NETWORK_ECAM) return;
#endif
    EFI_HANDLE *handles = NULL;
    size_t handle_count = 0;
    EFI_GUID guid = simple_network_guid;
    if (boot->locate_handle_buffer(EFI_BY_PROTOCOL, &guid, NULL,
            &handle_count, &handles) != EFI_SUCCESS) {
        (void)discover_acpi_network(system, info);
        return;
    }
    for (size_t index = 0; index < handle_count; ++index) {
        EFI_SIMPLE_NETWORK_PROTOCOL *network = NULL;
        guid = simple_network_guid;
        if (boot->handle_protocol(handles[index], &guid, (void **)&network) != EFI_SUCCESS ||
            !network || !network->mode) continue;
        EFI_SIMPLE_NETWORK_MODE *mode = network->mode;
        info->firmware_network = (uint64_t)(uintptr_t)network;
        info->network_device_count = 1;
        info->network_link_state = mode->media_present_supported
            ? (mode->media_present ? 2u : 1u)
            : 0u;
        info->network_mtu = mode->max_packet_size;
        info->network_capabilities = (network->receive ? 1u : 0u) |
                                     (network->transmit ? 2u : 0u);
        info->network_mac_length = mode->hw_address_size < sizeof(info->network_mac)
            ? mode->hw_address_size : sizeof(info->network_mac);
        info->network_reserved = 1u;
        memcpy(info->network_mac, mode->current_address.address, info->network_mac_length);
        serial_write("[BOOT] firmware network adapter ready\n");
        break;
    }
    boot->free_pool(handles);
    if (info->network_device_count != 0) return;

    /* VirtualBox ARM firmware can expose the emulated Ethernet controller on
       PCI without installing an SNP driver. Preserve that observed device as
       a typed, link-unknown adapter so onboarding does not report no hardware. */
    handles = NULL;
    handle_count = 0;
    guid = pci_io_guid;
    if (boot->locate_handle_buffer(EFI_BY_PROTOCOL, &guid, NULL,
            &handle_count, &handles) != EFI_SUCCESS) {
        (void)discover_acpi_network(system, info);
        return;
    }
    for (size_t index = 0; index < handle_count; ++index) {
        EFI_PCI_IO_PROTOCOL *pci = NULL;
        uint32_t config[4] = {0, 0, 0, 0};
        guid = pci_io_guid;
        if (boot->handle_protocol(handles[index], &guid, (void **)&pci) != EFI_SUCCESS ||
            !pci || !pci->pci.read ||
            pci->pci.read(pci, 2u, 0u, 4u, config) != EFI_SUCCESS) continue;
        if (((config[2] >> 24) & 0xffu) != 0x02u) continue;
        info->firmware_network = UINT64_C(0x8000000000000000) |
            ((uint64_t)(config[0] & 0xffffu) << 16) |
            ((uint64_t)(config[0] >> 16) & 0xffffu);
        info->network_device_count = 1;
        info->network_link_state = 0u;
        info->network_mtu = 0u;
        info->network_capabilities = 0u;
        info->network_mac_length = 0u;
        info->network_reserved = 2u;
        serial_write("[BOOT] PCI network adapter discovered\n");
        break;
    }
    boot->free_pool(handles);
    if (info->network_device_count == 0) (void)discover_acpi_network(system, info);
}

// ------------------------=
// FUNC: gather_firmware_audio
// DESC: Hands off one firmware-assigned 32-bit HDA BAR; no capture or playback starts at boot.
// ------------------=
static void gather_firmware_audio(EFI_SYSTEM_TABLE *system, InfinityBootInfo *info) {
    EFI_HANDLE *handles = NULL;
    size_t count = 0;
    EFI_GUID guid = pci_io_guid;
    EFI_BOOT_SERVICES *boot = system->boot_services;
    if (boot->locate_handle_buffer(EFI_BY_PROTOCOL, &guid, NULL, &count, &handles) != EFI_SUCCESS) return;
    for (size_t index = 0; index < count; ++index) {
        EFI_PCI_IO_PROTOCOL *pci = NULL;
        uint32_t config[6] = {0};
        if (boot->handle_protocol(handles[index], &guid, (void **)&pci) != EFI_SUCCESS ||
            !pci || !pci->pci.read || !pci->pci.write ||
            pci->pci.read(pci, 2u, 0u, 6u, config) != EFI_SUCCESS) continue;
        if ((config[2] >> 8) != 0x040300u || (config[4] & 1u) ||
            ((config[4] & 6u) == 4u && config[5] != 0) ||
            (config[4] & ~15u) < 0x100000u) continue;
        uint16_t command = (uint16_t)config[1] | 6u;
        if (pci->pci.write(pci, 1u, 4u, 1u, &command) != EFI_SUCCESS) continue;
        info->boot_reserved = config[4] & ~15u;
        break;
    }
    boot->free_pool(handles);
}

// ------------------------=
// FUNC: efi_main
// DESC: Runs the UEFI loader entry point.
// ------------------=
#include "payload_loader.h"

// ------------------------=
// FUNC: efi_main
// DESC: Boots the native kernel with optional streamed installation payloads.
// ------------------=
EFI_STATUS EFIAPI efi_main(EFI_HANDLE image, EFI_SYSTEM_TABLE *system) {
    serial_initialize();
    firmware_write(system, L"InfinityOS bootstrap\r\n");
    serial_write("InfinityOS bootstrap\n[BOOT] firmware entry\n");

    size_t image_size = 0;
    uint8_t media_boot = boot_media_has_kernel(image, system);
    void *kernel_image = media_boot ? NULL : try_load_installed_kernel(system, &image_size);
    if (!kernel_image && installed_generation_invalid) {
        switch (installed_generation_stage) {
            case 2: firmware_write(system, L"Installed boot validation: container only\r\n"); break;
            case 3: firmware_write(system, L"Installed boot validation: pool only\r\n"); break;
            case 4: firmware_write(system, L"Installed boot validation: spaces only\r\n"); break;
            case 5: firmware_write(system, L"Installed boot validation: catalog only\r\n"); break;
            case 6: firmware_write(system, L"Installed boot validation: system manifest only\r\n"); break;
            case 7: firmware_write(system, L"Installed boot validation: component manifest only\r\n"); break;
            case 8: firmware_write(system, L"Installed boot validation: kernel read failed\r\n"); break;
            case 9: firmware_write(system, L"Installed boot validation: kernel checksum failed\r\n"); break;
            default: firmware_write(system, L"Installed boot validation failed\r\n"); break;
        }
    }
    uint8_t booted_installed_generation = kernel_image != NULL;
    if (!kernel_image) kernel_image = load_kernel_file(image, system, &image_size);
    serial_write("[BOOT] kernel located\n");
    InfinityLoadedKernel kernel = load_elf(system, kernel_image, image_size);
    free_kernel_image(system->boot_services, kernel_image, image_size);
    serial_write("[BOOT] kernel loaded\n");

    uint64_t stack_base = UINT32_MAX;
    /* Signed Pool transactions nest bounded metadata, mutation and verification
       records. The optimized active call chain exceeds 568 KiB; reserve a fixed
       1 MiB on both architectures. Lightweight startup dispatch stays separate
       so idle polls do not reserve transaction-sized frames. */
    if (system->boot_services->allocate_pages(EFI_ALLOCATE_MAX_ADDRESS, EFI_LOADER_DATA, KERNEL_STACK_PAGES, &stack_base) != EFI_SUCCESS)
        fail(system, L"ERROR: stack allocation failed\r\n", "ERROR: stack allocation failed\n");
    uint64_t page_tables = prepare_identity_map(system);
    uint64_t native_pool = reserve_native_runtime_pool(system);
    InfinityBootInfo *info = NULL;
    if (system->boot_services->allocate_pool(EFI_LOADER_DATA, sizeof(*info), (void **)&info) != EFI_SUCCESS)
        fail(system, L"ERROR: BootInfo allocation failed\r\n", "ERROR: BootInfo allocation failed\n");

    info->magic = INFINITY_BOOT_MAGIC;
    info->version = INFINITY_BOOT_VERSION;
    info->worker_bridge = 0;
#if defined(INFINITY_AARCH64)
    info->worker_bridge = infinity_worker_bridge(system);
#else
    info->worker_bridge = x86_worker_bridge(system, page_tables);
#endif
    info->architecture = INFINITY_ARCHITECTURE;
    info->memory_map_address = 0;
    info->firmware_revision = system->header.revision;
    info->boot_flags = starts_with_edk(system->firmware_vendor) ? 1 : 0;
#if defined(INFINITY_AARCH64)
    if (acpi_serial_base(system) == UINT64_C(0xffddf000)) info->boot_flags |= UINT64_C(1) << 6;
#endif
    if (booted_installed_generation) info->boot_flags |= 32;
    info->framebuffer_address = 0;
    info->framebuffer_size = 0;
    info->framebuffer_width = 0;
    info->framebuffer_height = 0;
    info->framebuffer_stride = 0;
    info->framebuffer_format = 0;
    info->firmware_input = 0;
    info->firmware_pointer = 0;
    info->firmware_runtime_services = (uint64_t)(uintptr_t)system->runtime_services;
    info->firmware_network = 0;
    info->network_device_count = 0;
    info->network_link_state = 0;
    info->network_mtu = 0;
    info->network_capabilities = 0;
    info->network_mac_length = 0;
    info->network_reserved = 0;
    memset(info->network_mac, 0, sizeof(info->network_mac));
    memset(info->firmware_entropy, 0, sizeof(info->firmware_entropy));
    info->firmware_entropy_valid = 0;
    info->boot_reserved = 0;
    info->payload_bridge = 0;
    info->model_address = 0;
    info->model_bytes = 0;
    info->model_work_address = 0;
    info->model_work_bytes = 0;
    info->native_pool_address = native_pool;
    info->native_pool_bytes = native_pool ? (uint64_t)NATIVE_RUNTIME_POOL_PAGES * PAGE_SIZE : 0;
    info->kernel_address = kernel.low;
    info->kernel_bytes = kernel.high - kernel.low;
    info->kernel_text_address = kernel.text_low;
    info->kernel_text_bytes = kernel.text_high - kernel.text_low;
    info->kernel_rodata_address = kernel.rodata_low;
    info->kernel_rodata_bytes = kernel.rodata_high - kernel.rodata_low;
    info->kernel_data_address = kernel.data_low;
    info->kernel_data_bytes = kernel.data_high - kernel.data_low;
    info->kernel_page_table = page_tables;
    info->kernel_stack_address = stack_base;
    info->kernel_stack_bytes = (uint64_t)KERNEL_STACK_PAGES * PAGE_SIZE;
    prepare_payloads(image, system, info, booted_installed_generation);
    gather_firmware_entropy(system, info);
    gather_framebuffer(system, info);
#if defined(INFINITY_AARCH64)
    /* The extended console preserves Ctrl/Shift, unlike legacy text input. */
    EFI_GUID input_ex_guid = {0xdd9e7534, 0x7762, 0x4698,
        {0x8c, 0x14, 0xf5, 0x85, 0x17, 0xa6, 0x25, 0xaa}};
    void *input_ex = NULL;
    if (system->boot_services->handle_protocol(system->console_in_handle,
            &input_ex_guid, &input_ex) == EFI_SUCCESS && input_ex) {
        info->firmware_input = (uint64_t)(uintptr_t)input_ex;
        info->boot_flags |= 2 | 64;
    }
    else if (system->con_in && system->con_in->read_key_stroke) {
        info->firmware_input = (uint64_t)(uintptr_t)system->con_in;
        info->boot_flags |= 2;
        serial_write("[BOOT] firmware input bridge ready\n");
    }
    InfinityFirmwarePointers *pointers = gather_firmware_pointers(system);
    if (pointers) {
        info->firmware_pointer = (uint64_t)(uintptr_t)pointers;
        info->boot_flags |= 16;
        if (pointers->absolute_count) info->boot_flags |= 8;
        if (pointers->relative_count) info->boot_flags |= 4;
        serial_write("[BOOT] firmware pointer set ready\n");
    }
#endif
    gather_firmware_network(system, info);
    gather_firmware_audio(system, info);

    /* AArch64 retains selected firmware services for its input bridge. Disable
     * the standard five-minute image watchdog before transferring control so a
     * healthy installed system is not reset after 300 seconds. */
    if (system->boot_services->set_watchdog_timer)
        system->boot_services->set_watchdog_timer(0, 0, 0, NULL);

    size_t map_size = 0, descriptor_size = 0;
    uint64_t map_key = 0;
    uint32_t descriptor_version = 0;
    system->boot_services->get_memory_map(&map_size, NULL, &map_key, &descriptor_size, &descriptor_version);
    map_size += descriptor_size * 8;
    void *memory_map = NULL;
    if (descriptor_size == 0 || system->boot_services->allocate_pool(EFI_LOADER_DATA, map_size, &memory_map) != EFI_SUCCESS)
        fail(system, L"ERROR: memory-map allocation failed\r\n", "ERROR: memory-map allocation failed\n");
    info->memory_map_address = (uint64_t)(uintptr_t)memory_map;
    serial_write("[BOOT] memory prepared\n[BOOT] transferring control\n");

#if defined(INFINITY_AARCH64)
    size_t current_size = map_size;
    if (system->boot_services->get_memory_map(&current_size, memory_map, &map_key,
                                               &descriptor_size, &descriptor_version) != EFI_SUCCESS)
        fail(system, L"ERROR: final memory map failed\r\n", "ERROR: final memory map failed\n");
    info->memory_map_size = current_size;
    info->memory_descriptor_size = descriptor_size;
    infinity_handoff(info, kernel.entry, stack_base + KERNEL_STACK_PAGES * PAGE_SIZE, page_tables);
#else
    for (unsigned attempt = 0; attempt < 2; ++attempt) {
        size_t current_size = map_size;
        if (system->boot_services->get_memory_map(&current_size, memory_map, &map_key,
                                                   &descriptor_size, &descriptor_version) != EFI_SUCCESS)
            fail(system, L"ERROR: final memory map failed\r\n", "ERROR: final memory map failed\n");
        info->memory_map_size = current_size;
        info->memory_descriptor_size = descriptor_size;
        if (system->boot_services->exit_boot_services(image, map_key) == EFI_SUCCESS)
            infinity_handoff(info, kernel.entry, stack_base + KERNEL_STACK_PAGES * PAGE_SIZE, page_tables);
    }
#endif
    fail(system, L"ERROR: unable to exit boot services\r\n", "ERROR: unable to exit boot services\n");
}
