//! Configuration face check using existing Eclipse Access/Precision surfaces.
use super::{DisplayDevice,access_style as palette};
use crate::{runtime::face_setup::{Stage,PREVIEW_WIDTH,PREVIEW_HEIGHT},ui::face_setup_layout::Layout};

// ------------------------=
// FUNC: face_preview_present
// DESC: Updates only live camera pixels, preserving cursor backing and avoiding whole-scene blur and artwork repaint.
// ------------------=
#[cfg(target_os="none")]
pub fn face_preview_present() {unsafe {
    let Some(console)=(*core::ptr::addr_of_mut!(super::CONSOLE)).as_mut() else{return;};
    let r=Layout::new(console.display.width,console.display.height).preview;
    console.restore_cursor();
    crate::runtime::face_setup_host::view(|state| {
        if let Some(pixels)=state.preview() {
            for y in 0..r.height as usize {for x in 0..r.width as usize {
                let i=((y*PREVIEW_HEIGHT/r.height as usize)*PREVIEW_WIDTH+x*PREVIEW_WIDTH/r.width as usize)*3;
                console.display.pixel(r.x+x as i32,r.y+y as i32,pixels[i],pixels[i+1],pixels[i+2]);
            }}
        }
    });
    console.display.mark_dirty_rect(r.x as usize,r.y as usize,r.width as usize,r.height as usize);
    console.save_and_draw_cursor(console.cursor_x,console.cursor_y);
    console.display.present_damage();
}}
impl DisplayDevice {
    // ------------------------=
    // FUNC: face_setup_frame
    // DESC: Separates real camera thumbnails from the illustrative mask and truthful typed setup progress.
    // ------------------=
    #[cfg(target_os="none")]
    pub(super) fn face_setup_frame(&mut self,input:&[u8],focus:usize,error:bool) {
        crate::runtime::face_setup_host::view(|state|self.face_setup_frame_state(input,focus,error,state,
            crate::runtime::face_setup_host::available()));
    }
    // ------------------------=
    // FUNC: face_setup_frame_state
    // DESC: Paints the production enrollment UI from explicit bounded state, shared with pixel verification without a simulated camera.
    // ------------------=
    pub(super) fn face_setup_frame_state(&mut self,input:&[u8],focus:usize,error:bool,
        state:&crate::runtime::face_setup::Setup,camera_available:bool) {
        let l=Layout::new(self.width,self.height);
        let r=l.card;self.access_panel(r.x as usize,r.y as usize,r.width as usize,r.height as usize);
        let bounds=|r:crate::ui::geometry::Rect|(r.x as usize,r.y as usize,r.width as usize,r.height as usize);
        self.access_label(bounds(l.title),b"Create your facial identity",l.pixels*3/2,palette::TEXT,false,true);
            let (guidance,action):(&[u8],&[u8])=match state.stage() {
                Stage::Idle|Stage::Cancelled=>(b"The camera preview starts on this screen. Keep one person in the frame. Only Capture takes an enrollment photo.",b"Retry camera preview"),
                Stage::Loading=>(b"Checking the camera, then preparing private recognition. The real preview appears below; enrollment photos are captured separately.",b"Preparing..."),
                Stage::Photos(0)=>(b"Photo 1 of 3. Face the camera straight on. Use even lighting and keep your whole face visible.",b"Capture front view"),
                Stage::Photos(1)=>(b"Photo 2 of 3. Turn your face slightly to your left. Keep your eyes visible and hold still.",b"Capture left view"),
                Stage::Photos(_)=>(b"Photo 3 of 3. Turn slightly to your right. Avoid backlighting and motion blur.",b"Capture right view"),
                Stage::Review=>(b"Three views accepted. Verify with three fresh comparisons, then save encrypted face templates and portraits to your account on this device.",b"Verify & save identity"),
                Stage::Verifying(_)=>(b"Hold still and face the camera. Comparing three fresh captures against all enrolled views.",b"Verifying..."),
                Stage::Passed=>(b"Facial identity saved and verified from storage. Front and angled side portraits are encrypted locally. Password login remains on.",b"Continue"),
                Stage::Failed(_)=>(b"The camera check did not complete. Check camera access, lighting, focus and pose. Retry starts a clean enrollment.",b"Retry camera check"),
            };
            let (guidance,action)=if state.pending() && matches!(state.stage(),Stage::Photos(_)) {
                if state.checking_photo() {
                    (b"Photo captured. Checking face, lighting and pose. An accepted view advances automatically; otherwise, capture this view again." as &[u8],b"Checking photo..." as &[u8])
                } else {
                    (b"Waiting for a fresh camera frame. Face the camera and hold still." as &[u8],b"Capturing..." as &[u8])
                }
            } else {(guidance,action)};
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
            self.access_label((r.x as usize,r.y as usize-l.pixels-8,r.width as usize,l.pixels+4),
                if matches!(state.stage(),Stage::Loading|Stage::Photos(_)) && !state.checking_photo(){b"Live camera preview"}else{b"Camera capture"},l.pixels*4/5,palette::SECONDARY,false,false);
            self.access_surface(r.x as usize,r.y as usize,r.width as usize,r.height as usize,
                crate::ui::panel_recipe::Surface::Field,palette::BACKGROUND,palette::BORDER,255,false);
            if let Some(pixels)=state.preview() {
                for y in 0..r.height as usize {for x in 0..r.width as usize {
                    let index=((y*PREVIEW_HEIGHT/r.height as usize)*PREVIEW_WIDTH+x*PREVIEW_WIDTH/r.width as usize)*3;
                    self.pixel(r.x+x as i32,r.y+y as i32,pixels[index],pixels[index+1],pixels[index+2]);
                }}
            } else {
                self.access_label(bounds(r),if state.pending(){b"Starting camera..."}else{b"Camera unavailable"},
                    l.pixels*3/5,palette::SECONDARY,true,false);
            }
            self.face_mask_base(l.mask);
            // Reuse the kit's image-backed field surfaces for real accepted photos.
            let gap=l.pixels;
            let left=l.preview.x as usize+l.preview.width as usize+gap;
            let available=(l.field.x as usize+l.field.width as usize).saturating_sub(left);
            let width=available.saturating_sub(gap*2)/3;
            if width>0 {
                for (index,label) in [b"Front" as &[u8],b"Left",b"Right"].into_iter().enumerate() {
                    let x=left+index*(width+gap);let y=l.preview.y as usize;let height=width*3/4;
                    self.access_surface(x,y,width,height,crate::ui::panel_recipe::Surface::Field,
                        palette::BACKGROUND,palette::BORDER,255,false);
                    if let Some(photo)=state.photo(index) {
                        for py in 0..height {for px in 0..width {
                            let offset=((py*crate::runtime::face_setup::PHOTO_HEIGHT/height)
                                *crate::runtime::face_setup::PHOTO_WIDTH+px*crate::runtime::face_setup::PHOTO_WIDTH/width)*3;
                            self.pixel((x+px) as i32,(y+py) as i32,photo[offset],photo[offset+1],photo[offset+2]);
                        }}
                    }
                    self.access_label((x,y+height+gap/2,width,gap),label,l.pixels*3/5,palette::SECONDARY,true,false);
                    self.access_label((x,y+height+gap*2,width,gap),
                        match state.photo_status(index) {
                            crate::runtime::face_setup::PhotoStatus::Checking=>b"Checking...",
                            crate::runtime::face_setup::PhotoStatus::Accepted=>b"Accepted",
                            crate::runtime::face_setup::PhotoStatus::Pending=>b"Pending",
                        },
                        l.pixels*3/5,palette::SECONDARY,true,false);
                }
            }
            let note:&[u8]=if error {b"Unable to start. A supported camera, valid account password and a face name are required."}
                else if let Some(code)=state.rejection().or_else(||if let Stage::Failed(code)=state.stage(){Some(code)}else{None}) {match code {
                    104=>b"Camera disconnected or could not deliver a complete frame. Check the VM camera attachment.",
                    105 if matches!(state.stage(),Stage::Photos(_))=>b"No fresh camera frame arrived in time. Accepted views are kept. Check the camera attachment, then capture this view again.",
                    105=>b"No fresh camera frame arrived in time. Check the camera attachment and retry.",
                    111|220=>b"The enrollment session expired. Retry when you are ready.",
                    114=>b"Recognition models took too long to prepare. No identity was saved. Retry camera setup.",
                    115=>b"Face analysis took too long. No identity was saved. Retry camera setup.",
                    112=>b"The encrypted identity could not be saved and read back. Check storage and retry. No successful enrollment is being claimed.",
                    204|205=>b"That frame was too old or captured too soon. Earlier accepted views are kept. Hold still and capture this view again.",
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
                else if !camera_available {b"Native camera capture is unavailable on this configuration. You can skip this optional check."}
                else {b"Private, on-device enrollment. Facial sign-in defaults On in Settings. RGB matching cannot prevent photo or video replay. Password sign-in stays available."};
            self.access_text_wrapped(l.status.x as usize,l.status.y as usize,l.status.width as usize,note,
                palette::SECONDARY.0,palette::SECONDARY.1,palette::SECONDARY.2,255,l.pixels*4/5,false);
            for (r,label,id,primary) in [(l.back,b"Back" as &[u8],0,false),(l.skip,b"Skip" as &[u8],3,false),(l.primary,action,1,true)] {
                self.access_button(r.x as usize,r.y as usize,r.width as usize,r.height as usize,label,primary,focus==id);
            }
    }
}
