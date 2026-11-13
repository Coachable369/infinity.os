//! Pixel and damage proof using the guest's icon overlay painter.
use super::*;
// ------------------------=
// FUNC: verify
// DESC: Checks bounded reconstruction equals full rendering across a moving drag and emits its actual pixels.
// ------------------=
pub fn verify() {
    verify_launcher_and_trash();
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

// ------------------------=
// FUNC: verify_launcher_and_trash
// DESC: Renders native desktop artwork at two resolutions and verifies that each hover frame changes only the focused glyph.
// ------------------=
fn verify_launcher_and_trash() {
    let out = std::path::Path::new("builds/validation/desktop-trash");
    std::fs::create_dir_all(out).unwrap();
    for (width,height) in [(1280,720),(2560,1440)] {
        let mut pixels = vec![0; width*height];
        let mut display = DisplayDevice { buffer:pixels.as_mut_ptr(),width,height,stride:width,
            format:0,render_clip:None,fast_motion_frame:false,submissions:0,
            recording_surface:false,template_typography:[0;3] };
        let scale = display.ui_scale();
        display.paint_bitmap_cover_rect(include_bytes!("../assets/desktop/infinity-shell-wallpaper-v3.bmp"),0,0,width,height);
        display.desktop_trash_tile(scale);
        let tile=ui::desktop_polish::trash_rect(width,height,scale);
        assert_eq!(width-tile.right() as usize,12*scale);
        assert_eq!(height-tile.bottom() as usize,16*scale);
        let save = |name:&str, frame:&[u32]| {
            let mut ppm=format!("P6\n{width} {height}\n255\n").into_bytes();
            for &p in frame {ppm.extend_from_slice(&[p as u8,(p>>8) as u8,(p>>16) as u8]);}
            std::fs::write(out.join(format!("{name}-{width}.ppm")),ppm).unwrap();
        };
        save("desktop",&pixels);
        let geometry=ui::system_layout::SystemLayout::new(width,height).app_launcher_geometry();
        let r=geometry.panel;
        display.blur_framebuffer_region(r.x as usize,r.y as usize,r.width as usize,r.height as usize,6);
        let glass=pixels.clone();
        display.paint_app_launcher(scale,b"",0);
        save("idle",&pixels);
        let idle=pixels.clone();
        ui::app_launcher::animate_highlight(0,128,true);
        ui::app_launcher::animate_highlight(1,0,false);
        let mut last_count=0;
        for (name,elapsed) in [("hover-mid",64),("hover-full",64)] {
            ui::app_launcher::animate_highlight(1,elapsed,false);
            pixels.copy_from_slice(&glass);
            display.paint_app_launcher(scale,b"",1);
            let mut count=0;
            for (index,(&a,&b)) in pixels.iter().zip(&idle).enumerate() {
                if a!=b {
                    let (x,y)=(index%width,index/width);
                    if y < geometry.grid_top {continue;} // Search focus also changes.
                    assert!((geometry.grid_left..geometry.grid_left+geometry.grid_cell_width).contains(&x));
                    assert!(y < geometry.grid_top+geometry.grid_row_height.min(geometry.grid_cell_width)*63/100+6*scale);
                    count+=1;
                }
            }
            assert!(count>last_count);
            last_count=count;
            save(name,&pixels);
        }
    }
}
