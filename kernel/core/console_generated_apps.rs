//! Authenticated generated-application directory and launch adapter.
use super::*;
use crate::runtime::generated_apps::{Entry,Language,Error};
#[no_mangle]
pub static mut INFINITY_GENERATED_APPS_SNAPSHOT:[u64;16]=[0x494e4647454e4150,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
// ------------------------=
// FUNC: publish_catalog
// DESC: Exposes content-free catalog counts and verified launch receipts for native installed acceptance.
// ------------------=
fn publish_catalog(operation:u64,count:usize,entry:Option<Entry>,success:bool){unsafe{
    let p=core::ptr::addr_of_mut!(INFINITY_GENERATED_APPS_SNAPSHOT).cast::<u64>();let sequence=core::ptr::read_volatile(p.add(2)).wrapping_add(2)&!1;
    core::ptr::write_volatile(p.add(2),sequence|1);core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);
    core::ptr::write_volatile(p.add(5),operation);
    if operation<=1{core::ptr::write_volatile(p.add(3),count as u64);}
    if operation==0{core::ptr::write_volatile(p.add(4),count as u64);}
    if let Some(entry)=entry{core::ptr::write_volatile(p.add(6),u64::from_le_bytes(entry.source_id[..8].try_into().unwrap()));core::ptr::write_volatile(p.add(7),u64::from_le_bytes(entry.source_id[8..].try_into().unwrap()));core::ptr::write_volatile(p.add(8),entry.language as u64);core::ptr::write_volatile(p.add(9),entry.artifact_version as u64);}
    if operation==2{let at=if success{12}else{13};core::ptr::write_volatile(p.add(at),core::ptr::read_volatile(p.add(at)).wrapping_add(1));}
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);core::ptr::write_volatile(p.add(15),sequence);core::ptr::write_volatile(p.add(2),sequence);
}}
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: refresh_generated_launcher
    // DESC: Projects only the authenticated user's durable catalog into existing themed launcher cells.
    // ------------------=
    pub(super) fn refresh_generated_launcher(&self){
        use crate::ui::app_launcher::{VisibleEntry,LauncherAction,GENERATED_CAPACITY};
        let mut entries=[None;GENERATED_CAPACITY];
        if let Ok(registry)=crate::storage::generated_app_catalog(self.current_user.0,self.current_session.0,None){
            for (slot,entry) in entries.iter_mut().zip(registry.entries.iter().flatten()){
                *slot=Some(VisibleEntry{label:entry.name,label_length:entry.name_len,icon_role:49,action:LauncherAction::Generated(entry.source_id)});
            }
        }let count=entries.iter().flatten().count();crate::ui::app_launcher::publish_generated(entries);publish_catalog(0,count,None,true);
    }
    // ------------------------=
    // FUNC: launch_registered_identity
    // DESC: Resolves a clicked stable source identity through native storage before invoking the shared verified launcher.
    // ------------------=
    pub(super) fn launch_registered_identity(&mut self,id:[u8;16]){
        let entry=crate::storage::generated_app_catalog(self.current_user.0,self.current_session.0,None).ok()
            .and_then(|registry|registry.entries.into_iter().flatten().find(|entry|entry.source_id==id));
        self.open_command_window();
        if let Some(entry)=entry{let _=self.launch_registered_application(&entry.source[..entry.source_len]);}
        else{self.output.write_line(b"This application is no longer registered for the active user.");}
    }
    // ------------------------=
    // FUNC: generated_application_command
    // DESC: Lists owned generated applications or invokes the same version-checked launcher used by AI voice commands.
    // ------------------=
    pub(super) fn generated_application_command(&mut self,command:&[u8])->bool {
        if let Some(target)=command.strip_prefix(b"apps launch ") {let _=self.launch_registered_application(target.trim_ascii());return true;}
        if command!=b"apps"&&command!=b"apps list"&&command!=b"apps hooks"{return false;}
        if command==b"apps hooks" {
            let Some(context)=crate::runtime::ai::application_host::context().filter(|c|c.owner==self.current_user.0&&c.session==self.current_session.0)else{self.output.write_line(b"No live application has an owned interaction context.");return true;};
            let mut count=0;
            for index in 0..crate::runtime::ai::application::MAX_CHOICES {
                if let Ok(descriptor)=crate::runtime::ai::kit::CapabilityDirectory::application_descriptor(context,index){
                    let action=descriptor.action.action;let n=action.iter().position(|b|*b==0).unwrap_or(action.len());
                    self.output.write_segments(&[&action[..n],if descriptor.available{b" [available]"}else{b" [not currently available]"}]);count+=1;
                }
            }
            if count==0{self.output.write_line(b"This application has not published any control hooks.");}return true;
        }
        match crate::storage::generated_app_catalog(self.current_user.0,self.current_session.0,None){
            Ok(registry)=>{let mut count=0;for entry in registry.entries.iter().flatten(){
                self.output.write_segments(&[&entry.name[..entry.name_len],b"  ",&entry.source[..entry.source_len]]);count+=1;
            }if count==0{self.output.write_line(b"No verified generated applications are registered for this user.");}},
            Err(_)=>self.output.write_line(b"Sign in to access your application catalog."),
        }true
    }
    // ------------------------=
    // FUNC: register_created_application
    // DESC: Publishes a named application only after its source snapshot and real executable still match owned committed objects.
    // ------------------=
    pub(super) fn register_created_application(&self,source:&[u8],artifact:&[u8],fingerprint:u64)->Result<(),&'static[u8]>{
        let (source_meta,_)=crate::storage::object_inspect_path(source).map_err(|_|b"Application source is unavailable.".as_slice())?;
        let (artifact_meta,_)=crate::storage::object_inspect_path(artifact).map_err(|_|b"Application executable is unavailable.".as_slice())?;
        for metadata in [source_meta,artifact_meta]{if metadata.owner.0!=self.current_user.0||metadata.space!=crate::storage::object::Space::Personal{return Err(b"Only your owned Personal source and executable may be registered.");}}
        let mut bytes=[0;crate::ui::text_editor::DOCUMENT_CAPACITY];
        let (id,n)=crate::storage::object_read_path(source,Some(source_meta.current_version),&mut bytes).map_err(|_|b"Cannot verify application source.".as_slice())?;
        if id!=source_meta.object.id||crate::ui::app_assistant::fingerprint(&bytes[..n],0,None)!=fingerprint{return Err(b"Source changed during build; executable was not registered.");}
        let language=if source.ends_with(b".py"){Language::Python}else{Language::C};
        let entry=Entry::new(source,artifact,source_meta.object.id.0,artifact_meta.object.id.0,source_meta.current_version,artifact_meta.current_version,language)
            .map_err(|_|b"Application identity could not be registered.".as_slice())?;
        let mut registry=crate::storage::generated_app_catalog(self.current_user.0,self.current_session.0,None).map_err(|_|b"Application catalog is unavailable.".as_slice())?;
        registry.register(self.current_user.0,entry).map_err(|_|b"Application catalog is full or unauthorized.".as_slice())?;
        crate::storage::generated_app_catalog(self.current_user.0,self.current_session.0,Some(&registry)).map_err(|_|b"Application registration could not be saved.".as_slice())?;
        publish_catalog(1,registry.entries.iter().flatten().count(),Some(entry),true);Ok(())
    }
    // ------------------------=
    // FUNC: registered_application
    // DESC: Resolves one owned application and revalidates object identity and versions rather than trusting stale saved paths.
    // ------------------=
    pub(super) fn registered_application(&self,target:&[u8],launch:bool)->Result<Entry,&'static[u8]>{
        let registry=crate::storage::generated_app_catalog(self.current_user.0,self.current_session.0,None).map_err(|_|b"Sign in to access your application catalog.".as_slice())?;
        let entry=registry.find(self.current_user.0,target).map_err(|error|match error{Error::Ambiguous=>b"Several apps match; specify the exact source path.".as_slice(),_=>b"No registered application matches this name."})?;
        for(path,id,version)in [(&entry.source[..entry.source_len],entry.source_id,entry.source_version),(&entry.artifact[..entry.artifact_len],entry.artifact_id,entry.artifact_version)]{
            if !launch && id==entry.artifact_id && path!=&entry.source[..entry.source_len]{continue;}
            let(metadata,_)=crate::storage::object_inspect_path(path).map_err(|_|b"Registered application file is missing.".as_slice())?;
            if metadata.owner.0!=self.current_user.0||metadata.space!=crate::storage::object::Space::Personal||metadata.object.id.0!=id||(launch&&metadata.current_version!=version){
                return Err(b"Application source or executable changed; rebuild before launching.");
            }
        }Ok(entry)
    }
    // ------------------------=
    // FUNC: launch_registered_application
    // DESC: Uses existing native C and Python execution with no separate scheduler, shell interpolation or inferred permissions.
    // ------------------=
    pub(super) fn launch_registered_application(&mut self,target:&[u8])->bool{
        let entry=match self.registered_application(target,true){Ok(entry)=>entry,Err(error)=>{self.output.write_line(error);publish_catalog(2,0,None,false);return false;}};
        match entry.language{
            Language::C=>{
                #[cfg(all(feature="native-compiler",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
                {if compiler_commands::editor_running(self){self.output.write_line(b"Stop the current application first.");publish_catalog(2,0,Some(entry),false);return false;}
                    crate::ui::editor_chrome::terminal::mutate(|state|{state.attached=false;state.focused=false;state.restart=false;});
                    let _=compiler_commands::launch(self,&entry.artifact[..entry.artifact_len]);let success=compiler_commands::editor_running(self);publish_catalog(2,0,Some(entry),success);return success;}
            }
            Language::Python=>{
                #[cfg(all(feature="native-python",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
                {if python_commands::editor_running(self){self.output.write_line(b"Stop the current Python application first.");publish_catalog(2,0,Some(entry),false);return false;}
                    crate::ui::editor_chrome::terminal::mutate(|state|{state.attached=false;state.focused=false;state.restart=false;});
                    self.open_execution_console();let mut command=[0;102];command[..7].copy_from_slice(b"python ");command[7..7+entry.artifact_len].copy_from_slice(&entry.artifact[..entry.artifact_len]);
                    let _=python_commands::command(self,&command[..7+entry.artifact_len]);let success=python_commands::editor_running(self);publish_catalog(2,0,Some(entry),success);return success;}
            }
        }false
    }
}
