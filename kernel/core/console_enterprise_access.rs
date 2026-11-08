//! Actor-bound workplace storage and clipboard admission shared by local and SSH command sessions.
use crate::{runtime::identity::{StableId,SESSION_PERSONAL_READ,SESSION_PERSONAL_WRITE},storage::object::{ObjectMetadata,ObjectId,Space,ContentType,ObjectType}};
use infinity_enterprise_core::{Error,Files};

pub(super) struct NativeFiles {pub owner:StableId,pub session:StableId}
// ------------------------=
// FUNC: storage_error
// DESC: Preserves native authorization, collision and capacity failures in workplace receipts.
// ------------------=
pub(super) fn storage_error(error:crate::storage::object::ObjectError)->Error {
    use crate::storage::object::ObjectError as E;
    match error {E::Unauthorized=>Error::Denied,E::InsufficientCapacity=>Error::Capacity,E::NameConflict=>Error::Conflict,_=>Error::Storage}
}
// ------------------------=
// FUNC: clear_clipboard
// DESC: Checks the complete owning session before erasing shared clipboard content, history or browser grants.
// ------------------=
pub(super) fn clear_clipboard(clipboard:&mut crate::ui::clipboard::SessionClipboard,session:[u8;16])->Result<(),Error> {
    if !clipboard.authorized(session){return Err(Error::Denied);}clipboard.clear();Ok(())
}
impl NativeFiles {
    // ------------------------=
    // FUNC: authorized
    // DESC: Revalidates the active account, credential and exact session capability for every native workplace operation.
    // ------------------=
    pub(super) fn authorized(&self,write:bool)->bool {
        let required=if write{SESSION_PERSONAL_WRITE}else{SESSION_PERSONAL_READ};
        crate::runtime::with_runtime(|r|r.identity.console_session_authorized(self.session,self.owner)
            && (0..crate::runtime::identity::MAX_SESSIONS).filter_map(|i|r.identity.session_nth(i))
                .any(|s|s.id==self.session&&s.user==self.owner&&s.capabilities&required!=0)).unwrap_or(false)
    }
    // ------------------------=
    // FUNC: metadata
    // DESC: Admits only the caller's Personal objects or explicitly shared owner-zero Personal objects under native editor policy.
    // ------------------=
    pub(super) fn metadata(&self,path:&[u8],write:bool)->Result<ObjectMetadata,Error> {
        if !self.authorized(write){return Err(Error::Denied);}
        let (metadata,_)=crate::storage::object_inspect_path(path).map_err(storage_error)?;
        if metadata.space!=Space::Personal{return Err(Error::Invalid);}
        if metadata.owner.0!=[0;16]&&metadata.owner.0!=self.owner.0{return Err(Error::Denied);}
        Ok(metadata)
    }
    // ------------------------=
    // FUNC: destination
    // DESC: Requires a canonical unused path beneath an authorized Personal directory before creating or copying a workplace file.
    // ------------------=
    pub(super) fn destination(&self,path:&[u8])->Result<(),Error> {
        if !self.authorized(true){return Err(Error::Denied);}
        let normalized=crate::runtime::object_navigation::normalize_absolute_path(path).map_err(|_|Error::Invalid)?;
        if normalized.as_bytes()!=path{return Err(Error::Invalid);}
        let at=path.iter().rposition(|b|*b==b'/').ok_or(Error::Invalid)?;
        let parent=self.metadata(&path[..at.max(1)],true)?;
        if parent.kind!=ObjectType::NamespaceNode{return Err(Error::Invalid);}
        match crate::storage::namespace_resolve(path){Ok(_)=>Err(Error::Conflict),Err(crate::storage::object::ObjectError::NotFound|crate::storage::object::ObjectError::NamespaceNotFound)=>Ok(()),Err(e)=>Err(storage_error(e))}
    }
    // ------------------------=
    // FUNC: create
    // DESC: Creates a caller-owned support report only after live session and parent-directory authorization.
    // ------------------=
    pub(super) fn create(&self,path:&[u8],bytes:&[u8])->Result<(),Error> {
        self.destination(path)?;
        crate::storage::object_create_owned_note_at(crate::runtime::object_navigation::namespace_basename(path),bytes,path,ObjectId(self.owner.0)).map(|_|()).map_err(storage_error)
    }
}
impl Files for NativeFiles {
    // ------------------------=
    // FUNC: read
    // DESC: Reads exact authorized regular-file content, preserving native protection of private checkpoints and credentials.
    // ------------------=
    fn read(&mut self,path:&[u8],out:&mut[u8])->Result<usize,Error> {
        let metadata=self.metadata(path,false)?;
        if matches!(metadata.content_type,ContentType::Namespace|ContentType::System){return Err(Error::Invalid);}
        if metadata.logical_size as usize>out.len(){return Err(Error::Capacity);}
        let (_,n)=crate::storage::object_read_path(path,None,out).map_err(storage_error)?;
        if n!=metadata.logical_size as usize{return Err(Error::Integrity);}Ok(n)
    }
    // ------------------------=
    // FUNC: copy
    // DESC: Authorizes the source and destination before copying while preserving native ownership, type and atomic collision handling.
    // ------------------=
    fn copy(&mut self,source:&[u8],destination:&[u8])->Result<(),Error> {
        let metadata=self.metadata(source,false)?;
        if matches!(metadata.content_type,ContentType::Namespace|ContentType::System){return Err(Error::Invalid);}
        self.destination(destination)?;
        crate::storage::object_copy_path(source,destination).map(|_|()).map_err(storage_error)
    }
    // ------------------------=
    // FUNC: write
    // DESC: Revalidates write capability and target ownership before committing one native content version.
    // ------------------=
    fn write(&mut self,path:&[u8],bytes:&[u8])->Result<u32,Error> {
        let metadata=self.metadata(path,true)?;
        if matches!(metadata.content_type,ContentType::Namespace|ContentType::System){return Err(Error::Invalid);}
        crate::storage::object_write_path(path,bytes).map_err(storage_error)
    }
}
