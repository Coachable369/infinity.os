//! Bounded setup worker lifecycle. The caller is responsible for selecting the
//! packaged executable/models and authorizing the account before launch.
use super::{execution::SecurityIdentity, face_setup::{Setup,Stage},face_setup_dispatch::Dispatch,
    native_app_task::Task,native_cpu as cpu,native_memory::{NativeMemory,MemoryError},native_process::State};
#[path="face_schedule.rs"] mod schedule;

pub struct Job {
    pub setup:Setup,
    task:Option<Task>,
    owner:SecurityIdentity,
    detector:&'static [u8],
    recognizer:&'static [u8],
    deadline:u64,
    camera_open:bool,
    terminal:Option<State>,
    scrubbed:usize,
    account:super::identity::StableId,
    login:bool,
    proof:Option<super::face_login::Proof>,
}
impl Job {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a dormant OS-owned job with no executable, models, camera lease, or account authority.
    // ------------------=
    pub const fn new()->Self {
        Self {setup:Setup::new(),task:None,owner:SecurityIdentity([0;16]),detector:&[],recognizer:&[],
            deadline:0,camera_open:false,terminal:None,scrubbed:0,account:super::identity::StableId([0;16]),login:false,proof:None}
    }
    // ------------------------=
    // FUNC: start_setup
    // DESC: Starts the camera comparison only after password verification; rejected startup releases setup authority immediately.
    // ------------------=
    pub unsafe fn start_setup(&mut self,image:&[u8],models:[&'static [u8];2],owner:SecurityIdentity,
        worker:u64,account:super::identity::StableId,secret:&[u8],digest:[u8;32],name:&str,now_ns:u64,
        memory:&mut NativeMemory,controller:cpu::Controller)->Result<(),MemoryError> {
        if self.task.is_some() {return Err(MemoryError::AccessDenied);}
        if !crate::drivers::camera::authorize_setup(owner,account,secret) {return Err(MemoryError::AccessDenied);}
        // The full account identity remains bound in the camera authority record.
        // The worker receives a per-attempt opaque handle, not a truncated account ID.
        let result=self.start(image,models,owner,worker,worker,digest,name,None,now_ns,memory,controller);
        if result.is_ok() {self.account=account;}
        if result.is_err() {crate::drivers::camera::revoke_setup(owner);}
        result
    }
    // ------------------------=
    // FUNC: start
    // DESC: Maps a trusted packaged worker into private pages after the caller's password authorization; no inference runs in the kernel.
    // ------------------=
    unsafe fn start(&mut self, image:&[u8],models:[&'static [u8];2],owner:SecurityIdentity,
        worker:u64,account:u64,digest:[u8;32],name:&str,payload:Option<&[u8]>,now_ns:u64,
        memory:&mut NativeMemory,controller:cpu::Controller)->Result<(),MemoryError> {
        if self.task.is_some() || owner.0==[0;16] || models.iter().any(|m|m.len()<64 || m.len()>48*1024*1024) {
            return Err(MemoryError::InvalidAddress);
        }
        self.login=payload.is_some();self.proof=None;
        if let Some(payload)=payload {self.setup.begin_login(worker,digest,payload,now_ns/1_000_000)}
        else {self.setup.begin(worker,account,digest,name,now_ns/1_000_000)}.map_err(|_|MemoryError::InvalidAddress)?;
        self.deadline=now_ns.saturating_add(240_000_000_000);
        let task=match Task::start_camera_worker(image,owner,7,memory,controller,self.deadline) {
            Ok(task)=>task,Err(error)=>{self.setup.cancel();return Err(error);}
        };
        self.owner=owner;self.detector=models[0];self.recognizer=models[1];
        self.camera_open=false;self.terminal=None;self.scrubbed=0;self.task=Some(task);Ok(())
    }
    // ------------------------=
    // FUNC: poll
    // DESC: Executes one timer-preemptible user slice and one bounded service dispatch, preserving desktop scheduling and prompt cancellation.
    // ------------------=
    pub fn poll(&mut self, authorized:bool,now_ns:u64,memory:&mut NativeMemory)->Result<(),MemoryError> {
        let Some(mut task)=self.task.take() else {return Ok(());};
        if self.terminal.is_some() {
            if task.scrub_private(&mut self.scrubbed) {task.release(memory)?;}else{self.task=Some(task);}
            return Ok(());
        }
        let alive=authorized && (if self.login {crate::drivers::camera::face_login_authorized(self.owner,self.account)} else {crate::drivers::camera::setup_authorized(self.owner,self.account)})
            && now_ns<self.deadline && self.setup.tick(now_ns/1_000_000).is_ok()
            && !matches!(self.setup.stage(),Stage::Cancelled|Stage::Failed(_))
            && (self.login || self.setup.stage()!=Stage::Passed);
        let owner=self.owner;
        let mut dispatch=Dispatch {owner,setup:&mut self.setup,detector:self.detector,recognizer:self.recognizer};
        let open=&mut self.camera_open;
        let state=schedule::batch(super::ai::qwen::workers::clock_ns,|slice_now| {
        let state=task.poll(slice_now,alive,|caller,tables,op,args| {
            if caller!=owner || !alive {return -13;}
            if (0x1260..=0x1262).contains(&op) {
                let command=dispatch.setup.control();
                if !(dispatch.setup.pending() || matches!(dispatch.setup.stage(),Stage::Photos(_)))
                    || !matches!(command.command,super::face_setup::wire::BEGIN|super::face_setup::wire::CAPTURE|super::face_setup::wire::VERIFY|super::face_setup::wire::LOGIN) {return -13;}
                if op==0x1260 && !*open {
                    if !crate::drivers::camera::grant(owner) {return -11;}
                    *open=true;
                }
                let result=crate::drivers::camera::dispatch(owner,tables,op,args);
                if op==0x1262 {crate::drivers::camera::cancel(owner);*open=false;}
                return result;
            }
            dispatch.call(caller,alive,tables,op,args,slice_now/1_000_000)
        });
        (state!=State::Runnable).then_some(state)
        }).unwrap_or(State::Runnable);
        if !self.login && self.setup.stage()==Stage::Passed && !self.setup.saved() {
            let saved=self.save_identity(now_ns);
            self.setup.finish_save(saved);
            crate::drivers::camera::cancel(owner);crate::drivers::camera::revoke_setup(owner);
            self.camera_open=false;
        }
        if state==State::Runnable {
            self.task=Some(task);
        } else {
            if self.login && alive && state==State::Exited(0) && self.setup.stage()==Stage::Passed {
                self.proof=Some(super::face_login::Proof::new(self.account,now_ns/1_000_000));
            }
            crate::drivers::camera::cancel(owner);crate::drivers::camera::revoke_setup(owner);
            self.camera_open=false;self.terminal=Some(state);
            if !matches!(self.setup.stage(),Stage::Passed|Stage::Failed(_)|Stage::Cancelled) {self.setup.cancel();}
            // Reclaim only after cooperative wiping, including on fault or forced cancellation.
            self.task=Some(task);
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: save_identity
    // DESC: Binds verified data to the password-authorized full account ID, encrypts before persistence and verifies decrypted durable readback.
    // ------------------=
    fn save_identity(&self,now_ns:u64)->bool {
        if !crate::drivers::camera::setup_authorized(self.owner,self.account) {return false;}
        let Some(payload)=self.setup.enrollment() else{return false;};
        let Some(key)=super::with_runtime(|runtime| {
            runtime.identity.user(self.account)?;
            runtime.nodes.face_vault_key(&self.account.0)
        }).flatten() else{return false;};
        // The existing boot-seeded native entropy service is shared, but vault
        // keys and authenticated record contexts remain domain-separated.
        let Some(seed)=(unsafe {super::native_python_entropy::next(&self.account.0)}) else{return false;};
        let seed=zeroize::Zeroizing::new(seed);
        let mut nonce=[0;12];nonce.copy_from_slice(&seed[..12]);
        let control=self.setup.control();
        let Some(record)=super::face_vault::seal(&key,self.account.0,&control,nonce,now_ns,payload) else{return false;};
        if crate::storage::face_identity_commit(self.account.0,&*record).is_err() {return false;}
        let mut readback=zeroize::Zeroizing::new([0;super::face_vault::BYTES]);
        if crate::storage::face_identity_load(self.account.0,&mut *readback)!=Ok(super::face_vault::BYTES) {return false;}
        let Some(plain)=super::face_vault::open(&key,self.account.0,control.model,&*readback) else{return false;};
        plain[super::face_vault::HEADER..super::face_vault::BYTES-16]==*payload
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Revokes hardware capture immediately and schedules private worker memory reclamation at the next poll.
    // ------------------=
    pub fn cancel(&mut self) {
        self.proof=None;
        if !matches!(self.setup.stage(),Stage::Failed(_)) {self.setup.cancel();}
        crate::drivers::camera::cancel(self.owner);
        crate::drivers::camera::revoke_setup(self.owner);self.camera_open=false;
    }
    // ------------------------=
    // FUNC: terminal
    // DESC: Exposes process failure separately from recognition progress so a crashed worker cannot appear to remain processing.
    // ------------------=
    pub fn terminal(&self)->Option<State> {self.terminal}

    // ------------------------=
    // FUNC: start_login
    // DESC: Authenticates the account/model-bound encrypted enrollment before granting the fixed worker pre-session camera access.
    // ------------------=
    pub unsafe fn start_login(&mut self,image:&[u8],models:[&'static [u8];2],owner:SecurityIdentity,worker:u64,
        account:super::identity::StableId,digest:[u8;32],now:u64,memory:&mut NativeMemory,controller:cpu::Controller)->Result<(),MemoryError> {
        if self.task.is_some() {return Err(MemoryError::AccessDenied);}
        let key=zeroize::Zeroizing::new(super::with_runtime(|r|r.nodes.face_vault_key(&account.0)).flatten().ok_or(MemoryError::AccessDenied)?);
        let mut record=zeroize::Zeroizing::new([0;super::face_vault::BYTES]);
        if crate::storage::face_identity_load(account.0,&mut *record)!=Ok(record.len()) {return Err(MemoryError::AccessDenied);}
        let plain=super::face_vault::open(&key,account.0,digest,&*record).ok_or(MemoryError::AccessDenied)?;
        if !crate::drivers::camera::authorize_face_login(owner,account) {return Err(MemoryError::AccessDenied);}
        let result=self.start(image,models,owner,worker,worker,digest,"Login",Some(&plain[super::face_vault::HEADER..super::face_vault::BYTES-16]),now,memory,controller);
        if result.is_ok() {self.account=account;}else{crate::drivers::camera::revoke_setup(owner);}
        result
    }
    // ------------------------=
    // FUNC: take_login_proof
    // DESC: Delivers the successful fixed-worker result once; cancellation and failures cannot produce it.
    // ------------------=
    pub fn take_login_proof(&mut self)->Option<super::face_login::Proof> {self.proof.take()}
}
