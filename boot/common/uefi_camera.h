/* ARM retained-firmware adapter. Kernel-only, synchronous bounded USB requests.
 * No pointers from this interface may be forwarded into application memory. */
static EFI_SYSTEM_TABLE *infinity_camera_system;
static EFI_HANDLE infinity_camera_handle;
static size_t infinity_camera_scan;

// ------------------------=
// FUNC: infinity_camera_exchange
// DESC: Implements a fixed-camera allowlist and six narrowly scoped USB operations for the retained-firmware ARM path.
// ------------------=
static int64_t EFIAPI infinity_camera_exchange(uint64_t operation, uint8_t *bytes, uint64_t *length) {
    if (!infinity_camera_system || !length) return -1;
    EFI_BOOT_SERVICES *boot=infinity_camera_system->boot_services;
    EFI_GUID guid={0x2b2f68d6,0x0cd2,0x44cf,{0x8e,0x8b,0xbb,0xa2,0x0b,0x1b,0x5b,0x75}};
    EFI_USB_IO_PROTOCOL *usb=NULL;
    uint32_t result=0;
    if (operation==0) {
        EFI_HANDLE *handles=NULL; size_t count=0;
        infinity_camera_handle=NULL;
        if (boot->locate_handle_buffer(EFI_BY_PROTOCOL,&guid,NULL,&count,&handles)!=EFI_SUCCESS) return 1;
        size_t i=infinity_camera_scan++;
        if(i<count && i<64) do {
            EFI_USB_INTERFACE_DESCRIPTOR interface;
            if (boot->handle_protocol(handles[i],&guid,(void **)&usb)!=EFI_SUCCESS || !usb ||
                usb->get_interface_descriptor(usb,&interface)!=EFI_SUCCESS ||
                interface.interface_number!=1 || interface.interface_class!=14 || interface.interface_subclass!=2 ||
                !usb->bulk_transfer) break;
            uint8_t descriptor[18];
            EFI_USB_DEVICE_REQUEST request={0x80,6,0x100,0,18};
            if (usb->control_transfer(usb,&request,0,10,descriptor,18,&result)!=EFI_SUCCESS || result ||
                descriptor[8]!=0xee || descriptor[9]!=0x80 || descriptor[10]!=0x30 || descriptor[11]!=0) break;
            EFI_USB_ENDPOINT_DESCRIPTOR endpoint;
            for(uint8_t j=0;j<interface.num_endpoints && j<16;++j) {
                if (usb->get_endpoint_descriptor(usb,j,&endpoint)==EFI_SUCCESS &&
                    endpoint.endpoint_address==0x82 && (endpoint.attributes&3)==2) {
                    infinity_camera_handle=handles[i];break;
                }
            }
        } while(0);
        boot->free_pool(handles);
        if(infinity_camera_handle) {infinity_camera_scan=0;return 0;}
        if(infinity_camera_scan<count && infinity_camera_scan<64) return 2;
        infinity_camera_scan=0;return 1;
    }
    if(operation==5 && !infinity_camera_handle) {infinity_camera_scan=0;return 0;}
    if (!infinity_camera_handle || boot->handle_protocol(infinity_camera_handle,&guid,(void **)&usb)!=EFI_SUCCESS || !usb) {
        infinity_camera_handle=NULL;infinity_camera_scan=0;return -3;
    }
    EFI_STATUS status;
    if(operation>=1 && operation<=3) {
        if(!bytes || *length!=26) return -2;
        EFI_USB_DEVICE_REQUEST request={operation==2?0xa1:0x21,operation==2?0x81:1,operation==3?0x200:0x100,1,26};
        status=usb->control_transfer(usb,&request,operation==2?0:1,10,bytes,26,&result);
    } else if(operation==4) {
        if(!bytes || *length<2 || *length>65536) return -2;
        typedef EFI_STATUS (EFIAPI *Bulk)(EFI_USB_IO_PROTOCOL *,uint8_t,void *,size_t *,size_t,uint32_t *);
        size_t actual=(size_t)*length;
        status=((Bulk)usb->bulk_transfer)(usb,0x82,bytes,&actual,10,&result);*length=actual;
        if(status==0x8000000000000012ULL) return 1;
    } else if(operation==5) {
        EFI_USB_DEVICE_REQUEST request={0x02,1,0,0x82,0};
        status=usb->control_transfer(usb,&request,2,10,NULL,0,&result);
        /* All bulk calls have returned before this call; no retained DMA buffers. */
        if(status==EFI_SUCCESS && result==0) infinity_camera_handle=NULL;
    } else return -2;
    if(status!=EFI_SUCCESS || result) return -1;
    return 0;
}
