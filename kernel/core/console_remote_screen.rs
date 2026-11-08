//! Shared bounded terminal cell geometry, including tabs across wrapped rows.
// ------------------------=
// FUNC: walk
// DESC: Visits visible ASCII cells and returns the cursor after exactly the requested document bytes.
// ------------------=
fn walk(bytes:&[u8],limit:usize,columns:usize,mut visit:impl FnMut(usize,usize,u8))->(usize,usize){
    let columns=columns.max(1);let(mut row,mut column)=(0,0);
    for &byte in &bytes[..limit.min(bytes.len())]{
        if byte==b'\r'{continue;}
        if byte==b'\n'{row+=1;column=0;continue;}
        if column==columns{row+=1;column=0;}
        let count=if byte==b'\t'{4-column%4}else{1};
        for _ in 0..count{
            if column==columns{row+=1;column=0;}
            visit(row,column,if byte==b'\t'{b' '}else{byte});column+=1;
        }
    }
    if column==columns{row+=1;column=0;}(row,column)
}
// ------------------------=
// FUNC: position
// DESC: Resolves the same tab-aware cursor coordinates used to paint document cells.
// ------------------=
pub fn position(bytes:&[u8],cursor:usize,columns:usize)->(usize,usize){walk(bytes,cursor,columns,|_,_,_|{})}
// ------------------------=
// FUNC: cells
// DESC: Fills a clipped viewport with document cells without losing characters after wrapped tabs.
// ------------------=
pub fn cells(bytes:&[u8],top:usize,columns:usize,rows:usize,out:&mut[u8])->bool{
    let Some(length)=columns.checked_mul(rows).filter(|length|*length<=out.len()&&columns>0)else{return false;};
    out[..length].fill(b' ');
    walk(bytes,bytes.len(),columns,|row,column,byte|{if row>=top&&row-top<rows{out[(row-top)*columns+column]=byte;}});true
}
