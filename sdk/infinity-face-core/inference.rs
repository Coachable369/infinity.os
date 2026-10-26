//! Fixed float32 operator executor for the pinned face models. No allocator,
//! OS calls, threads, kernel access, ONNX parser or external runtime dependency.
//! Caller must authenticate the packed bytes and provide private bounded memory.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { Format, Bounds, Shape, Unsupported, Numeric, Cancelled }
#[derive(Clone, Copy)]
pub struct Tensor { pub rank: usize, pub shape: [usize;4], storage: usize, offset: usize, pub len: usize }
#[derive(Clone, Copy)]
pub struct Model<'a> { bytes: &'a [u8], tensors: usize, nodes: usize, outputs: usize, pub arena_len: usize, node_start: usize, output_start: usize, weight_start: usize }

// ------------------------=
// FUNC: word
// DESC: Reads one checked little-endian field without alignment assumptions.
// ------------------=
fn word(bytes: &[u8], at: usize) -> Result<usize, Error> {
    let s=bytes.get(at..at.checked_add(4).ok_or(Error::Bounds)?).ok_or(Error::Bounds)?;
    Ok(u32::from_le_bytes(s.try_into().map_err(|_|Error::Format)?) as usize)
}
impl<'a> Model<'a> {
    // ------------------------=
    // FUNC: kind
    // DESC: Identifies the pinned graph role after parsing, so detector and recognizer assets cannot be swapped accidentally.
    // ------------------=
    pub fn kind(&self)->usize {word(self.bytes,40).unwrap_or(0)}
    // ------------------------=
    // FUNC: parse
    // DESC: Checks packed tensor ranges and operation references before private worker execution; model authenticity is the launcher's responsibility.
    // ------------------=
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len()>48*1024*1024 || bytes.get(..8)!=Some(b"IFACE001") || !matches!(word(bytes,40)?,1|2) {return Err(Error::Format);}
        let tensors=word(bytes,44)?; let nodes=word(bytes,48)?; let outputs=word(bytes,52)?;
        let arena_len=word(bytes,56)?; let weights=word(bytes,60)?;
        if tensors==0 || tensors>512 || nodes==0 || nodes>256 || outputs==0 || outputs>12
            || arena_len==0 || arena_len>32*1024*1024 || weights>40*1024*1024 || weights%4!=0 {return Err(Error::Bounds);}
        let node_start=64+tensors*32; let output_start=node_start+nodes*64; let weight_start=output_start+outputs*4;
        if weight_start.checked_add(weights)!=Some(bytes.len()) {return Err(Error::Format);}
        let model=Self {bytes,tensors,nodes,outputs,arena_len,node_start,output_start,weight_start};
        for id in 0..tensors {
            let t=model.tensor(id)?;
            let capacity=if t.storage==0 {arena_len}else{weights/4};
            if t.offset.checked_add(t.len).is_none_or(|end|end>capacity) {return Err(Error::Bounds);}
        }
        if model.tensor(0)?.storage!=0 {return Err(Error::Format);}
        for n in 0..nodes {
            let at=node_start+n*64; let op=word(bytes,at)?; let out=model.tensor(word(bytes,at+4)?)?;
            let count=word(bytes,at+8)?;
            if !(1..=13).contains(&op) || !(1..=5).contains(&count) || out.storage!=0 {return Err(Error::Unsupported);}
            for i in 0..count {let input=model.tensor(word(bytes,at+12+i*4)?)?;
                if input.storage==0 && input.offset<out.offset+out.len && out.offset<input.offset+input.len {return Err(Error::Format);}
            }
        }
        for i in 0..outputs {model.tensor(word(bytes,output_start+i*4)?)?;}
        Ok(model)
    }
    // ------------------------=
    // FUNC: tensor
    // DESC: Validates tensor rank, dimensions, storage class and bounded element count.
    // ------------------=
    pub fn tensor(&self,id:usize)->Result<Tensor,Error> {
        if id>=self.tensors {return Err(Error::Bounds);}
        let at=64+id*32; let rank=word(self.bytes,at)?;
        if !(1..=4).contains(&rank) {return Err(Error::Shape);}
        let mut shape=[1;4]; let mut count=1usize;
        for (i,d) in shape.iter_mut().enumerate() {
            *d=word(self.bytes,at+4+i*4)?;
            if *d==0 || *d>16*1024*1024 || (i>=rank && *d!=1) {return Err(Error::Shape);}
            count=count.checked_mul(*d).ok_or(Error::Bounds)?;
        }
        let storage=word(self.bytes,at+20)?; let offset=word(self.bytes,at+24)?; let len=word(self.bytes,at+28)?;
        if storage>1 || len!=count || count>16*1024*1024 {return Err(Error::Bounds);}
        Ok(Tensor {rank,shape,storage,offset,len})
    }
    // ------------------------=
    // FUNC: read
    // DESC: Reads a checked float from immutable model data or the caller's private arena.
    // ------------------=
    fn read(&self,arena:&[f32],t:Tensor,index:usize)->Result<f32,Error> {
        if index>=t.len {return Err(Error::Bounds);}
        let value=if t.storage==0 {*arena.get(t.offset+index).ok_or(Error::Bounds)?}
            else {f32::from_bits(word(self.bytes,self.weight_start+(t.offset+index)*4)? as u32)};
        if value.is_finite() {Ok(value)}else{Err(Error::Numeric)}
    }
}

pub struct Execution<'a,'m> { model:Model<'m>, arena:&'a mut [f32], node:usize, element:usize, cancelled:bool }
impl<'a,'m> Execution<'a,'m> {
    // ------------------------=
    // FUNC: new
    // DESC: Prepares bounded private scratch; the isolated worker must supply the exact advertised arena length.
    // ------------------=
    pub fn new(model:Model<'m>,arena:&'a mut [f32])->Result<Self,Error> {
        if arena.len()!=model.arena_len {return Err(Error::Bounds);}
        arena.fill(0.0);
        Ok(Self {model,arena,node:0,element:0,cancelled:false})
    }
    // ------------------------=
    // FUNC: input
    // DESC: Exposes only the image input before inference begins; callers write the model-specific RGB/BGR layout here.
    // ------------------=
    pub fn input(&mut self)->Result<&mut [f32],Error> {
        if self.node!=0 || self.element!=0 || self.cancelled {return Err(Error::Bounds);}
        let t=self.model.tensor(0)?;
        self.arena.get_mut(t.offset..t.offset+t.len).ok_or(Error::Bounds)
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Prevents further execution and wipes private activation/image memory on cancellation.
    // ------------------=
    pub fn cancel(&mut self) {
        self.cancelled=true;
        for v in self.arena.iter_mut() {unsafe {core::ptr::write_volatile(v,0.0);}}
    }
    // ------------------------=
    // FUNC: output
    // DESC: Copies a completed graph output without exposing model or worker pointers to the caller.
    // ------------------=
    pub fn output(&self,index:usize,out:&mut [f32])->Result<Tensor,Error> {
        if self.cancelled {return Err(Error::Cancelled);}
        if self.node!=self.model.nodes || index>=self.model.outputs {return Err(Error::Bounds);}
        let t=self.model.tensor(word(self.model.bytes,self.model.output_start+index*4)?)?;
        if out.len()!=t.len {return Err(Error::Shape);}
        for (i,v) in out.iter_mut().enumerate() {*v=self.model.read(self.arena,t,i)?;}
        Ok(t)
    }
    // ------------------------=
    // FUNC: step
    // DESC: Evaluates at most a bounded number of output scalars; native process preemption additionally bounds large dot products.
    // ------------------=
    pub fn step(&mut self,budget:usize)->Result<bool,Error> {
        if self.cancelled {return Err(Error::Cancelled);}
        if budget==0 || budget>4096 {return Err(Error::Bounds);}
        let mut remaining=budget;
        while remaining>0 {
            if self.node==self.model.nodes {return Ok(true);}
            let at=self.model.node_start+self.node*64;
            let op=word(self.model.bytes,at)?;
            let out=self.model.tensor(word(self.model.bytes,at+4)?)?;
            let arity=word(self.model.bytes,at+8)?;
            let mut ins=[self.model.tensor(0)?;5];
            for (i,t) in ins.iter_mut().enumerate().take(arity) {*t=self.model.tensor(word(self.model.bytes,at+12+i*4)?)?;}
            let mut p=[0usize;8];
            for (i,v) in p.iter_mut().enumerate() {*v=word(self.model.bytes,at+32+i*4)?;}
            let count=remaining.min(out.len-self.element);
            for _ in 0..count {
                let value=self.scalar(op,arity,ins,out,p,self.element)?;
                if !value.is_finite() {return Err(Error::Numeric);}
                self.arena[out.offset+self.element]=value;
                self.element+=1;
            }
            remaining-=count;
            if self.element==out.len {self.element=0;self.node+=1;}
        }
        Ok(self.node==self.model.nodes)
    }
    // ------------------------=
    // FUNC: scalar
    // DESC: Implements only the packed models' fixed float32 operators with checked geometry and no allocations.
    // ------------------=
    fn scalar(&self,op:usize,arity:usize,t:[Tensor;5],out:Tensor,p:[usize;8],i:usize)->Result<f32,Error> {
        let read=|tensor,index|self.model.read(self.arena,tensor,index);
        let a=t[0];
        match op {
            1 | 4 => {
                if a.rank!=4 || out.rank!=4 || a.shape[0]!=1 || out.shape[0]!=1 || p[..4].contains(&0) || p[6]==0
                    || p[..6].iter().any(|&v|v>4096) {return Err(Error::Shape);}
                let [_,ic,ih,iw]=a.shape; let [_,oc,oh,ow]=out.shape;
                let c=i/(oh*ow); let y=(i/ow)%oh; let x=i%ow;
                let (kh,kw,sy,sx,py,px,group)=(p[0],p[1],p[2],p[3],p[4],p[5],p[6]);
                if op==1 && (arity<2 || ic%group!=0 || oc%group!=0 || t[1].shape!=[oc,ic/group,kh,kw]) {return Err(Error::Shape);}
                if op==4 && ic!=oc {return Err(Error::Shape);}
                let channels=if op==4 {1}else{ic/group};
                let first=if op==4 {c}else{(c/(oc/group))*channels};
                let mut value=if op==4 {f32::NEG_INFINITY}else if arity==3 {read(t[2],c)?}else{0.0};
                for k in 0..channels {for ky in 0..kh {for kx in 0..kw {
                    let yy=y*sy+ky; let xx=x*sx+kx;
                    if yy<py || xx<px || yy-py>=ih || xx-px>=iw {continue;}
                    let v=read(a,((first+k)*ih+yy-py)*iw+xx-px)?;
                    if op==4 {value=value.max(v);}else{value+=v*read(t[1],((c*channels+k)*kh+ky)*kw+kx)?;}
                }}}
                Ok(value)
            }
            2 => Ok(read(a,i)?.max(0.0)),
            3 => Ok(1.0/(1.0+exp(-read(a,i)?)?)),
            5 => {
                if a.rank!=4 || out.rank!=4 || a.shape[..2]!=out.shape[..2] {return Err(Error::Shape);}
                let [_,_,h,w]=out.shape; let [_,_,ah,aw]=a.shape;
                Ok(read(a,((i/(h*w))*ah+(i/w%h)*ah/h)*aw+(i%w)*aw/w)?)
            }
            6 | 7 | 8 | 9 => {
                if arity!=2 {return Err(Error::Shape);}
                let av=read(a,broadcast(a,out,i)?)?; let bv=read(t[1],broadcast(t[1],out,i)?)?;
                Ok(match op {6=>av+bv,7=>av-bv,8=>av*bv,_=>if av>=0.0 {av}else{av*bv}})
            }
            10 => {
                if arity!=3 || a.len!=out.len || a.rank<2 {return Err(Error::Shape);}
                let channel=(i/(a.len/(a.shape[0]*a.shape[1])))%a.shape[1];
                Ok(read(a,i)?*read(t[1],channel)?+read(t[2],channel)?)
            }
            11 => {
                if a.rank!=out.rank {return Err(Error::Shape);}
                let mut input_coords=[0usize;4];let mut index=i;let mut used=0usize;
                for j in (0..out.rank).rev() {
                    if p[j]>=a.rank || used&(1<<p[j])!=0 || out.shape[j]!=a.shape[p[j]] {return Err(Error::Shape);}
                    used|=1<<p[j];input_coords[p[j]]=index%out.shape[j];index/=out.shape[j];
                }
                let mut offset=0;
                for j in 0..a.rank {offset=offset*a.shape[j]+input_coords[j];}
                read(a,offset)
            }
            12 => {
                if arity<2 || a.rank!=2 || out.rank!=2 || t[1].rank!=2 || p[0]>1 || a.shape[0]!=out.shape[0] {return Err(Error::Shape);}
                let k=a.shape[1];let n=out.shape[1];let row=i/n;let column=i%n;
                let trans=p[0]==1;
                if t[1].shape[..2] != if trans {[n,k]}else{[k,n]} {return Err(Error::Shape);}
                let mut value=if arity==3 {read(t[2],broadcast(t[2],out,i)?)?}else{0.0};
                for j in 0..k {value+=read(a,row*k+j)?*read(t[1],if trans {column*k+j}else{j*n+column})?;}
                Ok(value)
            }
            13 => {if a.len!=out.len {return Err(Error::Shape);} read(a,i)}
            _ => Err(Error::Unsupported),
        }
    }
}
impl Drop for Execution<'_, '_> {
    // ------------------------=
    // FUNC: drop
    // DESC: Wipes image and activations when an inference visit is discarded, including failures.
    // ------------------=
    fn drop(&mut self) {self.cancel();}
}

// ------------------------=
// FUNC: broadcast
// DESC: Maps one output position to ONNX right-aligned broadcast coordinates without permitting implicit shape truncation.
// ------------------=
fn broadcast(input:Tensor,out:Tensor,mut index:usize)->Result<usize,Error> {
    if input.rank>out.rank {return Err(Error::Shape);}
    let mut offset=0;let mut stride=1;
    for j in (0..out.rank).rev() {
        let coordinate=index%out.shape[j];index/=out.shape[j];
        if j>=out.rank-input.rank {
            let d=input.shape[j-(out.rank-input.rank)];
            if d!=1 && d!=out.shape[j] {return Err(Error::Shape);}
            if d!=1 {offset+=coordinate*stride;}
            stride*=d;
        }
    }
    Ok(offset)
}

// ------------------------=
// FUNC: exp
// DESC: Computes a bounded range-reduced float exponential for detector sigmoid without libc or host math dependencies.
// ------------------=
pub(crate) fn exp(value:f32)->Result<f32,Error> {
    if !value.is_finite() {return Err(Error::Numeric);}
    let x=value.clamp(-80.0,80.0);let n=(x*core::f32::consts::LOG2_E) as i32;
    let r=x-(n as f32)*core::f32::consts::LN_2;
    let mut term=1.0;let mut sum=1.0;
    for k in 1..=10 {term*=r/k as f32;sum+=term;}
    Ok(sum*f32::from_bits(((n+127) as u32)<<23))
}
