use super::*;

// ------------------------=
// FUNC: click
// DESC: Dispatches a complete physical click and waits for both engine acknowledgments.
// ------------------=
fn click(engine:&Servo, view:&WebView, repaint:&Cell<bool>, input:&Cell<u8>, x:f32, y:f32)->bool {
    input.set(0);
    for action in [servo::MouseButtonAction::Down,servo::MouseButtonAction::Up] {
        view.notify_input_event(servo::InputEvent::MouseButton(servo::MouseButtonEvent::new(
            action,servo::MouseButton::Primary,servo::WebViewPoint::Device((x,y).into()))));
    }
    spin_until(engine,view,repaint,input)
}

// ------------------------=
// FUNC: key
// DESC: Exercises actual engine editing with a complete modifier-bearing key pair.
// ------------------=
fn key(engine:&Servo,view:&WebView,repaint:&Cell<bool>,input:&Cell<u8>,key:servo::Key,modifiers:servo::Modifiers)->bool {
    input.set(0);
    for state in [servo::KeyState::Down,servo::KeyState::Up] {
        let mut event=servo::KeyboardEvent::from_state_and_key(state,key.clone());
        event.event.modifiers=modifiers;
        view.notify_input_event(servo::InputEvent::Keyboard(event));
    }
    spin_until(engine,view,repaint,input)
}

// ------------------------=
// FUNC: selection_pixels
// DESC: Requires a rendered selection highlight and exports the actual guest frame for visual review.
// ------------------=
fn selection_pixels(engine:&Servo,view:&WebView,repaint:&Cell<bool>)->bool {
    let done=Rc::new(Cell::new(0));let result=done.clone();
    view.take_screenshot(None,move |image| {
        let Ok(image)=image else{result.set(2);return;};
        let selected=image.pixels().filter(|pixel|pixel.0==[5,100,200,255]).count();
        super::super::record(2,42,selected as u64);
        super::super::record(2,36,((image.width() as u64)<<32)|image.height() as u64);
        for (index,chunk) in image.as_raw().chunks(8).enumerate() {
            let mut bytes=[0;8];bytes[..chunk.len()].copy_from_slice(chunk);
            super::super::record(5,index as u64,u64::from_le_bytes(bytes));
        }
        result.set(if selected>100 {1}else{2});
    });
    spin_until(engine,view,repaint,&done)
}

// ------------------------=
// FUNC: verify
// DESC: Checks selection, editing, zoom layout and tab isolation using native engine DOM state.
// ------------------=
pub fn verify(engine:&Servo)->bool {
    let Ok(context)=SoftwareRenderingContext::new((480,320).into()) else{return false;};
    if context.make_current().is_err(){return false;}
    let repaint=Rc::new(Cell::new(false));let loaded=Rc::new(Cell::new(0));let input=Rc::new(Cell::new(0));
    let view=WebViewBuilder::new(engine,Rc::new(context)).delegate(Rc::new(PageDelegate{
        repaint:repaint.clone(),loaded:loaded.clone(),input:input.clone(),
    })).url("data:text/html,<body></body>".parse().unwrap()).build();
    view.show();
    if !spin_until(engine,&view,&repaint,&loaded){return false;}
    let check=|script:&str|javascript_true(engine,&view,&repaint,script);
    if !check(r#"document.body.style.cssText='margin:0;font:20px monospace';
        document.head.innerHTML='<style>::selection{background:rgb(5,100,200);color:white}</style>';
        document.body.innerHTML='<div id="line" style="position:absolute;left:10px;top:10px;width:400px;line-height:30px">alpha <span>bravo</span> charlie</div><div id="unicode" style="position:absolute;left:10px;top:70px;width:400px;line-height:30px">caf\u00e9 omega</div><input id="entry" value="alpha bravo" style="position:absolute;left:10px;top:130px;width:400px;height:30px;font:20px monospace"><div id="blocked" style="position:absolute;left:10px;top:200px;user-select:none">blocked text</div>';
        document.getElementById('unicode').textContent='caf\u00e9 omega';
        document.body.getBoundingClientRect().width===480"#){return false;}
    // Force the real layout/hit-test barrier before physical input.
    let ready=Rc::new(Cell::new(0));let result=ready.clone();
    view.take_screenshot(None,move |image|result.set(if image.is_ok(){1}else{2}));
    if !spin_until(engine,&view,&repaint,&ready){return false;}
    for _ in 0..2 {if !click(engine,&view,&repaint,&input,24.0,24.0){return false;}}
    if !check("getSelection().anchorNode===line.firstChild && getSelection().anchorOffset===0 && getSelection().focusNode===line.firstChild && getSelection().focusOffset===5"){return false;}
    super::super::record(2,40,1);
    for _ in 0..2 {if !click(engine,&view,&repaint,&input,24.0,24.0){return false;}}
    if !check("getSelection().anchorNode===line.firstChild && getSelection().anchorOffset===0 && getSelection().focusNode===line.lastChild && getSelection().focusOffset===8"){return false;}
    super::super::record(2,40,2);
    if !selection_pixels(engine,&view,&repaint){return false;}
    for _ in 0..2 {if !click(engine,&view,&repaint,&input,24.0,84.0){return false;}}
    if !check("getSelection().anchorNode===unicode.firstChild && getSelection().anchorOffset===0 && getSelection().focusOffset===4"){return false;}
    super::super::record(2,40,3);
    for _ in 0..2 {if !click(engine,&view,&repaint,&input,24.0,144.0){return false;}}
    if !check("entry.selectionStart===0 && entry.selectionEnd===5"){return false;}
    for _ in 0..2 {if !click(engine,&view,&repaint,&input,24.0,144.0){return false;}}
    if !check("entry.selectionStart===0 && entry.selectionEnd===11"){return false;}
    if !key(engine,&view,&repaint,&input,servo::Key::Character("a".into()),servo::Modifiers::CONTROL) ||
        !key(engine,&view,&repaint,&input,servo::Key::Character("q".into()),servo::Modifiers::empty()) ||
        !check("entry.value==='q' && entry.selectionStart===1"){return false;}
    if !key(engine,&view,&repaint,&input,servo::Key::Named(servo::NamedKey::ArrowLeft),servo::Modifiers::SHIFT) ||
        !check("entry.selectionStart===0 && entry.selectionEnd===1"){return false;}
    for action in ["c","x","v"] {
        if !key(engine,&view,&repaint,&input,servo::Key::Character(action.into()),servo::Modifiers::CONTROL){return false;}
        if !check(if action=="x" {"entry.value.length===0"} else {"entry.value==='q'"}){return false;}
    }
    super::super::record(2,40,4);
    if !check("entry.blur();line.style.width='80px';line.getBoundingClientRect().width===80"){return false;}
    for _ in 0..4 {if !click(engine,&view,&repaint,&input,24.0,24.0){return false;}}
    if !check("getSelection().anchorNode===line.firstChild && getSelection().anchorOffset===0 && getSelection().focusNode===line.firstChild && getSelection().focusOffset>=5 && getSelection().focusOffset<=6"){return false;}
    super::super::record(2,40,6);
    if !check("unicode.onmousedown=e=>e.preventDefault();true"){return false;}
    for _ in 0..2 {if !click(engine,&view,&repaint,&input,24.0,84.0){return false;}}
    for _ in 0..2 {if !click(engine,&view,&repaint,&input,24.0,214.0){return false;}}
    if !check("getSelection().anchorNode===line.firstChild && getSelection().anchorOffset===0 && getSelection().focusNode===line.firstChild && getSelection().focusOffset>=5"){return false;}
    super::super::record(2,40,7);
    view.set_page_zoom(1.5);
    if !check("Math.abs(devicePixelRatio-1.5)<0.01 && innerWidth===320"){return false;}
    view.set_page_zoom(1.0);
    if !check("devicePixelRatio===1 && innerWidth===480"){return false;}
    drop(view);drain_close(engine);
    let provider=||FixtureProvider{starts:Rc::new(Cell::new(0)),cancels:Rc::new(Cell::new(0)),serial:0};
    let Ok(first)=session::Session::new(engine,provider(),super::super::monotonic,128,128) else{return false;};
    let Ok(second)=session::Session::new(engine,provider(),super::super::monotonic,128,128) else{return false;};
    first.zoom(1);
    if first.zoom_percent()!=110 || second.zoom_percent()!=100 {return false;}
    first.zoom(-1);if first.zoom_percent()!=100{return false;}
    first.zoom(-1);first.zoom(0);if first.zoom_percent()!=100{return false;}
    drop(first);drop(second);drain_close(engine);
    super::super::record(2,40,5);true
}
