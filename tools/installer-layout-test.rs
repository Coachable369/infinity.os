#[path = "../kernel/ui/installer_template.rs"]
mod installer_template;

#[path = "../kernel/ui/installer_layout.rs"]
mod installer_layout;

use installer_layout::installer_wizard_layout;

// ------------------------=
// FUNC: main
// DESC: Verifies every installer state shares one centered, contained gold-standard frame.
// ------------------=
fn main() {
    for (display_width, display_height) in [(1024usize, 768usize), (2048, 1238), (2560, 1600)] {
        let reference = installer_wizard_layout(2, display_width, display_height);
        for screen in 1..=11 {
            let layout = installer_wizard_layout(screen, display_width, display_height);
            assert_eq!(layout, reference, "screen {screen} changed installer geometry");
            assert_eq!(layout.masthead.left * 2 + layout.masthead.width, display_width);
            assert_eq!(layout.panel.left * 2 + layout.panel.width, display_width);
            assert!(layout.masthead.bottom() <= layout.panel.top);
            assert!(layout.panel.right() <= display_width);
            assert!(layout.panel.bottom() <= display_height);
            assert!(layout.panel.contains(layout.content));
            assert!(layout.panel.contains(layout.navigation_rail));
            assert!(layout.panel.contains(layout.back_button));
            assert!(layout.panel.contains(layout.primary_button));
            assert!(layout.panel.contains(layout.footer_rail));
            assert!(layout.content.bottom() <= layout.back_button.top);
            assert_eq!(layout.back_button.top, layout.primary_button.top);
            assert_eq!(layout.back_button.width, layout.primary_button.width);
            assert_eq!(layout.back_button.height, layout.primary_button.height);
        }
    }
    println!("PASS installer layout: all wizard states share one symmetric frame");
}
