// ------------------------=
// FUNC: zoom_stress_pixels
// DESC: Requires the real rendered marker width to follow page zoom after every layout pass.
// ------------------=
fn zoom_stress_pixels(engine:&Servo,view:&WebView,repaint:&Cell<bool>,percent:u16)->bool {
    let done=Rc::new(Cell::new(0));let result=done.clone();
    view.take_screenshot(None,move |image| {
        let valid=image.is_ok_and(|image| {
            if image.width()!=480 || image.height()!=320 {return false;}
            let width=image.enumerate_pixels().filter(|(x,y,pixel)|
                *y==2 && *x<480 && pixel.0==[0,255,0,255]).count();
            // Fractional device-pixel edges can be antialiased or snapped.
            width.abs_diff(80*percent as usize/100)<=1
        });
        result.set(if valid {1}else{2});
    });
    spin_until(engine,view,repaint,&done)
}

// ------------------------=
// FUNC: zoom_stress
// DESC: Exercises repeated native zoom, reset and re-layout of a wide DOM with real geometry and pixels.
// ------------------=
fn zoom_stress(engine:&Servo,view:&WebView,repaint:&Cell<bool>)->bool {
    if !zoom_stress_setup(engine,view,repaint,r#"
        document.head.innerHTML='<style>body{margin:0;font:16px sans-serif}main{display:flex;flex-wrap:wrap}article{width:140px;padding:6px;border:1px solid #345}article:nth-child(3n){font-weight:bold}ul{padding-left:18px}#zoomMarker{position:fixed;z-index:9999;left:0;top:0;width:80px;height:16px;background:rgb(0,255,0)}</style>';
        document.body.innerHTML='<div id="zoomMarker"></div><main id="results"></main>';
        const fragment=document.createDocumentFragment();
        for(let i=0;i<256;i++){
            const item=document.createElement('article');
            item.innerHTML='<h3>Result '+i+'</h3><p>Native page layout and text reflow <b>sample</b></p><ul><li>First row</li><li>Second row</li></ul>';
            fragment.appendChild(item);
        }
        document.getElementById('results').appendChild(fragment);
        document.querySelectorAll('article').length===256 && document.body.scrollHeight>320
    "#){return false;}
    if !zoom_stress_pixels(engine,view,repaint,100){return false;}
    let mut percent=100;
    // Traverse both bounded extremes and reset, not only alternating 100/110.
    let directions=[1,1,1,1,1,1,1,1,0,-1,-1,-1,-1,-1,-1,0];
    for (index,direction) in directions.into_iter().enumerate() {
        percent=super::super::zoom_steps::next(percent,direction);
        super::super::record(2,0x600,((index as u64)<<32)|percent as u64);
        view.set_page_zoom(percent as f32/100.0);
        let query=std::format!(
            "Math.abs(devicePixelRatio-{})<0.01 && Math.abs(innerWidth*devicePixelRatio-480)<4 && document.getElementById('zoomMarker').getBoundingClientRect().width===80 && document.querySelectorAll('article').length===256",
            percent as f32/100.0);
        let deadline=Instant::now()+Duration::from_secs(10);
        while !javascript_true(engine,view,repaint,&query) {
            if Instant::now()>=deadline{return false;}
            std::thread::sleep(Duration::from_millis(10));
        }
        if !zoom_stress_pixels(engine,view,repaint,percent){return false;}
        super::super::record(2,0x601,((index as u64)<<32)|percent as u64);
    }
    javascript_true(engine,view,repaint,"devicePixelRatio===1 && innerWidth===480")
}
// ------------------------=
// FUNC: zoom_stress_setup
// DESC: Allows bounded native debug-build DOM construction without changing production scheduling or the guest watchdog.
// ------------------=
fn zoom_stress_setup(engine:&Servo,view:&WebView,repaint:&Cell<bool>,script:&str)->bool {
    let done=Rc::new(Cell::new(0));let result=done.clone();
    view.evaluate_javascript(script,move |value| {
        result.set(if matches!(value,Ok(servo::JSValue::Boolean(true))){1}else{2});
    });
    let deadline=Instant::now()+Duration::from_secs(45);
    while done.get()==0 && Instant::now()<deadline {
        engine.spin_event_loop();
        if repaint.replace(false){view.paint();}
        std::thread::sleep(Duration::from_millis(1));
    }
    super::super::record(2,0x602,done.get() as u64);
    done.get()==1
}

