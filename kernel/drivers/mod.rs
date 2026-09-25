pub mod device;
pub mod display;
pub mod input;
pub mod svga;
pub mod network;
pub(crate) mod hda;
pub mod audio;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(crate) mod https;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
mod e1000;

use crate::{boot_info::BootInfo, bootstrap};
use device::{DeviceIdentity, DeviceKind, DeviceState};

// ------------------------=
// FUNC: initialize
// DESC: Initializes initialize state.
// ------------------=
pub fn initialize(info: &BootInfo) -> [DeviceIdentity; 3] {
    let display_ready = bootstrap::show_splash(info);
    let display_driver = if display_ready {
        display::initialize(info)
    } else {
        "unavailable"
    };
    let input = input::initialize(info);
    input::report_pointer_discovery(input.pointer);
    let devices = [
        DeviceIdentity::new(
            "display0",
            DeviceKind::Display,
            display_driver,
            if display_ready {
                DeviceState::Ready
            } else {
                DeviceState::Unavailable
            },
        ),
        DeviceIdentity::new(
            "keyboard0",
            DeviceKind::Keyboard,
            if cfg!(target_arch = "aarch64") {
                "uefi-input-bridge"
            } else {
                "ps2-keyboard"
            },
            if input.keyboard {
                DeviceState::Ready
            } else {
                DeviceState::Unavailable
            },
        ),
        DeviceIdentity::new(
            "mouse0",
            DeviceKind::Mouse,
            input.pointer.driver_name(),
            if input.mouse {
                DeviceState::Ready
            } else {
                DeviceState::Unavailable
            },
        ),
    ];
    for device in devices {
        let _ = (device.kind, device.driver);
        crate::output_text(b"[device] ");
        crate::output_text(device.name.as_bytes());
        crate::output_text(match device.state {
            DeviceState::Ready => b" ready\n",
            DeviceState::Unavailable => b" unavailable\n",
        });
    }
    crate::output_text(b"[driver] input/display foundation online\n");
    devices
}
