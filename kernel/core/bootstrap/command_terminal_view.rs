//! Shared Command Window renderer; outer chrome remains compositor-owned.
use super::DisplayDevice;
use crate::ui::{command_terminal::{self as terminal,Menu,Layout},command_terminal_paint::{self as paint,Canvas},geometry::Rect,editor_tools::preferences::{self,Preferences}};
struct Surface<'a>{display:&'a mut DisplayDevice,body:Rect,opacity:u8}
impl Canvas for Surface<'_> {
    // ------------------------=
    // FUNC: fill
    // DESC: Blends only the terminal background; text and interaction feedback retain full contrast.
    // ------------------=
    fn fill(&mut self,r:Rect,c:(u8,u8,u8)) {
        if r==self.body&&self.display.recording_surface {
            if let Some(region)=self.display.clipped_render_region(r.x.max(0)as usize,r.y.max(0)as usize,r.width as usize,r.height as usize){
                for y in region.top..region.bottom{unsafe{core::ptr::write_bytes(self.display.buffer.add(y*self.display.stride+region.left),0,region.right-region.left);}}
            }
        }
        self.display.fill_rect_alpha(r.x.max(0)as usize,r.y.max(0)as usize,r.width as usize,r.height as usize,c.0,c.1,c.2,if r==self.body{self.opacity}else{255});}
    // ------------------------=
    // FUNC: rail
    // DESC: Uses the same rounded rail treatment as Settings.
    // ------------------=
    fn rail(&mut self,r:Rect,c:(u8,u8,u8)){self.display.fill_rounded_rect_alpha(r.x.max(0) as usize,r.y.max(0) as usize,r.width as usize,r.height as usize,r.width as usize/2,c.0,c.1,c.2,235);}
    // ------------------------=
    // FUNC: glyph
    // DESC: Draws packaged monospace raster pixels at their measured grid advance.
    // ------------------=
    fn glyph(&mut self,x:i32,y:i32,b:u8,scale:usize,p:Preferences,c:(u8,u8,u8)) {paint::raster_glyph(p,b,scale,|dx,dy,a|self.display.blend_color(x+dx as i32,y+dy as i32,c.0,c.1,c.2,a));}
}
impl DisplayDevice {
    // ------------------------=
    // FUNC: command_terminal_view
    // DESC: Renders the continuous terminal, shared preference surface, and navigable ASCII help catalog.
    // ------------------=
    pub(super) fn command_terminal_view(&mut self,body:Rect,toolbar:Rect,scale:usize) {
        let g=terminal::inspect(|s|s.layout(body,toolbar,scale));terminal::mutate(|s|s.reflow(g.columns(),g.rows()));terminal::publish_snapshot();
        terminal::inspect(|s|{
            let mut canvas=Surface{display:self,body:g.body,opacity:s.opacity};if !s.remote && !s.monitor && !s.disk.active {paint::content(&mut canvas,s,g,true);}
            if s.remote {canvas.fill(g.body,s.background_color());crate::ui::terminal_vt::inspect(|t|{
                for y in 0..t.rows.min(g.rows()){for x in 0..t.columns.min(g.columns()){
                    let c=t.cells[y][x];let mut fg=c.fg;let mut bg=c.bg;if c.inverse{core::mem::swap(&mut fg,&mut bg);}if c.bold&&fg<8{fg+=8;}
                    let r=Rect{x:g.text.x+(x*g.cell)as i32,y:g.text.y+(y*g.line)as i32,width:g.cell as u32,height:g.line as u32};
                    if bg!=0 {canvas.fill(r,ansi_color(bg));}canvas.glyph(r.x,r.y+2*scale as i32,c.byte,scale,s.appearance,if fg==7{s.appearance.color(4)}else{ansi_color(fg)});
                }}
                if t.cursor_visible {canvas.fill(Rect{x:g.text.x+(t.x*g.cell)as i32,y:g.text.y+(t.y*g.line)as i32,width:2*scale as u32,height:g.line as u32},s.appearance.color(5));}
            });}
            if s.disk.active { disk_paint(&mut canvas,g,s); }
            if s.monitor {
                use core::fmt::Write;
                canvas.fill(g.body,s.background_color());
                colored(&mut canvas,g,s.appearance,1,b"  PID   CPU    MEM KiB  STATUS    PROCESS",ansi_color(14));
                let visible=g.rows().saturating_sub(4).max(1);
                crate::runtime::with_runtime(|r|{
                    let count=r.execution.count();let selected=s.monitor_row.min(count.saturating_sub(1));
                    let mut runnable=0;let mut waiting=0;
                    for i in 0..count {if let Some(t)=r.execution.nth(i) {
                        use crate::runtime::execution::ContextState::*;
                        match t.state {Running|Runnable=>runnable+=1,Waiting=>waiting+=1,_=>{}}
                    }}
                    let mut summary=crate::runtime::console_execution::Bytes::<120>::new();
                    let _=write!(summary,"TOP   Tasks: {}   Running/ready: {}   Waiting: {}",count,runnable,waiting);
                    colored(&mut canvas,g,s.appearance,0,summary.bytes(),s.appearance.color(5));
                    let start=selected/visible*visible;
                    for i in start..count.min(start+visible) {
                        let Some(t)=r.task_manager.task_nth(&r.execution,i) else{break};
                        let mut line=crate::runtime::console_execution::Bytes::<160>::new();
                        let _=write!(line,"{} {:4} {:3}% {:9}  {:8}  ",if i==selected{'>'}else{' '},t.handle.0,t.cpu_share_percent,t.usage.memory_bytes/1024,
                            core::str::from_utf8(crate::console::context_state_text(t.state)).unwrap_or("?"));
                        line.write(crate::console::task_name(r,&t));
                        let color=if i==selected{14}else if t.cpu_share_percent>=80{9}else if t.cpu_share_percent>=40{11}
                            else if t.cpu_share_percent!=0{10}else{7};
                        colored(&mut canvas,g,s.appearance,i-start+2,line.bytes(),ansi_color(color));
                    }
                    if g.rows()>=4 {if let Some(t)=r.task_manager.task_nth(&r.execution,selected) {
                        let mut line=crate::runtime::console_execution::Bytes::<96>::new();
                        let _=write!(line,"PID {}: CPU weight {}   Memory budget {} KiB",t.handle.0,t.budget.cpu_weight,t.budget.memory_limit/1024);
                        ascii(&mut canvas,g,s.appearance,g.rows()-2,line.bytes());
                    }}
                });
                ascii(&mut canvas,g,s.appearance,g.rows().saturating_sub(1),b"Arrows: select  P: pause  T: CPU  M: memory  K: end  R: relaunch  Q: quit");
            }
            if s.help {
                canvas.fill(g.body,s.background_color());
                let entries=crate::runtime::console_help::COMMANDS;
                let e=&entries[s.help_selected.min(entries.len()-1)];
                if s.help_detail {
                    ascii(&mut canvas,g,s.appearance,0,b"+-- COMMAND REFERENCE ---------------------------------------+");
                    ascii(&mut canvas,g,s.appearance,1,e.usage);
                    let mut row=3;for part in e.description.chunks(g.columns()){ascii(&mut canvas,g,s.appearance,row,part);row+=1;}
                    ascii(&mut canvas,g,s.appearance,g.rows().saturating_sub(1),b"Esc: command list");
                }else {
                    ascii(&mut canvas,g,s.appearance,0,b"+-- COMMAND ------------------+-- USAGE ----------------------+");
                    let visible=g.rows().saturating_sub(3).max(1);let start=s.help_selected/visible*visible;
                    for (i,e) in entries.iter().skip(start).take(visible).enumerate(){
                        let mut line=[b' ';320];line[0]=if start+i==s.help_selected{b'>'}else{b'|'};
                        let n=e.name.len().min(26);line[2..2+n].copy_from_slice(&e.name[..n]);line[29]=b'|';
                        let n=e.usage.len().min(288);line[31..31+n].copy_from_slice(&e.usage[..n]);
                        ascii(&mut canvas,g,s.appearance,i+1,&line[..(31+n).min(g.columns())]);
                    }
                    ascii(&mut canvas,g,s.appearance,g.rows().saturating_sub(1),b"Up/Down: select    Enter: help    Esc: prompt");
                }
            }
        });
        terminal::inspect(|s|{
            let mut status=[0u8;128];let mut text=crate::runtime::console_execution::Bytes::<128>::new();
            use core::fmt::Write;let _=write!(text,"{} columns x {} rows  |  {}",s.columns,s.rows,if s.remote{"SSH  Ctrl+] disconnect"}else{"Ctrl+, preferences"});
            if !s.notice.is_empty(){text.write(b" | ");text.write(s.notice);}
            let n=text.bytes().len();status[..n].copy_from_slice(text.bytes());
            let (r,gc,b)=s.background_color();
            self.fill_rect(g.status.x.max(0) as usize,g.status.y.max(0) as usize,g.status.width as usize,g.status.height as usize,r.saturating_add(20),gc.saturating_add(20),b.saturating_add(20));
            self.app_label(g.status,&status[..n],s.appearance.color(8),false,scale);
            if s.menu!=Menu::None {
                let (rect,row)=g.menu_rect(s);self.app_dropdown_panel(rect,scale);
                for i in 0..terminal::PREFERENCE_ROWS {let r=Rect{x:rect.x+6*scale as i32,y:rect.y+(i*row)as i32,width:rect.width.saturating_sub(12*scale as u32),height:row as u32};
                    self.app_dropdown_row(r,i==s.menu_row,scale);
                    let mut b=crate::runtime::console_execution::Bytes::<128>::new();
                    match i {
                        0=>{b.write(b"Font: ");b.write(preferences::FONTS[s.appearance.font as usize]);},
                        1=>{b.write(b"Size: ");b.write(preferences::SIZE_NAMES[s.appearance.size as usize]);},
                        2=>{b.write(b"Scheme: ");b.write(preferences::NAMES[s.appearance.scheme as usize]);},
                        3=>{if s.setting_edit&&s.menu_row==3{let _=write!(b,"Rows: [{}] Enter",s.setting_value);}else if s.row_override==0{b.write(b"Rows: Auto");}else{let _=write!(b,"Rows: {}",s.row_override);}},
                        4=>{if s.setting_edit&&s.menu_row==4{let _=write!(b,"Columns: [{}] Enter",s.setting_value);}else if s.column_override==0{b.write(b"Columns: Auto");}else{let _=write!(b,"Columns: {}",s.column_override);}},
                        5=>{let _=write!(b,"Opacity: {}%",(s.opacity as usize*100+127)/255);},
                        6=>{b.write(b"Background: ");b.write(terminal::BACKGROUNDS[s.background as usize].0);},
                        _=>b.write(b"Restore defaults"),
                    }self.app_menu_label(Rect{x:r.x+6*scale as i32,width:r.width.saturating_sub(12*scale as u32),..r},b.bytes(),(235,242,250),scale);
                }
            }
        });
    }
}
// ------------------------=
// FUNC: ascii
// DESC: Paints a clipped ASCII menu row on the actual terminal grid.
// ------------------=
fn ascii(canvas:&mut impl Canvas,g:Layout,p:Preferences,row:usize,bytes:&[u8]) {
    colored(canvas,g,p,row,bytes,p.color(4));
}

// ------------------------=
// FUNC: colored
// DESC: Draws one bounded monitor row with semantic load coloring on the terminal's current font grid.
// ------------------=
fn colored(canvas:&mut impl Canvas,g:Layout,p:Preferences,row:usize,bytes:&[u8],color:(u8,u8,u8)) {
    if row>=g.rows(){return;}for(i,&b)in bytes.iter().take(g.columns()).enumerate(){canvas.glyph(g.text.x+(i*g.cell)as i32,g.text.y+(row*g.line)as i32,b,g.scale,p,color);}
}

// ------------------------=
// FUNC: ansi_color
// DESC: Resolves the standard sixteen ANSI colors independently of terminal background opacity.
// ------------------=
fn ansi_color(index:u8)->(u8,u8,u8){const C:[u32;16]=[0x000000,0xcd3131,0x0dbc79,0xe5e510,0x2472c8,0xbc3fbc,0x11a8cd,0xe5e5e5,0x666666,0xf14c4c,0x23d18b,0xf5f543,0x3b8eea,0xd670d6,0x29b8db,0xffffff];let c=C[index.min(15)as usize];((c>>16)as u8,(c>>8)as u8,c as u8)}

// ------------------------=
// FUNC: disk_paint
// DESC: Paints measured partition and file usage, selection, and cleanup controls on the current terminal grid.
// ------------------=
fn disk_paint(canvas:&mut impl Canvas,g:Layout,s:&terminal::State) {
use core::fmt::Write;
                let v=&s.disk;
                canvas.fill(g.body,s.background_color());
                let mut header=crate::runtime::console_execution::Bytes::<160>::new();
                if v.depth==0 {let _=write!(header,"DISK   |   System disk   {} MiB   |   GPT partitions",v.capacity/1048576);}
                else {header.write(b"DISK / ");header.write(v.partition.label());let _=write!(header,"   {} MiB   |   LBA {}",v.partition.bytes/1048576,v.partition.first);}
                colored(canvas,g,s.appearance,0,header.bytes(),ansi_color(14));
                ascii(canvas,g,s.appearance,1,if v.depth==0 {b"   PARTITION SHARE            SIZE       NAME"}else if v.partition.container{b"   RELATIVE FILE SIZE         SIZE       PERSONAL FILE (largest first)"}else{b"   RELATIVE FILE SIZE         SIZE       FILE (read-only)"});
                let chart=g.rows()>=9 && !v.inspect && v.depth==0 && v.capacity>0;
                if chart {
                    let columns=g.columns().saturating_sub(2).min(120);
                    let mut left=0; let mut bytes=0u128;
                    for (index,row) in v.rows[..v.count].iter().enumerate(){
                        bytes+=u128::from(row.bytes);
                        let right=((bytes*columns as u128)/u128::from(v.capacity)).min(columns as u128) as usize;
                        for col in left..right{canvas.glyph(g.text.x+((col+1)*g.cell)as i32,g.text.y+2*g.line as i32,
                            b'#',g.scale,s.appearance,ansi_color([14,12,13,10,11][index%5]));}
                        left=right;
                    }
                    for col in left..columns{canvas.glyph(g.text.x+((col+1)*g.cell)as i32,g.text.y+2*g.line as i32,
                        b'.',g.scale,s.appearance,ansi_color(8));}
                }
                let visible=g.rows().saturating_sub(5+usize::from(chart)).max(1);
                let base=v.selected/visible*visible;
                let max=if v.depth==0{v.capacity}else{v.rows.first().map_or(0,|r|r.bytes)}.max(1);
                if v.inspect && v.selected<v.count {
                    let row=v.rows[v.selected];colored(canvas,g,s.appearance,3,row.label(),ansi_color(14));
                    let mut info=crate::runtime::console_execution::Bytes::<160>::new();
                    let _=write!(info,"Logical size: {} bytes  |  {}",row.bytes,if v.partition.container{"Personal file"}else{"Read-only filesystem"});
                    ascii(canvas,g,s.appearance,4,info.bytes());
                } else {for i in base..v.count.min(base+visible) {
                    let row=v.rows[i];let mut line=crate::runtime::console_execution::Bytes::<160>::new();
                    line.write(if i==v.selected{b"> ["}else{b"  ["});
                    let bars=(u128::from(row.bytes)*16/u128::from(max)).min(16) as usize;
                    line.write(&[b'#';16][..bars]);line.write(&[b'.';16][..16-bars]);
                    let _=write!(line,"] {:>10} KiB  ",row.bytes.div_ceil(1024));line.write(row.label());
                    colored(canvas,g,s.appearance,i-base+2+usize::from(chart),line.bytes(),ansi_color(if i==v.selected{14}else{7}));
                }
                }
                if v.depth==1 && v.partition.container {
                    let mut usage=crate::runtime::console_execution::Bytes::<96>::new();
                    let _=write!(usage,"Pool allocation: {} MiB used",v.used/1048576);
                    ascii(canvas,g,s.appearance,g.rows().saturating_sub(3),usage.bytes());
                }
                colored(canvas,g,s.appearance,g.rows().saturating_sub(2),if v.confirm {
                    if v.rows[v.selected].label().starts_with(b"/trash/"){b"Permanently delete selected file? Y confirms."}else{b"Move selected file to Trash? Y confirms."}
                }else{v.message},ansi_color(11));
                ascii(canvas,g,s.appearance,g.rows().saturating_sub(1),b"Up/Down select   Enter inspect   Left back   D clean   U restore   R refresh   Q quit");
}
