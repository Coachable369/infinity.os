//! Local forecast clock and the next 24 complete hours from one provider observation.
#[derive(Clone,Copy,Debug,PartialEq,Eq,Default)]
pub struct Hour {pub hour:u8,pub temperature:i16}
#[derive(Clone,Copy,Default)]
pub struct Outlook {
    pub hours:[Hour;24],pub count:usize,pub page:usize,
    pub observed:i64,pub sunrise:i64,pub sunset:i64,
}
// ------------------------=
// FUNC: field
// DESC: Locates a complete provider key without accepting a similarly named units section.
// ------------------=
fn field<'a>(text:&'a str,key:&str)->Option<&'a str>{
    text.split_once(key)?.1.trim_start().strip_prefix(':').map(str::trim_start)
}
// ------------------------=
// FUNC: numbers
// DESC: Iterates a bounded provider array until its closing bracket.
// ------------------=
fn numbers<'a>(text:&'a str,key:&str)->Option<impl Iterator<Item=&'a str>>{
    Some(field(text,key)?.strip_prefix('[')?.split_once(']')?.0.split(',').map(str::trim))
}
// ------------------------=
// FUNC: integer
// DESC: Reads an integer JSON field regardless of its position within the object.
// ------------------=
fn integer(text:&str,key:&str)->Option<i64>{
    field(text,key)?.split([',','}']).next()?.trim().parse().ok()
}
impl Outlook {
    // ------------------------=
    // FUNC: parse
    // DESC: Uses observation time and the city's UTC offset to label future hours and its solar arc.
    // ------------------=
    pub fn parse(text:&str)->Option<Self>{
        let current=field(text,"\"current\"")?.split_once('}')?.0;
        let hourly=field(text,"\"hourly\"")?.split_once('}')?.0;
        let daily=field(text,"\"daily\"")?.split_once('}')?.0;
        let observed=integer(current,"\"time\"")?;
        let offset=integer(text,"\"utc_offset_seconds\"")?;
        let mut out=Self{observed,sunrise:numbers(daily,"\"sunrise\"")?.next()?.parse().unwrap_or(0),
            sunset:numbers(daily,"\"sunset\"")?.next()?.parse().unwrap_or(0),..Self::default()};
        for (time,temp) in numbers(hourly,"\"time\"")?.zip(numbers(hourly,"\"temperature_2m\"")?){
            let Ok(epoch)=time.parse::<i64>() else {continue};
            let Ok(t)=temp.parse::<f32>() else {continue};
            if epoch<=observed || !(-150.0..=160.0).contains(&t){continue}
            out.hours[out.count]=Hour{hour:((epoch+offset).rem_euclid(86400)/3600)as u8,temperature:(t+if t<0.0{-0.5}else{0.5})as i16};
            out.count+=1;if out.count==24{break}
        }
        Some(out)
    }
}
