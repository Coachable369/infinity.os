/* Read-only firmware USB descriptor probe. Not a capture driver or login path. */
#define efi_main unused_production_entry
#include "../../boot/common/uefi_loader.c"
#undef efi_main

// ------------------------=
// FUNC: probe_serial
// DESC: Writes bounded diagnostics to VirtualBox ARM PL011 rather than the QEMU UART address.
// ------------------=
static void probe_serial(const char *text) {
    for (;*text;++text) {
        size_t budget=1000000;
        while((*(volatile uint32_t *)(uintptr_t)0xffddf018&32) && --budget) {}
        if(!budget) return;
        *(volatile uint32_t *)(uintptr_t)0xffddf000=(uint8_t)*text;
    }
}
#define serial_write probe_serial

// ------------------------=
// FUNC: save_bytes
// DESC: Saves raw evidence only to the diagnostic application's own boot volume.
// ------------------=
static EFI_STATUS save_bytes(EFI_HANDLE image, EFI_SYSTEM_TABLE *system, CHAR16 *path, void *bytes, size_t length) {
    EFI_LOADED_IMAGE_PROTOCOL *loaded=NULL;
    EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *fs=NULL;
    EFI_FILE_PROTOCOL *root=NULL,*file=NULL;
    EFI_GUID guid=loaded_image_guid;
    EFI_STATUS status=system->boot_services->handle_protocol(image,&guid,(void **)&loaded);
    if(status!=EFI_SUCCESS) return status;
    guid=simple_fs_guid;
    status=system->boot_services->handle_protocol(loaded->device_handle,&guid,(void **)&fs);
    if(status!=EFI_SUCCESS) return status;
    status=fs->open_volume(fs,&root);
    if(status!=EFI_SUCCESS) return status;
    status=root->open(root,&file,path,0x8000000000000003ULL,0);
    if(status==EFI_SUCCESS) {
        typedef EFI_STATUS (EFIAPI *Write)(EFI_FILE_PROTOCOL *,size_t *,void *);
        size_t actual=length;
        status=((Write)file->write)(file,&actual,bytes);
        if(status==EFI_SUCCESS && actual!=length) status=0x8000000000000007ULL;
        file->close(file);
    }
    root->close(root);
    return status;
}

// ------------------------=
// FUNC: dump_bytes
// DESC: Emits best-effort hexadecimal diagnostics; binary files are authoritative evidence.
// ------------------=
static void dump_bytes(const uint8_t *bytes, size_t length) {
    const char *digits = "0123456789abcdef";
    for (size_t i=0; i<length; ++i) {
        char pair[3]={digits[bytes[i]>>4],digits[bytes[i]&15],0};
        serial_write(pair);
    }
    serial_write("\n");
}

// ------------------------=
// FUNC: efi_main
// DESC: Polls firmware USB handles for one minute and reads actual device/configuration descriptors without starting a stream.
// ------------------=
EFI_STATUS EFIAPI efi_main(EFI_HANDLE image, EFI_SYSTEM_TABLE *system) {
    (void)image;
    serial_initialize();
    system->boot_services->set_watchdog_timer(0,0,0,NULL);
    EFI_GUID guid={0x2b2f68d6,0x0cd2,0x44cf,{0x8e,0x8b,0xbb,0xa2,0x0b,0x1b,0x5b,0x75}};
    typedef EFI_STATUS (EFIAPI *Stall)(size_t);
    /* Stall is Boot Services slot 28 (unused_27_28[1]). */
    Stall stall=(Stall)system->boot_services->unused_27_28[1];
    serial_write("CAMERA_DESCRIPTOR_PROBE_BEGIN\n");
    for (size_t attempt=0;attempt<60;++attempt) {
        EFI_HANDLE *handles=NULL; size_t count=0;
        EFI_STATUS status=system->boot_services->locate_handle_buffer(EFI_BY_PROTOCOL,&guid,NULL,&count,&handles);
        if (status==EFI_SUCCESS) {
            for(size_t i=0;i<count;++i) {
                EFI_USB_IO_PROTOCOL *usb=NULL;
                if(system->boot_services->handle_protocol(handles[i],&guid,(void **)&usb)!=EFI_SUCCESS || !usb || !usb->get_interface_descriptor || !usb->control_transfer) continue;
                EFI_USB_INTERFACE_DESCRIPTOR interface;
                if(usb->get_interface_descriptor(usb,&interface)!=EFI_SUCCESS) continue;
                if(attempt==0){serial_write("INTERFACE ");dump_bytes((const uint8_t *)&interface,sizeof(interface));}
                if(interface.interface_class!=14) continue;
                static uint8_t device[18], config[4096]; uint32_t result=0;
                EFI_USB_DEVICE_REQUEST request={0x80,6,0x100,0,18};
                status=usb->control_transfer(usb,&request,0,1000,device,18,&result);
                if(status!=EFI_SUCCESS || result){serial_write("DEVICE_READ_FAILED\n");continue;}
                serial_write("DEVICE ");dump_bytes(device,18);
                request.value=0x200;request.length=9;
                status=usb->control_transfer(usb,&request,0,1000,config,9,&result);
                if(status!=EFI_SUCCESS || result){serial_write("CONFIG_HEADER_FAILED\n");continue;}
                size_t length=(size_t)config[2]|((size_t)config[3]<<8);
                if(length<9 || length>sizeof(config)){serial_write("CONFIG_LENGTH_REJECTED\n");continue;}
                request.length=(uint16_t)length;
                status=usb->control_transfer(usb,&request,0,1000,config,length,&result);
                if(status!=EFI_SUCCESS || result){serial_write("CONFIG_READ_FAILED\n");continue;}
                serial_write("CONFIG ");dump_bytes(config,length);
                if(save_bytes(image,system,(CHAR16 *)L"\\device.bin",device,18)!=EFI_SUCCESS ||
                   save_bytes(image,system,(CHAR16 *)L"\\config.bin",config,length)!=EFI_SUCCESS) {
                    system->boot_services->free_pool(handles);
                    return 0x8000000000000007ULL;
                }
                system->boot_services->free_pool(handles);
                serial_write("CAMERA_DESCRIPTOR_PROBE_COMPLETE\n");
                return EFI_SUCCESS;
            }
            system->boot_services->free_pool(handles);
        }
        stall(1000000);
    }
    serial_write("CAMERA_DESCRIPTOR_PROBE_NO_UVC_HANDLE\n");
    return 0x800000000000000eULL;
}
