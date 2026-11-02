mod console {
    #[derive(Debug, PartialEq)]
    pub enum ConsoleKey {
        Shortcut(u8),
        SelectMove(i8),
        Character(u8),
        Backspace,
        Enter,
        Escape,
        Tab(bool),
        Up,
        Down,
        Left,
        Right,
        Home,
        End,
        Delete,
        Help,
    }
}
#[path = "../kernel/drivers/input/desktop_shortcuts.rs"]
mod desktop_shortcuts;
#[path = "../kernel/drivers/input/firmware_key.rs"]
mod firmware_key;
#[path = "../kernel/drivers/input/keyboard.rs"]
mod keyboard;
#[path = "../kernel/drivers/input/modifiers.rs"]
mod modifiers;
#[path = "../kernel/drivers/input/firmware_keyboard.rs"]
mod firmware_keyboard;

// ------------------------=
// FUNC: main
// DESC: Exercises actual firmware translation with extended and legacy key packets.
// ------------------=
fn main() {
    use console::ConsoleKey::*;
    use firmware_key::decode;
    native_browser_keys();
    physical_modifier_lifecycle();
    firmware_protocol_lifecycle();
    for command in [0x40, 0x80, 0xc0] {
        assert_eq!(decode(0,8,0x8000_0000|command),Some(Backspace));
        assert_eq!(decode(0,13,0x8000_0000|command),Some(Enter));
        for key in [b'=', b'+', b'-', b'0', b'[', b']', b'a', b'c', b'v'] {
            assert_eq!(decode(0,key as u16,0x8000_0000|command),Some(Shortcut(key)));
        }
        assert_eq!(
            decode(0, 9, 0x8000_0000 | command),
            Some(Shortcut(desktop_shortcuts::SPATIAL_NEXT))
        );
        assert_eq!(
            decode(0, 9, 0x8000_0001 | command),
            Some(Shortcut(desktop_shortcuts::SPATIAL_PREVIOUS))
        );
        assert_eq!(decode(0, 9, command), Some(Tab(false)));
    }
    for bits in 0..=255u8 {
        let modifiers = desktop_shortcuts::hid_modifiers(bits);
        assert_eq!(modifiers & 1 != 0, bits & 0x22 != 0);
        assert_eq!(modifiers & 2 != 0, bits & 0x11 != 0);
        assert_eq!(modifiers & 4 != 0, bits & 0x88 != 0);
        assert_eq!(modifiers & 8 != 0, bits & 0x44 != 0);
        assert_eq!(
            desktop_shortcuts::spatial_chord(true, modifiers & 4 != 0, modifiers & 1 != 0),
            if bits & 0x88 == 0 {
                None
            } else {
                Some(if bits & 0x22 == 0 {
                    desktop_shortcuts::SPATIAL_NEXT
                } else {
                    desktop_shortcuts::SPATIAL_PREVIOUS
                })
            }
        );
        assert_eq!(
            desktop_shortcuts::spatial_chord(false, modifiers & 4 != 0, modifiers & 1 != 0),
            None
        );
    }
    for letter in [b'n', b'e', b't', b'p', b'l'] {
        assert_eq!(
            decode(0, letter as u16, 0x8000_0005),
            Some(Shortcut(letter.to_ascii_uppercase()))
        );
    }
    for ctrl in [4, 8] {
        for key in [b'=',b'+',b'-',b'0',b'[',b']'] {
            assert_eq!(decode(0,key as u16,0x8000_0000|ctrl),Some(Shortcut(key)));
        }
        for value in [10, b'j' as u16, b'J' as u16] {
            assert_eq!(decode(0, value, 0x8000_0000 | ctrl), Some(Shortcut(b'j')));
        }
        assert_eq!(decode(0, 13, 0x8000_0000 | ctrl), Some(Shortcut(b'm')));
        assert_eq!(
            decode(0, b'c' as u16, 0x8000_0000 | ctrl),
            Some(Shortcut(b'c'))
        );
    }
    assert_eq!(decode(0, b'j' as u16, 8), Some(Character(b'j')));
    assert_eq!(decode(0, 10, 0), Some(Shortcut(b'j')));
    assert_eq!(decode(0, 9, 0x8000_0001), Some(Tab(true)));
    assert_eq!(decode(0, 9, 0), Some(Tab(false)));
    assert_eq!(decode(0, 13, 0), Some(Enter));
    assert_eq!(decode(0, 8, 0), Some(Backspace));
    assert_eq!(decode(4, 0, 0x8000_0002), Some(SelectMove(-1)));
    assert_eq!(decode(3, 0, 0), Some(Right));
    assert_eq!(decode(0, 0, 0x8000_0008), None);
}

// ------------------------=
// FUNC: native_browser_keys
// DESC: Exercises actual shared HID/PS2 and firmware decoding, including modifier-preserving tab and page commands.
// ------------------=
fn native_browser_keys() {
    use console::ConsoleKey::*;
    use desktop_shortcuts::*;
    use keyboard::modified_console_key as hid;
    for (mask,firmware) in [(2,4),(2,8),(4,0x40),(4,0x80)] {
        for (usage,c) in [(4,b'a'),(6,b'c'),(27,b'x'),(25,b'v'),(29,b'z'),(28,b'y'),
            (15,b'l'),(21,b'r'),(23,b't'),(26,b'w'),(9,b'f'),(0x2f,b'['),(0x30,b']'),
            (0x2e,b'='),(0x2d,b'-'),(0x27,b'0')] {
            assert_eq!(hid(usage,mask),Some(Shortcut(c)));
            assert_eq!(firmware_key::decode(0,u16::from(c),0x8000_0000|firmware),Some(Shortcut(c)));
        }
        assert_eq!(hid(29,mask|1),Some(Shortcut(b'y')));
        assert_eq!(firmware_key::decode(0,b'z' as u16,0x8000_0001|firmware),Some(Shortcut(b'y')));
        assert_eq!(hid(0x2e,mask|1),Some(Shortcut(b'+')));
        assert_eq!(hid(0x57,mask),Some(Shortcut(b'+')));
        assert_eq!(hid(0x56,mask),Some(Shortcut(b'-')));
        assert_eq!(hid(0x4b,mask),Some(Shortcut(TAB_PREVIOUS)));
        assert_eq!(hid(0x4e,mask),Some(Shortcut(TAB_NEXT)));
        for reverse in [false,true] {
            let expected=if mask==4 {if reverse {SPATIAL_PREVIOUS}else{SPATIAL_NEXT}}
                else if reverse {TAB_PREVIOUS}else{TAB_NEXT};
            assert_eq!(hid(0x2b,mask|u8::from(reverse)),Some(Shortcut(expected)));
            assert_eq!(firmware_key::decode(0,9,0x8000_0000|firmware|u32::from(reverse)),Some(Shortcut(expected)));
        }
    }
    for (usage,scan,code) in [(0x4b,9,PAGE_UP),(0x4e,10,PAGE_DOWN),(0x3e,0x0f,RELOAD)] {
        assert_eq!(hid(usage,0),Some(Shortcut(code)));
        assert_eq!(firmware_key::decode(scan,0,0),Some(Shortcut(code)));
    }
    assert_ne!(RELOAD,b'r');
    assert_eq!(hid(0x2b,1),Some(Tab(true)));
    assert_eq!(hid(0x2b,0),Some(Tab(false)));
    assert_eq!(hid(0x4a,0),Some(Home));assert_eq!(hid(0x4d,0),Some(End));
    assert_eq!(hid(0x29,0),Some(Escape));
    assert_eq!(hid(0x50,8),Some(Shortcut(b'[')));
    assert_eq!(hid(0x4f,8),Some(Shortcut(b']')));
    for alt in [0x10,0x20] {
        assert_eq!(firmware_key::decode(4,0,0x8000_0000|alt),Some(Shortcut(b'[')));
        assert_eq!(firmware_key::decode(3,0,0x8000_0000|alt),Some(Shortcut(b']')));
    }
}

// ------------------------=
// FUNC: physical_modifier_lifecycle
// DESC: Verifies simultaneous modifier sides, idle packets, disconnect/reset and independent keyboard sources.
// ------------------=
fn physical_modifier_lifecycle() {
    use modifiers::*;
    reset();assert_eq!(held(),0);
    for (both,right,expected) in [(0x11,0x10,2),(0x88,0x80,4),(0x22,0x20,1)] {
        usb_report(0,both);assert_eq!(held(),expected);
        usb_report(0,right);assert_eq!(held(),expected);
        assert!(!usb_fault(0x8000_0000_0000_0012,0x40));assert_eq!(held(),expected);
        assert!(!usb_fault(0x8000_0000_0000_0006,0));assert_eq!(held(),expected);
        usb_report(0,0);assert_eq!(held(),0);
    }
    usb_report(0,1);usb_report(1,0x80);assert_eq!(held(),6);
    usb_reset(0);assert_eq!(held(),4);
    assert!(usb_fault(0x8000_0000_0000_0007,0));usb_reset(1);assert_eq!(held(),0);
    ps2_transition(0x1d,false);ps2_transition(0x1d,true);assert_eq!(held(),2);
    ps2_transition(0x9d,false);assert_eq!(held(),2);
    ps2_transition(0x9d,true);assert_eq!(held(),0);
    ps2_transition(0x5b,true);ps2_transition(0x5c,true);assert_eq!(held(),4);
    ps2_transition(0xdb,true);assert_eq!(held(),4);
    ps2_transition(0xdc,true);assert_eq!(held(),0);
    ps2_transition(0x2a,false);ps2_transition(0x36,false);assert_eq!(held(),1);
    ps2_transition(0xaa,false);assert_eq!(held(),1);
    ps2_transition(0xb6,false);assert_eq!(held(),0);
    usb_report(0,0x88);ps2_transition(0x1d,false);reset();assert_eq!(held(),0);
}

#[repr(C)]
struct MockKeyboard {
    protocol: firmware_keyboard::SimpleTextInputEx,
    status: u64,
    data: firmware_keyboard::KeyData,
    set_result: u64,
    set_calls: usize,
    last_toggle: u8,
    omit_state: bool,
}

// ------------------------=
// FUNC: mock_read
// DESC: Models current key state returned even when no keystroke is queued, or an implementation that omits it.
// ------------------=
unsafe extern "efiapi" fn mock_read(protocol:*mut firmware_keyboard::SimpleTextInputEx,out:*mut firmware_keyboard::KeyData)->u64 {
    let mock=&mut *protocol.cast::<MockKeyboard>();
    if !mock.omit_state {*out=mock.data;}
    mock.status
}

// ------------------------=
// FUNC: mock_set
// DESC: Records actual protocol toggle requests and simulates supported or unsupported partial-key exposure.
// ------------------=
unsafe extern "efiapi" fn mock_set(protocol:*mut firmware_keyboard::SimpleTextInputEx,toggle:*mut u8)->u64 {
    let mock=&mut *protocol.cast::<MockKeyboard>();
    mock.set_calls+=1;mock.last_toggle=*toggle;
    if mock.set_result==0 {mock.data.toggle_state=*toggle;}
    mock.set_result
}

// ------------------------=
// FUNC: firmware_protocol_lifecycle
// DESC: Exercises the production protocol adapter with partial keys, idle release, invalid state and lock-preserving exposure.
// ------------------=
fn firmware_protocol_lifecycle() {
    use firmware_keyboard::*;
    assert_eq!(core::mem::size_of::<KeyData>(),12);
    assert_eq!(core::mem::offset_of!(KeyData,shift_state),4);
    assert_eq!(core::mem::offset_of!(SimpleTextInputEx,set_state),3*core::mem::size_of::<usize>());
    let mut mock=MockKeyboard {
        protocol:SimpleTextInputEx {reset:0,read_key_stroke:mock_read,wait_for_key:0,set_state:Some(mock_set),register_notify:0,unregister_notify:0},
        status:NOT_READY,data:KeyData {key:InputKey::default(),shift_state:0x8000_0000,toggle_state:0x87},
        set_result:0,set_calls:0,last_toggle:0,omit_state:false,
    };
    let mut exposure=Exposure::new();
    let sample=unsafe {poll(&mut mock.protocol,&mut exposure)};
    assert_eq!(sample.status,NOT_READY);assert_eq!(sample.modifiers,0);
    assert_eq!(mock.last_toggle,0xc7);assert_eq!(mock.set_calls,1);assert!(exposure.enabled);
    for (both,right,expected) in [(12,4,2),(0xc0,0x40,4),(3,1,1),(0x30,0x10,8)] {
        for state in [both,right,0] {
            mock.status=0;mock.data.shift_state=0x8000_0000|state;
            let sample=unsafe {poll(&mut mock.protocol,&mut exposure)};
            assert_eq!(sample.modifiers,if state==0 {0}else{expected});
            assert!(firmware_key::decode(sample.data.key.scan_code,sample.data.key.unicode_character,sample.data.shift_state).is_none());
            modifiers::firmware_report(sample.modifiers);assert_eq!(modifiers::held(),sample.modifiers);
            mock.status=NOT_READY;
            assert_eq!(unsafe {poll(&mut mock.protocol,&mut exposure)}.modifiers,sample.modifiers);
        }
    }
    assert_eq!(mock.set_calls,1);
    mock.data.shift_state=0x8000_0008;mock.status=NOT_READY;
    modifiers::firmware_report(unsafe {poll(&mut mock.protocol,&mut exposure)}.modifiers);assert_eq!(modifiers::held(),2);
    mock.data.shift_state=0x8000_0000;
    modifiers::firmware_report(unsafe {poll(&mut mock.protocol,&mut exposure)}.modifiers);assert_eq!(modifiers::held(),0);
    for (status,state) in [(NOT_READY,8),(0,8),(0x8000_0000_0000_0007,0x8000_0008)] {
        mock.status=status;mock.data.shift_state=state;
        assert_eq!(unsafe {poll(&mut mock.protocol,&mut exposure)}.modifiers,0);
    }
    mock.status=NOT_READY;mock.data.shift_state=0x8000_0008;mock.omit_state=true;
    assert_eq!(unsafe {poll(&mut mock.protocol,&mut exposure)}.modifiers,0);
    mock.omit_state=false;mock.set_result=UNSUPPORTED;mock.data.toggle_state=0x82;exposure=Exposure::new();
    assert_eq!(unsafe {poll(&mut mock.protocol,&mut exposure)}.modifiers,2);
    assert!(!exposure.enabled);assert_eq!(mock.last_toggle,0xc2);
    mock.data.shift_state=0x8000_0000;
    assert_eq!(unsafe {poll(&mut mock.protocol,&mut exposure)}.modifiers,0);
    mock.data.toggle_state=0;exposure=Exposure::new();let calls=mock.set_calls;
    unsafe {poll(&mut mock.protocol,&mut exposure)};
    assert_eq!(mock.set_calls,calls);
    assert_eq!(unsafe {poll(core::ptr::null_mut(),&mut exposure)}.modifiers,0);
    mock.data.toggle_state=0x81;mock.set_result=0;
    unsafe {poll(&mut mock.protocol,&mut exposure)};
    assert_eq!(mock.set_calls,calls+1);assert_eq!(mock.last_toggle,0xc1);
    mock.protocol.set_state=None;mock.data.toggle_state=0x80;exposure=Exposure::new();
    unsafe {poll(&mut mock.protocol,&mut exposure)};
    assert!(!exposure.enabled);
    modifiers::reset();
}
