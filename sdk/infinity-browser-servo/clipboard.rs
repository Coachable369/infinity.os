//! Gesture-gated native text clipboard delegate, shared by the component and engine fixture.
use servo::{ClipboardDelegate,StringRequest,WebView};
use std::string::String;

pub struct NativeClipboard {pub read:fn(&mut [u8])->Option<usize>,pub write:fn(&[u8])->bool}
impl ClipboardDelegate for NativeClipboard {
    // ------------------------=
    // FUNC: clear
    // DESC: Clears only from a focused view through an authorized native clipboard write.
    // ------------------=
    fn clear(&self,view:WebView) {if view.focused() {(self.write)(b"");}}
    // ------------------------=
    // FUNC: get_text
    // DESC: Answers focused-view paste requests with complete UTF-8 or explicit denial, without a fallback cache.
    // ------------------=
    fn get_text(&self,view:WebView,request:StringRequest) {
        if !view.focused() {request.failure("Clipboard access denied".into());return;}
        let mut bytes=std::vec![0;16384];
        let Some(n)=(self.read)(&mut bytes).filter(|n|*n<=bytes.len()) else {request.failure("Clipboard access denied".into());return;};
        bytes.truncate(n);
        match String::from_utf8(bytes) {Ok(text)=>request.success(text),Err(_)=>request.failure("Invalid clipboard text".into())}
    }
    // ------------------------=
    // FUNC: set_text
    // DESC: Routes normal copy through the same admitted write used by transactional cut.
    // ------------------=
    fn set_text(&self,view:WebView,text:String) {let _=self.try_set_text(view,text);}
    // ------------------------=
    // FUNC: try_set_text
    // DESC: Acknowledges storage before the engine may delete selected text for cut.
    // ------------------=
    fn try_set_text(&self,view:WebView,text:String)->bool {view.focused() && text.len()<=16384 && (self.write)(text.as_bytes())}
}
