//! Pixel and damage proof using the guest's icon overlay painter.
use super::*;
// ------------------------=
// FUNC: verify
// DESC: Checks bounded reconstruction equals full rendering across a moving drag and emits its actual pixels.
// ------------------=
pub fn verify() {
    let (width,height)=(1280,720);
    let background=0x3a1b10;
    let mut pixels=vec![background;width*height];
    let mut display=DisplayDevice{buffer:pixels.as_mut_ptr(),width,height,stride:width,
        format:0,render_clip:None,fast_motion_frame:true,submissions:0,
        recording_surface:false,template_typography:[0;3]};
    display.desktop_trash_tile(1);
    let tile=ui::desktop_polish::trash_rect(width,height,1);
    let mut ppm=format!("P6\n{} {}\n255\n",tile.width,tile.height).into_bytes();
    let mut changed=0;
    for y in tile.y..tile.bottom() {
        for x in tile.x..tile.right() {
            let pixel=pixels[y as usize*width+x as usize];
            changed+=usize::from(pixel!=background);
            ppm.extend_from_slice(&[pixel as u8,(pixel>>8)as u8,(pixel>>16)as u8]);
        }
    }
    assert!(changed>200 && changed<(tile.width*tile.height) as usize/2);
    std::fs::create_dir_all("builds/validation/desktop-trash").unwrap();
    std::fs::write("builds/validation/desktop-trash/trash.ppm",ppm).unwrap();
    pixels.fill(background);
    let start=Instant::now();
    for step in 0..120 {
        let now=step*16;
        ui::desktop_drag::motion(0,[100+step as i32*4,300],Some([100,300]),now,false);
        let damage=ui::desktop_drag::take_damage(width,height).unwrap();
        for y in damage.y.max(0)..damage.bottom().min(height as i32) {
            for x in damage.x.max(0)..damage.right().min(width as i32) {pixels[y as usize*width+x as usize]=background;}
        }
        display.desktop_drag_overlay();
        let mut expected=vec![background;width*height];
        let mut reference=display;reference.buffer=expected.as_mut_ptr();reference.desktop_drag_overlay();
        assert_eq!(pixels,expected,"drag frame {step} left stale pixels");
    }
    println!("120 bounded drag frames with full pixel comparison: {:?}",start.elapsed());
    let mut ppm=format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in &pixels {ppm.extend_from_slice(&[(pixel>>16)as u8,(pixel>>8)as u8,*pixel as u8]);}
    std::fs::write("build/desktop-drag-proof.ppm",ppm).unwrap();
    ui::desktop_drag::finish(false,2000,true);
    let damage=ui::desktop_drag::take_damage(width,height).unwrap();
    for y in damage.y.max(0)..damage.bottom().min(height as i32) {
        for x in damage.x.max(0)..damage.right().min(width as i32) {pixels[y as usize*width+x as usize]=background;}
    }
    assert!(pixels.iter().all(|p|*p==background));
}
