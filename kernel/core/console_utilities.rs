//! Native filesystem/network front ends and bounded local output paging.
use super::*;
use core::fmt::Write;
use crate::runtime::console_execution::{Bytes,OUTPUT_BYTES};
struct Pager { data:[u8;OUTPUT_BYTES], len:usize, at:usize, owner:[u8;16] }
static mut PAGE:Pager=Pager{data:[0;OUTPUT_BYTES],len:0,at:0,owner:[0;16]};
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: page_output
    // DESC: Retains one bounded result privately and displays its first page without losing earlier rows.
    // ------------------=
    pub(super) fn page_output(&mut self,bytes:&[u8]) {
        if self.command_terminal_visible(){crate::ui::command_terminal::mutate(|s|s.append(bytes));return;}
        unsafe {let p=&mut *core::ptr::addr_of_mut!(PAGE);p.data.fill(0);p.len=bytes.len().min(OUTPUT_BYTES);p.data[..p.len].copy_from_slice(&bytes[..p.len]);p.at=0;p.owner=self.current_session.0;}
        self.next_output_page();
    }
    // ------------------------=
    // FUNC: page_result
    // DESC: Marks a bounded response that exceeded its output budget instead of silently claiming complete results.
    // ------------------=
    fn page_result(&mut self,out:&Bytes<OUTPUT_BYTES>) {
        self.page_output(out.bytes());
        if out.truncated(){unsafe{let p=&mut *core::ptr::addr_of_mut!(PAGE);let notice=b"\nOutput limit reached. Narrow the path, help topic, or search.\n";let start=p.len.saturating_sub(notice.len());p.data[start..p.len].copy_from_slice(notice);}}
    }
    // ------------------------=
    // FUNC: next_output_page
    // DESC: Wraps long lines to the actual legacy Console row capacity and exposes explicit continuation.
    // ------------------=
    fn next_output_page(&mut self) {unsafe {
        let p=&mut *core::ptr::addr_of_mut!(PAGE);self.output.clear();
        if p.owner!=self.current_session.0 {p.data.fill(0);p.len=0;p.at=0;return;}
        for _ in 0..5 {
            if p.at==p.len{break;}
            let start=p.at;
            while p.at<p.len&&p.data[p.at]!=b'\n'&&p.at-start<LINE_CAPACITY{p.at+=1;}
            self.output.write_line(&p.data[start..p.at]);
            if p.at<p.len&&p.data[p.at]==b'\n'{p.at+=1;}
        }
        if p.at<p.len {self.output.write_line(b"-- Enter / Space: next page    Esc: return --");}
    }}
    // ------------------------=
    // FUNC: pager_key
    // DESC: Consumes only paging controls; typing a new command ends paging without dropping the typed key.
    // ------------------=
    pub(super) fn pager_key(&mut self,key:ConsoleKey)->bool {unsafe {
        let p=&mut *core::ptr::addr_of_mut!(PAGE);
        if p.owner!=self.current_session.0 {p.data.fill(0);p.len=0;p.at=0;}
        if p.at>=p.len{return false;}
        if matches!(key,ConsoleKey::Enter|ConsoleKey::Character(b' ')){self.next_output_page();return true;}
        p.at=p.len;matches!(key,ConsoleKey::Escape)
    }}
    // ------------------------=
    // FUNC: utility_command
    // DESC: Routes documented local utilities through existing identity, network and namespace services.
    // ------------------=
    pub(super) fn utility_command(&mut self,command:&[u8])->bool {
        let first=command_word(command,0).unwrap_or(b"");
        if first==b"commands" {
            let mut out=Bytes::<OUTPUT_BYTES>::new();crate::runtime::console_help::render(b"help commands",&mut out);self.page_result(&out);return true;
        }
        if crate::runtime::console_help::requested(command) {
            let mut out=Bytes::<OUTPUT_BYTES>::new();crate::runtime::console_help::render(command,&mut out);self.page_result(&out);return true;
        }
        if !matches!(first,b"ipconfig"|b"list"|b"ls"|b"dir"|b"edit"){return false;}
        if !self.workplace_authorized(false){self.output.write_line(b"An active local session is required.");return true;}
        if first==b"edit" {self.start_terminal_edit(command_tail(command,1).unwrap_or(b""));return true;}
        let mut out=Bytes::<OUTPUT_BYTES>::new();
        if first==b"ipconfig" {
            if command_word(command,1).is_some_and(|a|a!=b"--all")||command_word(command,2).is_some(){out.write_line(b"Usage: ipconfig [--all]");}
            else {crate::runtime::with_runtime(|runtime|{
                let net=&runtime.network;
                for i in 0..net.interfaces.interface_count() {
                    let Some(iface)=net.interfaces.interface_nth(i)else{continue;};
                    let _=writeln!(out,"Interface {}  {:?}  {:?}  enabled={}",iface.id,iface.device.link_type,iface.device.link_state,iface.enabled);
                    if let Some(mac)=iface.device.hardware_address {let _=writeln!(out,"  MAC {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",mac[0],mac[1],mac[2],mac[3],mac[4],mac[5]);}
                    let _=writeln!(out,"  MTU {}  state {:?}",iface.device.maximum_frame_size,iface.device.operational_state);
                    for a in (0..net.interfaces.address_count()).filter_map(|i|net.interfaces.address_nth(i)).filter(|a|a.interface_id==iface.id) {
                        out.write(b"  Address ");ip(&mut out,a.address);let _=writeln!(out,"/{} {:?} {:?}",a.prefix_length,a.source,a.state);
                        if let crate::runtime::network::types::IpAddress::V4(_)=a.address {out.write(b"  Netmask ");let mask=u32::MAX.checked_shl(32u32.saturating_sub(a.prefix_length as u32)).unwrap_or(0).to_be_bytes();ip(&mut out,crate::runtime::network::types::IpAddress::V4(mask));out.write(b"\n");}
                    }
                    for r in (0..net.interfaces.route_count()).filter_map(|i|net.interfaces.route_nth(i)).filter(|r|r.interface_id==iface.id) {
                        out.write(b"  Route ");ip(&mut out,r.destination);let _=write!(out,"/{} via ",r.prefix_length);
                        if let Some(gateway)=r.next_hop{ip(&mut out,gateway);}else{out.write(b"direct");}
                        let _=writeln!(out," metric {}",r.metric);
                    }
                    let _=writeln!(out,"  RX {} TX {} drops RX {} TX {}",iface.rx_packets,iface.tx_packets,iface.rx_drops,iface.tx_drops);
                }
                out.write_line(b"DNS servers:");for i in 0..8{if let Some(address)=net.resolver.server(i){out.write(b"  ");ip(&mut out,address);out.write(b"\n");}}
            });}
        } else {self.list_command(command,&mut out);}
        self.page_result(&out);true
    }
    // ------------------------=
    // FUNC: utility_path
    // DESC: Resolves quoted and relative paths through the native namespace normalizer.
    // ------------------=
    pub(super) fn utility_path(&self,path:&[u8])->Option<crate::runtime::object_navigation::ByteText<96>> {
        use crate::runtime::object_navigation::{join_path,normalize_absolute_path};
        let path=path.trim_ascii();let path=if path.len()>1&&path[0]==b'"'&&path[path.len()-1]==b'"'{&path[1..path.len()-1]}else{path};
        if path.first()==Some(&b'/'){normalize_absolute_path(path).ok()}else{join_path(self.navigation_context.path(),path).ok()}
    }
    // ------------------------=
    // FUNC: list_command
    // DESC: Implements list/ls/dir with one parser and truthful typed object metadata.
    // ------------------=
    fn list_command(&self,command:&[u8],out:&mut Bytes<OUTPUT_BYTES>) {
        let mut long=false;let mut all=false;let mut tree=false;let mut path=None;
        let mut tail=command_tail(command,1).unwrap_or(b"").trim_ascii();
        while !tail.is_empty(){
            let n=tail.iter().position(u8::is_ascii_whitespace).unwrap_or(tail.len());let word=&tail[..n];
            if word==b"--"{path=Some(tail[n..].trim_ascii());break;}
            if word==b"--long"{long=true;}else if word==b"--all"{all=true;}else if word==b"--tree"||word==b"tree"{tree=true;}
            else if word.starts_with(b"-") {
                for b in &word[1..]{match b{b'l'=>long=true,b'a'=>all=true,b'R'=>tree=true,_=>{out.write_line(b"Unknown option. Use help list.");return;}}}
            }else{path=Some(tail);break;}tail=tail[n..].trim_ascii();
        }
        let resolved=path.and_then(|p|self.utility_path(p));
        if path.is_some()&&resolved.is_none(){out.write_line(b"Invalid path.");return;}
        let prefix=resolved.as_ref().map(|p|p.as_bytes()).unwrap_or(self.navigation_context.path());
        let metadata=crate::storage::object_inspect_path(prefix);
        if metadata.is_err(){out.write_line(b"Path not found or inaccessible.");return;}
        let file=metadata.is_ok_and(|(m,_)|m.kind!=crate::storage::object::ObjectType::NamespaceNode);
        let mut shown=0;
        for index in 0..crate::storage::object::PUBLIC_NAMESPACE_CAPACITY {
            let entry=match crate::storage::namespace_list_nth(prefix,index){Ok(Some(e))=>e,_=>break};
            let path=&entry.path[..entry.path_len as usize];
            if file {if path!=prefix{continue;}}else {
                if path==prefix||(!tree&&!immediate_namespace_child(prefix,path)){continue;}
                if prefix!=b"/"&&path.get(prefix.len())!=Some(&b'/'){continue;}
            }
            if !file&&!all&&path[prefix.len().min(path.len())..].split(|b|*b==b'/').any(|p|p.starts_with(b".")){continue;}
            out.write_line(path);shown+=1;
            if long {match crate::storage::object_inspect_path(path){Ok((m,_))=>{
                let _=writeln!(out,"  {:?}  {} bytes  version {}",m.kind,m.logical_size,m.current_version);
                out.write(b"  Owner: ");let name=crate::runtime::with_runtime(|r|r.identity.user(crate::runtime::identity::StableId(m.owner.0))).flatten();
                if let Some(user)=name {out.write(user.handle.as_bytes());out.write(b"\n");}else{out.write_hex(b"",&m.owner.0);}
                timestamp(out,b"  Created: ",m.created);timestamp(out,b"  Modified: ",m.modified);
            },Err(_)=>out.write_line(b"  Metadata unavailable (protected object).")}}
        }
        let _=writeln!(out,"{} entries",shown);
    }
}
// ------------------------=
// FUNC: ip
// DESC: Formats native IPv4/IPv6 values without DNS or host networking.
// ------------------=
fn ip(out:&mut Bytes<OUTPUT_BYTES>,address:crate::runtime::network::types::IpAddress){
    use crate::runtime::network::types::IpAddress;
    match address {IpAddress::V4(a)=>{let _=write!(out,"{}.{}.{}.{}",a[0],a[1],a[2],a[3]);},IpAddress::V6(a)=>{for (i,p) in a.chunks_exact(2).enumerate(){if i>0{out.write(b":");}let _=write!(out,"{:x}",u16::from_be_bytes([p[0],p[1]]));}}}
}
// ------------------------=
// FUNC: timestamp
// DESC: Displays real UTC dates only for tagged wall-clock records; legacy generations are explicitly identified.
// ------------------=
fn timestamp(out:&mut Bytes<OUTPUT_BYTES>,label:&[u8],value:u64){
    out.write(label);
    if value>>63==1 {if let Some(t)=crate::runtime::ai::clock::local_time(value&!(1<<63),0){let _=writeln!(out,"{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",t.year,t.month,t.day,t.hour,t.minute,t.second);return;}}
    let _=writeln!(out,"not recorded (legacy generation {})",value);
}
