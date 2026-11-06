//! Persistent owner-scoped File Navigator shortcuts.
use super::*;

impl<D:BlockDevice> ObjectStore<D> {
    // ------------------------=
    // FUNC: sidebar_location
    // DESC: Reads or saves a personal shortcut, validating destination ownership and keeping bounded replacement history.
    // ------------------=
    pub fn sidebar_location(&mut self, owner:ObjectId, home:user_home::Path,index:usize,save:Option<&[u8]>) -> Result<user_home::Path,ObjectError> {
        let index=if index==11{9}else{index};
        if index>=10 || self.metadata(self.resolve(home.as_bytes())?)?.owner!=owner {return Err(ObjectError::Unauthorized);}
        let file=home.child(b".navigator-sidebar").ok_or(ObjectError::InvalidPath)?;
        let mut values=[user_home::Path{bytes:[0;96],len:0};10];
        let mut bytes=[0u8;970];
        let id=self.resolve(file.as_bytes()).ok();
        if let Some(id)=id {
            if self.metadata(id)?.owner!=owner{return Err(ObjectError::Unauthorized);}
            let n=self.read(id,None,&mut bytes)?;
            for (i,line) in bytes[..n].split(|b|*b==b'\n').take(10).enumerate(){
                if !line.is_empty(){values[i]=sidebar_path(line)?;}
            }
        }
        if let Some(path)=save {
            let path=sidebar_path(path)?;
            let target=self.metadata(self.resolve(path.as_bytes())?)?;
            if target.kind!=ObjectType::NamespaceNode || (target.space==Space::Personal && target.owner.0!=[0;16] && target.owner.0!=owner.0){return Err(ObjectError::Unauthorized);}
            values[index]=path;
            let mut n=0;
            for value in values {let value=value.as_bytes();bytes[n..n+value.len()].copy_from_slice(value);n+=value.len();bytes[n]=b'\n';n+=1;}
            if let Some(id)=id {self.replace_state(id,&bytes[..n])?;} else {
                self.create_owned_attached(b"Sidebar mappings",ObjectType::Text,Space::Personal,&bytes[..n],file.as_bytes(),owner)?;
            }
        }
        if !values[index].as_bytes().is_empty(){return Ok(values[index]);}
        if index==9{return sidebar_path(b"/");}
        let child=[b"".as_slice(),b"",b"documents",b"downloads",b"pictures",b"music",b"videos",b"projects",b"trash"][index];
        let path=if child.is_empty(){home}else{home.child(child).ok_or(ObjectError::InvalidPath)?};
        Ok(path)
    }
}

// ------------------------=
// FUNC: sidebar_path
// DESC: Validates absolute object-namespace paths without host filesystem resolution.
// ------------------=
fn sidebar_path(bytes:&[u8])->Result<user_home::Path,ObjectError>{
    validate_path(bytes)?;
    let mut path=user_home::Path{bytes:[0;96],len:bytes.len()};
    path.bytes[..bytes.len()].copy_from_slice(bytes);Ok(path)
}
