//! Binary parity assertion: every generated cursor is embedded in both boot paths.
use std::{env,fs};
// ------------------------=
// FUNC: main
// DESC: Verifies exact RGBA payload bytes in live and installed ELF kernels; no text or source-name oracle.
// ------------------=
fn main() {
    let args:Vec<_>=env::args().skip(1).collect();
    assert_eq!(args.len(),2,"Provide installed and live kernel ELF paths");
    let names=["classic-white","classic-black","outline","crystal","silver","comet","rocket","leaf","wand","pixel"];
    for path in args {
        let elf=fs::read(&path).unwrap();assert_eq!(&elf[..4],&[127,69,76,70]);
        for name in names {
            let art=fs::read(format!("assets/cursors/{name}.rgba")).unwrap();
            assert_eq!(art.len(),128*128*4);
            let anchor=art.chunks_exact(4).position(|p|p[3]>200).unwrap()*4;
            let key=&art[anchor..anchor+16];
            let found=elf.windows(16).enumerate().filter(|(_,s)|*s==key).any(|(i,_)| {
                i>=anchor && elf.get(i-anchor..i-anchor+art.len())==Some(art.as_slice())
            });
            assert!(found,"Generated cursor missing from {path}: {name}");
        }
    }
    println!("PASS: all ten exact cursor sprites in both installed and live kernels");
}
