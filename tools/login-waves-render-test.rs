use super::display;
use crate::ui::{login_waves::{self,Frame}, installer_layout, installer_template::InstallerTemplate};

// ------------------------=
// FUNC: verify
// DESC: Measures production motion and audio pixels, proves credential-safe partial repaint, and exports screenshots across viewport sizes.
// ------------------=
pub(crate) fn verify() {
    let bytes = installer_layout::LOGIN_TEMPLATE_BYTES;
    let template = InstallerTemplate::parse_login(bytes).unwrap();
    let output = std::path::Path::new("builds/validation/login-waves");
    std::fs::create_dir_all(output).unwrap();
    for (width,height) in [(1024,768),(1920,1080),(2560,1080)] {
        let mut pixels = vec![0u32;width*height];
        let mut target = display(&mut pixels,width,height);
        let mut frames = Vec::new();
        for (name,frame) in [("idle",Frame{time_ms:4000,level:0}),
            ("idle-next",Frame{time_ms:4600,level:0}), ("speech",Frame{time_ms:4000,level:1000})] {
            login_waves::publish(frame);
            assert!(target.login_template_scene(bytes,b"Jonathan",b"private password",1));
            frames.push(pixels.clone());
            let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
            for p in &pixels { ppm.extend_from_slice(&[(*p&255)as u8,((*p>>8)&255)as u8,((*p>>16)&255)as u8]); }
            std::fs::write(output.join(format!("{width}-{name}.ppm")),ppm).unwrap();
        }
        let moved = frames[0].iter().zip(&frames[1]).filter(|(a,b)|a!=b).count();
        let reacted = frames[0].iter().zip(&frames[2]).filter(|(a,b)|a!=b).count();
        assert!(moved>width*10 && reacted>width*10);
        pixels.copy_from_slice(&frames[0]);
        login_waves::publish(Frame{time_ms:4000,level:1000});
        let before = pixels.clone();
        let started=std::time::Instant::now();
        let _ = std::hint::black_box(InstallerTemplate::parse_login(bytes).unwrap());
        let parsing=started.elapsed();
        let started=std::time::Instant::now();
        let _ = std::hint::black_box(installer_layout::login_wave_regions(&template,width,height));
        let geometry=started.elapsed();
        let started=std::time::Instant::now();
        let mut emitted=0usize;
        login_waves::paint(login_waves::current(),width,height,(0,0,width,height),|x,y,c,a|{
            std::hint::black_box((x,y,c,a)); emitted+=1;
        });
        println!("parse {parsing:?}, geometry {geometry:?}, pure paint {:?} ({emitted} pixels)",started.elapsed());
        let started=std::time::Instant::now();
        assert!(target.repaint_login_waves());
        let elapsed=started.elapsed();
        let (regions,count)=installer_layout::login_wave_regions(&template,width,height);
        assert!(count>0);
        let mut changed=0;
        for y in 0..height { for x in 0..width {
            let i=y*width+x;
            let in_damage=regions[..count].iter().any(|r|x>=r.left&&x<r.left+r.width&&y>=r.top&&y<r.top+r.height);
            if !in_damage {assert_eq!(pixels[i],before[i],"foreground modified at {x},{y}");}
            else {assert_eq!(pixels[i],frames[2][i],"partial repaint differs at {x},{y}");}
            changed+=usize::from(pixels[i]!=before[i]);
        }}
        assert!(changed>width*5 && changed<width*height/2);
        assert!(elapsed.as_millis()<50,"wave paint exceeded its host frame budget: {elapsed:?}");
        println!("{width}x{height}: {moved} idle pixels, {reacted} reactive pixels, {changed} damage pixels, {elapsed:?}");
        // Wave-only updates cannot repaint any hero pixels. Existing choreography
        // owns that rectangle and must still reproduce each original keyframe.
        let (hero_regions,hero_count)=installer_layout::login_animation_regions(&template,width,height);
        let mut hero_cost = std::time::Duration::ZERO;
        for wave in &regions[..count] { for hero in &hero_regions[..hero_count] {
            assert!(wave.left+wave.width<=hero.left || hero.left+hero.width<=wave.left
                || wave.top+wave.height<=hero.top || hero.top+hero.height<=wave.top);
        }}
        for time in [0,160,672,880,1500,2000,2400] {
            let motion=crate::ui::authentication_motion::presentation_at(time);
            assert!(target.login_template_scene_motion(bytes,b"Jonathan",b"private password",1,Some(motion)));
            let original=pixels.clone();
            login_waves::publish(Frame{time_ms:6000,level:300});
            assert!(target.repaint_login_waves());
            for r in &hero_regions[..hero_count] {for y in r.top..r.top+r.height {
                let row=y*width+r.left;
                assert_eq!(&pixels[row..row+r.width],&original[row..row+r.width]);
            }}
            assert!(target.login_template_scene_motion(bytes,b"Jonathan",b"private password",1,Some(motion)));
            let expected=pixels.clone();
            pixels.copy_from_slice(&original);
            let started=std::time::Instant::now();
            assert!(target.repaint_login_motion(motion));
            hero_cost = hero_cost.max(started.elapsed());
            for r in &hero_regions[..hero_count] {for y in r.top..r.top+r.height {
                let row=y*width+r.left;
                assert_eq!(&pixels[row..row+r.width],&expected[row..row+r.width]);
            }}
            login_waves::publish(Frame{time_ms:4000,level:1000});
        }
        println!("{width}x{height}: maximum hero repaint {hero_cost:?}");
        let mut still=None;
        for time in [0,1000,4000,24000] {
            login_waves::publish(login_waves::advance(login_waves::current(),time,Some(16000),true));
            assert!(target.login_template_scene(bytes,b"Jonathan",b"private password",1));
            if let Some(expected)=&still {assert_eq!(&pixels,expected);} else {still=Some(pixels.clone());}
        }
    }
    login_waves::publish(Frame::default());
}
