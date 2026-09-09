//! Atomic reserved native metadata writes independent of user namespace capacity.
use super::*;
impl<D:BlockDevice> ObjectStore<D>{
    // ------------------------=
    // FUNC: replace_reserved_state
    // DESC: Creates or replaces one exact reserved metadata record while preserving compatible legacy named state and atomic rollback.
    // ------------------=
    pub(crate) fn replace_reserved_state(&mut self,name:&[u8],legacy:&[u8],bytes:&[u8])->Result<(),ObjectError>{
        if bytes.len()>MAX_CONTENT{return Err(ObjectError::InvalidObject)}let before=self.begin()?;
        let result=(||{let id=self.reserve_system_metadata_record(name,legacy)?;self.replace_state_record(id,bytes).map(|_|())})();self.finish(before,result)
    }
    // ------------------------=
    // FUNC: replace_reserved_state_pair
    // DESC: Publishes both signed quorum state and its immutable payload under one native root without allocating user namespace slots.
    // ------------------=
    pub(crate) fn replace_reserved_state_pair(&mut self,first_name:&[u8],first_path:&[u8],first:&[u8],second_name:&[u8],second_path:&[u8],second:&[u8])->Result<(),ObjectError>{
        if first_name==second_name||first_path==second_path||first.len()>MAX_CONTENT||second.len()>MAX_CONTENT{return Err(ObjectError::InvalidObject)}
        let before=self.begin()?;let result=(||{
            let a=self.reserve_system_metadata_record(first_name,first_path)?;let b=self.reserve_system_metadata_record(second_name,second_path)?;
            if a==b{return Err(ObjectError::InvalidObject)}self.replace_state_record(a,first)?;self.replace_state_record(b,second)?;Ok(())
        })();self.finish(before,result)
    }
}
