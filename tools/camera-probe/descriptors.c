/* Firmware transport diagnostic. Not an isolated process or login path. */
#define efi_main unused_production_entry
#include "../../boot/common/uefi_loader.c"
#undef efi_main
#include "uvc_transport.h"
static uint8_t camera_probe_reply[26];
/* Diagnostic counters contain no frame content or host pointers. */
static uint64_t camera_transfers[8];

// ------------------------=
// FUNC: camera_control
// DESC: Sends only bounded UVC streaming control requests to interface one.
// ------------------=
static int camera_control(void *context, uint8_t operation, uint16_t selector, uint8_t *data, size_t length) {
    EFI_USB_IO_PROTOCOL *usb=context;
    EFI_USB_DEVICE_REQUEST request={operation&0x80?0xa1:0x21,operation,selector,1,(uint16_t)length};
    uint32_t result=0;
    EFI_STATUS status=usb->control_transfer(usb,&request,operation&0x80?0:1,100,data,length,&result);
    if(operation==0x81 && length==26) for(size_t i=0;i<26;++i) camera_probe_reply[i]=data[i];
    return status!=EFI_SUCCESS || result ? -1 : 0;
}
// ------------------------=
// FUNC: camera_bulk
// DESC: Performs one bounded firmware transfer on the discovered bulk IN endpoint.
// ------------------=
static int camera_bulk(void *context, uint8_t *data, size_t *length, unsigned timeout) {
    EFI_USB_IO_PROTOCOL *usb=context;
    typedef EFI_STATUS (EFIAPI *Bulk)(EFI_USB_IO_PROTOCOL *,uint8_t,void *,size_t *,size_t,uint32_t *);
    uint32_t result=0;
    EFI_STATUS status=((Bulk)usb->bulk_transfer)(usb,0x82,data,length,timeout,&result);
    ++camera_transfers[0];
    camera_transfers[1]=status;
    camera_transfers[2]=result;
    camera_transfers[3]=*length;
    if(status==EFI_SUCCESS && !result) {
        ++camera_transfers[4];
        camera_transfers[5]+=*length;
    } else if(status==0x8000000000000012ULL) {
        ++camera_transfers[6];
        camera_transfers[7]+=*length;
    }
    if(status==0x8000000000000012ULL) return 1;
    return status!=EFI_SUCCESS || result ? -1 : 0;
}
// ------------------------=
// FUNC: camera_milliseconds
// DESC: Reads the guest architectural counter for deadlines without host timing.
// ------------------=
static uint64_t camera_milliseconds(void) {
    uint64_t counter,frequency;
    __asm__ volatile("mrs %0, cntvct_el0":"=r"(counter));
    __asm__ volatile("mrs %0, cntfrq_el0":"=r"(frequency));
    return counter/(frequency/1000);
}

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
// DESC: Reads actual descriptors and attempts a bounded real-frame capture on the disposable probe VM.
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
                if(interface.interface_class!=14 || interface.interface_subclass!=2) continue;
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
                if(device[8]!=0xee || device[9]!=0x80 || device[10]!=0x30 || device[11]!=0 || !usb->bulk_transfer) {
                    system->boot_services->free_pool(handles);
                    return 0x8000000000000003ULL;
                }
                static uint8_t payload[UVC_PAYLOAD_LIMIT], frame[UVC_FRAME_LIMIT];
                UvcIo io={usb,camera_control,camera_bulk}; UvcStream stream;
                uint64_t start=camera_milliseconds(), longest=0, calls=0;
                uvc_start(&stream,io,start);
                while(stream.state==UVC_RUNNING && calls<10000) {
                    uint64_t before=camera_milliseconds();
                    uvc_step(&stream,io,before,0,payload,frame);
                    uint64_t elapsed=camera_milliseconds()-before;
                    if(elapsed>longest) longest=elapsed;
                    ++calls;
                }
                uint64_t report[6]={stream.state,stream.length,camera_milliseconds()-start,longest,calls,stream.payload_limit};
                save_bytes(image,system,(CHAR16 *)L"\\capture.bin",report,sizeof(report));
                save_bytes(image,system,(CHAR16 *)L"\\probe.bin",camera_probe_reply,sizeof(camera_probe_reply));
                save_bytes(image,system,(CHAR16 *)L"\\usb.bin",camera_transfers,sizeof(camera_transfers));
                if(stream.state==UVC_FRAME) save_bytes(image,system,(CHAR16 *)L"\\frame.jpg",frame,stream.length);
                uvc_cancel(&stream);
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
