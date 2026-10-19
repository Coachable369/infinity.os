#[path="../kernel/ui/browser_glyphs.rs"] mod glyphs;

#[test]
// ------------------------=
// FUNC: glyph_pixels_and_states
// DESC: Exercises every atlas cell at production sizes, transparent bounds and disabled-state coverage.
// ------------------=
fn glyph_pixels_and_states() {
    for size in [16,20,24,32,48,64] {for id in 0..17 {
        let mut coverage=0u64;
        for y in 0..size {for x in 0..size {
            let normal=glyphs::pixel(id,size,x,y,true);
            let disabled=glyphs::pixel(id,size,x,y,false);
            coverage+=normal.3 as u64;
            assert_eq!(disabled.3 as u32,normal.3 as u32*38/100);
            assert_eq!((normal.0,normal.1,normal.2),(disabled.0,disabled.1,disabled.2));
        }}
        assert!(coverage>255);
        assert_eq!(glyphs::pixel(id,size,0,0,true).3,0);
        assert_eq!(glyphs::pixel(id,size,size,0,true),(0,0,0,0));
    }}
    assert_eq!(glyphs::pixel(17,24,12,12,true),(0,0,0,0));
    assert_eq!(glyphs::pixel(0,0,0,0,true),(0,0,0,0));
}
