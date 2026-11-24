//! Configuration face check using existing Eclipse Access/Precision surfaces.
use super::{DisplayDevice,access_style as palette};
use crate::{runtime::face_setup::{Stage,PREVIEW_WIDTH,PREVIEW_HEIGHT},ui::face_setup_layout::Layout};

// ------------------------=
// FUNC: face_guidance
// DESC: Shares live capture instructions and actions between legacy and authored enrollment scenes.
// ------------------=
fn face_guidance(state:&crate::runtime::face_setup::Setup)->(&'static [u8],&'static [u8]) {
    let (guidance,action):(&[u8],&[u8])=match state.stage() {
        Stage::Idle|Stage::Cancelled=>(b"The camera preview starts on this screen. Keep one person in the frame. Only Capture takes an enrollment photo.",b"Retry camera preview"),
        Stage::Loading=>(b"Checking the camera, then preparing private recognition. The real preview appears below; enrollment photos are captured separately.",b"Preparing..."),
        Stage::Photos(0)=>(b"Photo 1 of 3. Face the camera straight on. Use even lighting and keep your whole face visible.",b"Capture front view"),
        Stage::Photos(1)=>(b"Photo 2 of 3. Turn slightly to YOUR left (nose towards the right of the preview). Keep both eyes visible; do not turn fully sideways.",b"Capture left view"),
        Stage::Photos(_)=>(b"Photo 3 of 3. Turn slightly to YOUR right (nose towards the left of the preview). Keep both eyes visible and hold still.",b"Capture right view"),
        Stage::Review=>(b"Three views accepted. Verify with three fresh comparisons, then save encrypted face templates and portraits to your account on this device.",b"Verify & save identity"),
        Stage::Verifying(_)=>(b"Hold still and face the camera. Comparing three fresh captures against all enrolled views.",b"Verifying..."),
        Stage::Passed=>(b"Facial identity saved and verified from storage. Front and angled side portraits are encrypted locally. Password login remains on.",b"Continue"),
        Stage::Failed(112|116|117)=>(b"Your captures were accepted, but encrypted storage did not complete. See the storage error below before retrying.",b"Retry enrollment"),
        Stage::Failed(_)=>(b"The camera check did not complete. Check camera access, lighting, focus and pose. Retry starts a clean enrollment.",b"Retry camera check"),
    };
    let (guidance,action)=if state.pending() && matches!(state.stage(),Stage::Photos(_)) {
        if state.checking_photo() {
            (b"Photo captured. Checking face, lighting and pose. An accepted view advances automatically; otherwise, capture this view again." as &[u8],b"Checking photo..." as &[u8])
        } else {
            (b"Waiting for a fresh camera frame. Face the camera and hold still." as &[u8],b"Capturing..." as &[u8])
        }
    } else {(guidance,action)};

    (guidance,action)
}
// ------------------------=
// FUNC: face_notice
// DESC: Shares actionable capture and persistence errors without hiding failed enrollment.
// ------------------=
fn face_notice(state:&crate::runtime::face_setup::Setup,error:bool,camera_available:bool)->&'static [u8] {
    let note:&[u8]=if error {b"Unable to start. A supported camera, valid account password and a face name are required."}
        else if let Some(code)=state.rejection().or_else(||if let Stage::Failed(code)=state.stage(){Some(code)}else{None}) {match code {
            104=>b"Camera disconnected or could not deliver a complete frame. Check the VM camera attachment.",
            105 if matches!(state.stage(),Stage::Photos(_))=>b"No fresh camera frame arrived in time. Accepted views are kept. Check the camera attachment, then capture this view again.",
            105=>b"No fresh camera frame arrived in time. Check the camera attachment and retry.",
            111|220=>b"The enrollment session expired. Retry when you are ready.",
            114=>b"Recognition models took too long to prepare. No identity was saved. Retry camera setup.",
            115=>b"Face analysis took too long. No identity was saved. Retry camera setup.",
            116=>b"The identity store is full. Free storage before retrying facial enrollment.",
            117=>b"Storage could not confirm the save. Restart before retrying; no enrollment success is claimed.",
            112=>b"The encrypted identity could not be saved and read back. Check storage and retry. No successful enrollment is being claimed.",
            204|205=>b"That frame was too old or captured too soon. Earlier accepted views are kept. Hold still and capture this view again.",
            206=>b"No face detected. Center your face and improve the lighting.",
            207=>b"More than one face detected. Only one person may be in the frame.",
            208=>b"Move closer: the face is too small for a reliable comparison.",
            210|213=>b"The image is too dark or low contrast. Face a soft light source.",
            211|212=>b"Facial detail is lost in bright highlights or deep shadows. Use soft, even lighting.",
            214=>b"The image is blurred. Hold still and check camera focus.",
            222=>b"Could not align the face. Center your whole face, turn only slightly and keep both eyes visible. Accepted views are kept.",
            216 if state.stage()==Stage::Photos(1)=>b"Left view not accepted. Turn slightly to YOUR left: your nose should move right in the preview. Keep both eyes visible, then click Capture left view.",
            216 if state.stage()==Stage::Photos(2)=>b"Right view not accepted. Turn slightly to YOUR right: your nose should move left in the preview. Keep both eyes visible, then click Capture right view.",
            216=>b"The face angle does not match the requested view. Follow the capture guidance.",
            217|218=>b"The faces did not match consistently. Retry with the same person throughout.",
            _=>b"The recognition worker could not complete. No enrollment or login change was saved.",
        }}
        else if !camera_available {b"Native camera capture is unavailable on this configuration. You can skip this optional check."}
        else {b"Private, on-device enrollment. Password sign-in stays available. RGB matching cannot prevent photo or video replay."};

    note
}

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
            let w=(r.width as usize).min(r.height as usize*PREVIEW_WIDTH/PREVIEW_HEIGHT);
            let h=w*PREVIEW_HEIGHT/PREVIEW_WIDTH;
            let left=r.x+(r.width as usize-w) as i32/2;
            let top=r.y+(r.height as usize-h) as i32/2;
            for y in 0..h {for x in 0..w {
                let i=((y*PREVIEW_HEIGHT/h)*PREVIEW_WIDTH+x*PREVIEW_WIDTH/w)*3;
                console.display.pixel(left+x as i32,top+y as i32,pixels[i],pixels[i+1],pixels[i+2]);
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
        if self.face_template_frame(crate::ui::installer_layout::CONFIGURATION_TEMPLATE_BYTES,
            input,focus,error,state,camera_available) {return;}
        let l=Layout::new(self.width,self.height);
        let r=l.card;self.access_panel(r.x as usize,r.y as usize,r.width as usize,r.height as usize);
        let bounds=|r:crate::ui::geometry::Rect|(r.x as usize,r.y as usize,r.width as usize,r.height as usize);
        self.access_label(bounds(l.title),b"Create your facial identity",l.pixels*3/2,palette::TEXT,false,true);
            let (guidance,action)=face_guidance(state);
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
            self.face_mask_base(l.mask,false);
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
                            crate::runtime::face_setup::PhotoStatus::Pending=>if state.rejection().is_some()
                                && state.stage()==Stage::Photos(index as u8) {b"Retry required"}else{b"Pending"},
                        },
                        l.pixels*3/5,palette::SECONDARY,true,false);
                }
            }
            let note=face_notice(state,error,camera_available);
            let note_color=if error || state.rejection().is_some() || matches!(state.stage(),Stage::Failed(_)) {
                palette::ERROR
            } else {palette::SECONDARY};
            self.access_text_wrapped(l.status.x as usize,l.status.y as usize,l.status.width as usize,note,
                note_color.0,note_color.1,note_color.2,255,l.pixels*4/5,false);
            for (r,label,id,primary) in [(l.back,b"Back" as &[u8],0,false),(l.skip,b"Skip" as &[u8],3,false),(l.primary,action,1,true)] {
                self.access_button(r.x as usize,r.y as usize,r.width as usize,r.height as usize,label,primary,focus==id);
            }
    }

    // ------------------------=
    // FUNC: face_template_frame
    // DESC: Composes the exact Studio face scene with live camera, enrollment state and native animated mesh bindings.
    // ------------------=
    pub(super) fn face_template_frame(&mut self,bytes:&[u8],input:&[u8],focus:usize,error:bool,
        state:&crate::runtime::face_setup::Setup,camera_available:bool)->bool {
        use crate::ui::installer_template::InstallerTemplate;
        use crate::ui::installer_layout::scale_template_rect;
        let Ok(template)=InstallerTemplate::parse(bytes) else{return false;};
        let Some(count)=template.element_count(9) else{return false;};
        let (guidance,action)=face_guidance(state);
        let notice=face_notice(state,error,camera_available);
        for index in 0..count {
            let Some(mut e)=template.layer_at(9,index) else{continue;};
            if e.hidden || e.opacity==0 {continue;}
            let r=scale_template_rect(e.frame,self.width,self.height);
            let id=e.code_identifier;
            if matches!(id,b"facePoseGuide0"|b"facePoseGuide1"|b"facePoseGuide2")
                && state.photo((id[id.len()-1]-b'0') as usize).is_some() {continue;}
            if id==b"faceGuidance" && state.saved() {
                self.biometric_auto_toggle(crate::ui::geometry::Rect{x:r.left as i32,y:r.top as i32,width:r.width as u32,height:r.height as u32},focus==4);
                continue;
            }
            match id {
                b"faceGuidance"=>e.text=match state.stage() {
                    Stage::Photos(0) if !state.pending()=>b"Keep your whole face visible in even lighting.",
                    Stage::Photos(1) if !state.pending()=>b"Turn slightly to your left. Keep both eyes visible and hold still.",
                    Stage::Photos(_) if !state.pending()=>b"Turn slightly to your right. Keep both eyes visible and hold still.",
                    Stage::Photos(_) if state.checking_photo()=>b"Checking lighting, detail and pose. Accepted views are kept.",
                    Stage::Photos(_)=>b"Waiting for a fresh frame. Hold still.",
                    Stage::Review=>b"Three views accepted. Verify and save your identity to this account.",
                    Stage::Verifying(_)=>b"Face the camera. Comparing fresh captures with your three views.",
                    Stage::Passed=>b"Your identity is saved. You can now continue setup.",
                    Stage::Failed(_)=>b"Review the message below, then retry. Password sign-in stays available.",
                    _=>b"Preparing the camera. Keep one person in the frame.",
                },
                b"faceStepTitle"=>e.text=match state.stage() {
                    Stage::Photos(0)=>b"Face the camera",
                    Stage::Photos(1)=>b"Turn slightly left",
                    Stage::Photos(_)=>b"Turn slightly right",
                    Stage::Review=>b"Review your photos",
                    Stage::Verifying(_)=>b"Verifying identity",
                    Stage::Passed=>b"Identity saved",
                    Stage::Failed(_)=>b"Please try again",
                    _=>b"Preparing camera",
                },
                b"facePrimary"=>e.text=action,
                b"faceStatus"=>{
                    e.text=notice;
                    if error || state.rejection().is_some() || matches!(state.stage(),Stage::Failed(_)) {
                        e.fill=[palette::ERROR.0,palette::ERROR.1,palette::ERROR.2,255];
                    }
                },
                b"faceName"=>{
                    if state.stage()==Stage::Review {
                        e.kind=5;e.text=b"Retake photos";
                    } else {
                        self.onboarding_input_field_with_element(Some(e),r.left,r.top,r.width,r.height,
                            input,false,focus==2,b"Face name (for example: Everyday)");
                        continue;
                    }
                },
                b"facePhotoStatus0"|b"facePhotoStatus1"|b"facePhotoStatus2"=>{
                    e.text=match state.photo_status((id[id.len()-1]-b'0') as usize) {
                        crate::runtime::face_setup::PhotoStatus::Checking=>b"Checking...",
                        crate::runtime::face_setup::PhotoStatus::Accepted=>b"Accepted",
                        crate::runtime::face_setup::PhotoStatus::Pending=>b"Pending",
                    };
                },
                b"facePhotoCard0"|b"facePhotoCard1"|b"facePhotoCard2"=>{
                    let slot=(id[id.len()-1]-b'0') as usize;
                    if state.stage()==Stage::Photos(slot as u8) {
                        e.border=[40,225,255,255];
                    } else if state.photo_status(slot)==crate::runtime::face_setup::PhotoStatus::Accepted {
                        e.border=[90,205,180,255];
                    }
                },
                b"faceMesh"=>{
                    self.face_mask_base(crate::ui::geometry::Rect{x:r.left as i32,y:r.top as i32,
                        width:r.width as u32,height:r.height as u32},
                        crate::ui::face_setup_layout::binding(&template,b"faceSprite",self.width,self.height).is_some());
                    continue;
                },
                _=>{},
            }
            if matches!(id,b"facePhotoStatus0"|b"facePhotoStatus1"|b"facePhotoStatus2"|b"faceStepTitle") {
                self.access_label((r.left,r.top,r.width,r.height),e.text,
                    (e.font_size as usize*self.height/1000).max(8),palette::TEXT,
                    id!=b"faceStepTitle",id==b"faceStepTitle");
                continue;
            }
            if matches!(e.kind,1|4|5) {
                let target=match id {b"faceBack"=>Some(0),b"facePrimary"=>Some(1),
                    b"faceName"=>Some(2),b"faceSkip"=>Some(3),_=>None};
                self.access_template_control_with_focus(e,None,(0,0),Some(target==Some(focus)));
            } else {
                self.template_element_in_rect(e,template.asset(e.image_asset),None,r);
            }
            let image=match id {
                b"facePreview"=>state.preview().map(|p|(p,PREVIEW_WIDTH,PREVIEW_HEIGHT)),
                b"facePhoto0"|b"facePhoto1"|b"facePhoto2"=>state.photo((id[id.len()-1]-b'0') as usize)
                    .map(|p|(p,crate::runtime::face_setup::PHOTO_WIDTH,crate::runtime::face_setup::PHOTO_HEIGHT)),
                _=>None,
            };
            if let Some((pixels,width,height))=image {
                // Aspect-fit real camera evidence; resizing a skin must not distort a face.
                let w=r.width.min(r.height*width/height).max(1);
                let h=(w*height/width).max(1);
                let left=r.left+(r.width.saturating_sub(w))/2;
                let top=r.top+(r.height.saturating_sub(h))/2;
                for y in 0..h {for x in 0..w {
                    let i=((y*height/h)*width+x*width/w)*3;
                    self.blend_color((left+x) as i32,(top+y) as i32,pixels[i],pixels[i+1],pixels[i+2],
                        (255u16*u16::from(e.opacity)/100) as u8);
                }}
            } else if id==b"facePreview" {
                self.face_alignment_guide(r.left,r.top,r.width,r.height);
            }
        }
        true
    }
    // ------------------------=
    // FUNC: face_alignment_guide
    // DESC: Draws an antialiased empty-camera alignment oval without presenting a simulated enrollment photograph.
    // ------------------=
    fn face_alignment_guide(&mut self,x:usize,y:usize,w:usize,h:usize) {
        let rx=(w as f32*0.38).min(h as f32*0.40);let ry=h as f32*0.47;
        if rx<1.0 || ry<1.0 {return;}
        for py in 0..h {for px in 0..w {
            let dx=(px as f32-w as f32*0.5)/rx;
            let dy=(py as f32-h as f32*0.5)/ry;
            let distance=(dx*dx+dy*dy-1.0).abs()*rx*0.5;
            let alpha=((1.6-distance).max(0.0).min(1.0)*180.0) as u8;
            if alpha>0 {self.blend_color((x+px) as i32,(y+py) as i32,170,215,245,alpha);}
        }}
    }
}
