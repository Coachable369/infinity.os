//! Shared preview/photo acquisition: preserve exposure while requiring fresh pixels.
use super::{call,clock,visit};
#[derive(Default)]
pub struct Stream { opened:Option<u64>, first_frame:Option<u64>, sequence:u64 }

// ------------------------=
// FUNC: capture
// DESC: Drains startup frames for exposure settling and captures a fresh pinned frame without restarting a healthy preview stream.
// ------------------=
pub fn capture(command:u64,encoded:&mut [u8],stream:&mut Stream,live:bool)->Result<(usize,u64,u64),u32> {
    let begin=clock()?;
    // Renew before the existing bounded camera lease expires, never per click.
    if stream.opened.is_some_and(|opened|begin.saturating_sub(opened)>=8000) {
        call(0x1262,[0;6]);*stream=Stream::default();
    }
    let result=(|| {
        while stream.opened.is_none() {
            let status=call(0x1260,[0;6]);
            if status==0 {stream.opened=Some(clock()?);break;}
            if status!= -11 && status!= -16 {return Err(104);}
            if clock()?.saturating_sub(begin)>5000 || !visit(command) {return Err(105);}
        }
        // The first frame available on a photo request may predate the click.
        let mut after_click=live;
        loop {
            if clock()?.saturating_sub(begin)>10_000 || !visit(command) {return Err(105);}
            let mut meta=[0u64;2];
            let status=call(0x1261,[0,0,0,0,meta.as_mut_ptr() as u64,0]);
            if status== -11 {continue;}
            if status== -110 {return Err(105);}
            if status<0 || meta[0]==0 || meta[1]<4 || meta[1]>encoded.len() as u64 {return Err(104);}
            let length=meta[1] as usize;let mut stale=false;
            // Even a discarded frame must be fully drained to release its pin.
            for offset in (0..length).step_by(4096) {
                let n=(length-offset).min(4096);
                let status=call(0x1261,[meta[0],offset as u64,unsafe {encoded.as_mut_ptr().add(offset)} as u64,n as u64,0,0]);
                if status== -116 {stale=true;break;}
                if status== -110 {return Err(105);}
                if status!=n as i64 {return Err(104);}
                if !visit(command) {return Err(105);}
            }
            if stale {continue;}
            let now=clock()?;
            let first=*stream.first_frame.get_or_insert(now);
            let fresh=meta[0]!=stream.sequence;
            stream.sequence=meta[0];
            let settled=now.saturating_sub(first)>=2000;
            if fresh && after_click && settled {return Ok((length,meta[0],now));}
            after_click=true;
        }
    })();
    // A Capture command superseding preview must not power-cycle the camera.
    if result.is_err() && visit(command) {call(0x1262,[0;6]);*stream=Stream::default();}
    result
}
