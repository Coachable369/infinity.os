//! Configuration face check using existing Eclipse Access/Precision surfaces.
use super::{DisplayDevice,access_style as palette};
use crate::{runtime::face_setup::{Stage,PREVIEW_WIDTH,PREVIEW_HEIGHT},ui::face_setup_layout::Layout};
impl DisplayDevice {
    // ------------------------=
    // FUNC: face_setup_frame
    // DESC: Renders real camera thumbnails and typed setup progress; no decorative face or security-success simulation is shown.
    // ------------------=
    pub(super) fn face_setup_frame(&mut self,input:&[u8],focus:usize,error:bool) {
        let l=Layout::new(self.width,self.height);
        let r=l.card;self.onboarding_glass_panel(r.x as usize,r.y as usize,r.width as usize,r.height as usize);
        let bounds=|r:crate::ui::geometry::Rect|(r.x as usize,r.y as usize,r.width as usize,r.height as usize);
        self.access_label(bounds(l.title),b"Check facial recognition",l.pixels*3/2,palette::TEXT,false,true);
        crate::runtime::face_setup_host::view(|state| {
            let (guidance,action):(&[u8],&[u8])=match state.stage() {
                Stage::Idle|Stage::Cancelled=>(b"Optional camera check. Name this face, then capture three clear views. Keep one person in the frame.",b"Start camera check"),
                Stage::Loading=>(b"Preparing the private recognition worker. The camera stays off until you choose Capture.",b"Preparing..."),
                Stage::Photos(0)=>(b"Photo 1 of 3. Face the camera straight on. Use even lighting and keep your whole face visible.",b"Capture front view"),
                Stage::Photos(1)=>(b"Photo 2 of 3. Turn your face slightly to your left. Keep your eyes visible and hold still.",b"Capture left view"),
                Stage::Photos(_)=>(b"Photo 3 of 3. Turn slightly to your right. Avoid backlighting and motion blur.",b"Capture right view"),
                Stage::Review=>(b"Three views accepted. Face the camera again for three fresh comparisons. These are new captures, not the enrollment photos.",b"Verify with camera"),
                Stage::Verifying(_)=>(b"Hold still and face the camera. Comparing three fresh captures against all enrolled views.",b"Verifying..."),
                Stage::Passed=>(b"Camera comparison passed. This does not certify liveness or enable facial login. Password login remains on.",b"Continue"),
                Stage::Failed(_)=>(b"The camera check did not complete. Check camera access, lighting, focus and pose. Retry starts a clean enrollment.",b"Retry camera check"),
            };
            self.access_text_wrapped(l.guidance.x as usize,l.guidance.y as usize,l.guidance.width as usize,
                guidance,palette::TEXT.0,palette::TEXT.1,palette::TEXT.2,255,l.pixels,false);
            if state.stage()==Stage::Review {
                self.access_button(l.field.x as usize,l.field.y as usize,l.field.width as usize,l.field.height as usize,
                    b"Retake photos",false,focus==2);
            } else {
                self.onboarding_input_field_with_element(None,l.field.x as usize,l.field.y as usize,
                    l.field.width as usize,l.field.height as usize,input,false,focus==2,b"Face name (for example: Everyday)");
            }
            let r=l.preview;
            self.access_surface(r.x as usize,r.y as usize,r.width as usize,r.height as usize,
                crate::ui::panel_recipe::Surface::Field,palette::BACKGROUND,palette::BORDER,255,false);
            if let Some(pixels)=state.preview() {
                for y in 0..r.height as usize {for x in 0..r.width as usize {
                    let index=((y*PREVIEW_HEIGHT/r.height as usize)*PREVIEW_WIDTH+x*PREVIEW_WIDTH/r.width as usize)*3;
                    self.pixel(r.x+x as i32,r.y+y as i32,pixels[index],pixels[index+1],pixels[index+2]);
                }}
            } else {
                self.access_label(bounds(r),if state.pending(){b"Capturing / processing"}else{b"Camera preview appears after capture"},
                    l.pixels,palette::SECONDARY,true,false);
            }
            let note:&[u8]=if error {b"Unable to start. A supported camera, valid account password and a face name are required."}
                else if let Stage::Failed(code)=state.stage() {match code {
                    104=>b"Camera disconnected or could not deliver a complete frame. Check the VM camera attachment.",
                    105|111|220=>b"The camera check timed out or was cancelled. Retry when you are ready.",
                    206=>b"No face detected. Center your face and improve the lighting.",
                    207=>b"More than one face detected. Only one person may be in the frame.",
                    208=>b"Move closer: the face is too small for a reliable comparison.",
                    210|213=>b"The image is too dark or low contrast. Face a soft light source.",
                    211|212=>b"The face is overexposed or cropped. Reduce glare and keep the whole face visible.",
                    214=>b"The image is blurred. Hold still and check camera focus.",
                    216=>b"The face angle does not match the requested view. Follow the capture guidance.",
                    217|218=>b"The faces did not match consistently. Retry with the same person throughout.",
                    _=>b"The recognition worker could not complete. No enrollment or login change was saved.",
                }}
                else if !crate::runtime::face_setup_host::available() {b"Native camera capture is unavailable on this configuration. You can skip this optional check."}
                else {b"Temporary check only: photos and biometric data are discarded. Nothing is saved for login."};
            self.access_text_wrapped(l.status.x as usize,l.status.y as usize,l.status.width as usize,note,
                palette::SECONDARY.0,palette::SECONDARY.1,palette::SECONDARY.2,255,l.pixels*4/5,false);
            for (r,label,id,primary) in [(l.back,b"Back" as &[u8],0,false),(l.skip,b"Skip" as &[u8],3,false),(l.primary,action,1,true)] {
                self.access_button(r.x as usize,r.y as usize,r.width as usize,r.height as usize,label,primary,focus==id);
            }
        });
    }
}
