//! Byte fixtures from the production OS sampler, never from Studio's preview implementation.
#![allow(dead_code)]
#[path = "../kernel/ui/panel_recipe.rs"] pub mod panel_recipe;
mod ui { pub use crate::panel_recipe; }
#[path = "../kernel/ui/dock_material.rs"] mod dock_material;
use panel_recipe::{Material, Surface};
use std::io::{Seek, SeekFrom, Write};

// ------------------------=
// FUNC: main
// DESC: Emits typed native RGBA witnesses across surface geometry, scale, tint, opacity and interaction states for Studio parity tests.
// ------------------=
fn main() -> std::io::Result<()> {
    let path = std::env::args().nth(1).expect("explicit repository fixture path");
    let surfaces = [Surface::Frame, Surface::WindowFill, Surface::Panel, Surface::Primary,
        Surface::Secondary, Surface::Field, Surface::Current, Surface::Inactive];
    let mut records = Vec::new();
    for (kind, surface) in surfaces.into_iter().enumerate() {
        for scale in [1usize, 2, 4] {
            for (width, height) in [(1, 1), (17, 9), (112, 64), (358, 96)] {
                let (width, height) = (width * scale, height * scale);
                for focus in [0, 93, 255] { for pressed in [false, true] {
                    for (opacity, edge_opacity) in [(255, 255), (0, 255), (127, 61), (255, 0)] {
                        for custom in [false, true] {
                            let fill = if custom {(79,33,67)} else if matches!(surface, Surface::Primary|Surface::Current) {(62,111,255)} else {(16,27,58)};
                            let edge = if custom {(255,188,101)} else {(134,199,255)};
                            let material = Material { fill, edge, opacity, edge_opacity, focus, pressed, scale };
                            for y in [0, 1.min(height - 1), height / 2, height - 1, height] {
                                for x in [0, 1.min(width - 1), width / 2, width - 1, width] {
                                    records.extend_from_slice(&[kind as u8, focus, pressed as u8, opacity, edge_opacity, scale as u8]);
                                    for value in [width, height, x, y] { records.extend_from_slice(&(value as u16).to_le_bytes()); }
                                    records.extend_from_slice(&[fill.0, fill.1, fill.2, edge.0, edge.1, edge.2]);
                                    let pixel = panel_recipe::pixel_styled(surface, width, height, x, y, material, panel_recipe::SurfaceStyle::AUTHORED);
                                    records.extend_from_slice(&[pixel.0, pixel.1, pixel.2, pixel.3]);
                                }
                            }
                        }
                    }
                }}
            }
        }
    }
    let mut file = std::fs::File::create(&path)?;
    file.write_all(b"IPARITY1")?;
    file.write_all(&((records.len() / 24) as u32).to_le_bytes())?;
    file.write_all(&records)?;
    write_frost(&std::path::Path::new(&path).with_extension("frost"))?;
    write_styles(&std::path::Path::new(&path).with_extension("styles"))?;
    write_frost_styles(&std::path::Path::new(&path).with_extension("stylefrost"))?;
    Ok(())
}

// ------------------------=
// FUNC: write_styles
// DESC: Exercises native selectable materials, all corner families, radii, state transitions and independent ARGB overrides as typed RGBA records.
// ------------------=
fn write_styles(path: &std::path::Path) -> std::io::Result<()> {
    let mut records = Vec::new();
    let surfaces = [Surface::Frame, Surface::WindowFill, Surface::Panel, Surface::Primary,
        Surface::Secondary, Surface::Field, Surface::Current, Surface::Inactive];
    for (kind, surface) in surfaces.into_iter().enumerate() {
        for material_kind in 0..=2 { for corners in 0..=2 { for radius in [0u8, 4, 18, 40] {
            for scale in [1usize, 2] { for (focus, pressed) in [(0u8,false),(93,false),(255,true)] {
                for (width, height) in [(1usize,1usize),(37,21),(173,94)] { for tint in [false,true] {
                    let fill=if kind==3 {(62,111,255)}else{(16,27,58)};
                    let edge=(134,199,255);
                    let material=Material{fill,edge,opacity:173,edge_opacity:219,focus,pressed,scale};
                    let style=panel_recipe::SurfaceStyle{material:material_kind,corners,radius,
                        fill:if tint{Some(0x814f2143)}else{None},border:if tint{Some(0xc9ffbc65)}else{None}};
                    for y in [0,1.min(height-1),height/2,height-1,height] {
                        for x in [0,1.min(width-1),width/2,width-1,width] {
                            records.extend_from_slice(&[kind as u8,focus,pressed as u8,173,219,scale as u8]);
                            for value in [width,height,x,y] {records.extend_from_slice(&(value as u16).to_le_bytes());}
                            records.extend_from_slice(&[fill.0,fill.1,fill.2,edge.0,edge.1,edge.2]);
                            let p=panel_recipe::pixel_styled(surface,width,height,x,y,material,style);
                            records.extend_from_slice(&[p.0,p.1,p.2,p.3]);
                            records.extend_from_slice(&[material_kind,corners,radius,if tint{3}else{0}]);
                            records.extend_from_slice(&style.fill.unwrap_or(0).to_le_bytes());
                            records.extend_from_slice(&style.border.unwrap_or(0).to_le_bytes());
                        }
                    }
                }}
            }}
        }}}
    }
    let mut file=std::fs::File::create(path)?;
    file.write_all(b"ISTYLE01")?;
    file.write_all(&((records.len()/36) as u32).to_le_bytes())?;
    file.write_all(&records)?;
    Ok(())
}

// ------------------------=
// FUNC: write_frost
// DESC: Records real native frost composites over deterministic high-frequency RGB input for full-image Swift comparison.
// ------------------=
fn write_frost(path: &std::path::Path) -> std::io::Result<()> {
    let mut file = std::fs::File::create(path)?;
    file.write_all(b"IFROST01")?;
    file.write_all(&32u32.to_le_bytes())?;
    let (width, height) = (131usize, 71usize);
    for opacity in [0u8, 127, 230, 255] { for edge in [0u8,255] { for scale in [1usize,2] { for custom in [false,true] {
        let fill=if custom {(79u8,33u8,67u8)}else{(16,27,58)};
        let border=if custom {(255u8,188u8,101u8)}else{(134,199,255)};
        let mut pixels=Vec::new();
        for y in 0..height { for x in 0..width {
            let red=((x*31+y*7)%256) as u32;
            let green=((x*3+y*29)%256) as u32;
            let blue=((x*19+y*11)%256) as u32;
            pixels.push(0xff000000|red<<16|green<<8|blue);
        }}
        let material=dock_material::Material{primary:(fill.0 as u32)<<16|(fill.1 as u32)<<8|fill.2 as u32,
            accent:(border.0 as u32)<<16|(border.1 as u32)<<8|border.2 as u32,
            opacity:((u16::from(opacity)*100+127)/255) as u8,blur:12,scale,bgr:true};
        dock_material::Backdrop::new().paint_styled_with_edge_opacity(&mut pixels,width,(0,0,width,height),(0,0,width,height),material,edge,panel_recipe::SurfaceStyle::AUTHORED);
        file.write_all(&[fill.0,fill.1,fill.2,border.0,border.1,border.2,opacity,edge,scale as u8,12])?;
        file.write_all(&(width as u16).to_le_bytes())?;
        file.write_all(&(height as u16).to_le_bytes())?;
        for p in pixels {file.write_all(&[(p>>16) as u8,(p>>8) as u8,p as u8,(p>>24) as u8])?;}
    }}}}
    Ok(())
}

// ------------------------=
// FUNC: write_frost_styles
// DESC: Captures complete native glass and matte panels with custom antialiased geometry and translucent independent tint overrides.
// ------------------=
fn write_frost_styles(path: &std::path::Path) -> std::io::Result<()> {
    let mut file = std::fs::File::create(path)?;
    file.write_all(b"ISTFRO01")?;
    file.write_all(&0u32.to_le_bytes())?;
    let mut count = 0u32;
    let (width,height)=(131usize,71usize);
    for material_kind in 0..=2 {for corners in 0..=2 {for radius in [0u8,12,40] {for scale in [1usize,2] {for tint in [false,true] {
        let fill=(16u8,27u8,58u8);let border=(134u8,199u8,255u8);
        let opacity=230u8;let edge=197u8;
        let mut pixels=Vec::new();
        for y in 0..height {for x in 0..width {
            pixels.push(0xff000000|(((x*31+y*7)%256)as u32)<<16|(((x*3+y*29)%256)as u32)<<8|((x*19+y*11)%256)as u32);
        }}
        let material=dock_material::Material{primary:0x101b3a,accent:0x86c7ff,
            opacity:((u16::from(opacity)*100+127)/255)as u8,blur:12,scale,bgr:true};
        let style=panel_recipe::SurfaceStyle{material:material_kind,corners,radius,
            fill:if tint{Some(0x814f2143)}else{None},border:if tint{Some(0xc9ffbc65)}else{None}};
        dock_material::Backdrop::new().paint_styled_with_edge_opacity(&mut pixels,width,(0,0,width,height),(0,0,width,height),material,edge,style);
        file.write_all(&[fill.0,fill.1,fill.2,border.0,border.1,border.2,opacity,edge,scale as u8,12])?;
        file.write_all(&(width as u16).to_le_bytes())?;file.write_all(&(height as u16).to_le_bytes())?;
        file.write_all(&[material_kind,corners,radius,if tint{3}else{0}])?;
        file.write_all(&style.fill.unwrap_or(0).to_le_bytes())?;file.write_all(&style.border.unwrap_or(0).to_le_bytes())?;
        for p in pixels {file.write_all(&[(p>>16)as u8,(p>>8)as u8,p as u8,(p>>24)as u8])?;}
        count += 1;
    }}}}}
    file.seek(SeekFrom::Start(8))?;
    file.write_all(&count.to_le_bytes())?;
    Ok(())
}
