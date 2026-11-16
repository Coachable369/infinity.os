//! Bounded location and URL values shared by the weather actor and its host checks.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Text<const N:usize>{ pub bytes:[u8;N],pub len:usize }
impl<const N:usize> Text<N>{
    pub const EMPTY:Self=Self{bytes:[0;N],len:0};
    // ------------------------=
    // FUNC: add
    // DESC: Appends a complete byte slice or leaves the buffer unchanged.
    // ------------------=
    pub fn add(&mut self,b:&[u8])->Option<()> {self.bytes.get_mut(self.len..self.len+b.len())?.copy_from_slice(b);self.len+=b.len();Some(())}
    // ------------------------=
    // FUNC: value
    // DESC: Returns initialized text bytes.
    // ------------------=
    pub fn value(&self)->&[u8]{&self.bytes[..self.len]}
}
#[derive(Copy,Clone,Debug,PartialEq,Eq)]
pub struct Place{pub city:Text<64>,lat:Text<24>,lon:Text<24>}
impl Place{
    // ------------------------=
    // FUNC: new
    // DESC: Accepts a printable city and bounded decimal coordinates only.
    // ------------------=
    pub fn new(city:&str,lat:&str,lon:&str)->Option<Self>{
        let mut p=Self{city:Text::EMPTY,lat:Text::EMPTY,lon:Text::EMPTY};
        if city.trim().is_empty()||city.chars().any(char::is_control){return None}
        for (v,bound,out) in [(lat,90.0,&mut p.lat),(lon,180.0,&mut p.lon)]{
            if v.is_empty()||!v.bytes().all(|b|b.is_ascii_digit()||b==b'-'||b==b'.')||!(-bound..=bound).contains(&v.parse::<f64>().ok()?){return None}
            out.add(v.as_bytes())?;
        }
        p.city.add(city.as_bytes())?;Some(p)
    }
    // ------------------------=
    // FUNC: path
    // DESC: Builds a forecast request exclusively from validated coordinates.
    // ------------------=
    pub fn path(self)->Option<Text<192>>{let mut s=Text::EMPTY;for b in [b"/v1/forecast?latitude=".as_slice(),self.lat.value(),b"&longitude=",self.lon.value(),b"&current=temperature_2m,weather_code&temperature_unit=fahrenheit"]{s.add(b)?;}Some(s)}
    // ------------------------=
    // FUNC: encode
    // DESC: Serializes only the resolved locality for the owner's preferences.
    // ------------------=
    pub fn encode(self)->Text<120>{let mut s=Text::EMPTY;for b in [self.city.value(),b"\n",self.lat.value(),b"\n",self.lon.value()]{s.add(b).unwrap();}s}
    // ------------------------=
    // FUNC: decode
    // DESC: Revalidates stored preferences before requesting a forecast.
    // ------------------=
    pub fn decode(b:&[u8])->Option<Self>{let mut i=core::str::from_utf8(b).ok()?.split('\n');let p=Self::new(i.next()?,i.next()?,i.next()?)?;i.next().is_none().then_some(p)}
}
// ------------------------=
// FUNC: search
// DESC: Encodes field input as one city parameter, including spaces and reserved characters.
// ------------------=
pub fn search(city:&[u8])->Option<Text<256>>{
    if city.is_empty(){return None} let mut s=Text::EMPTY;s.add(b"/v1/search?count=1&language=en&name=")?;
    for &b in city{if b.is_ascii_alphanumeric(){s.add(&[b])?}else{let h=b"0123456789ABCDEF";s.add(&[b'%',h[(b>>4)as usize],h[(b&15)as usize]])?}}Some(s)
}
// ------------------------=
// FUNC: resolve
// DESC: Reads the first geocoder result or the opted-in network location response.
// ------------------=
pub fn resolve(bytes:&[u8],auto:bool)->Option<Place>{
    let text=core::str::from_utf8(bytes).ok()?;
    let object=if auto{text}else{text.split_once("\"results\"")?.1.trim_start().strip_prefix(':')?.trim_start().strip_prefix('[')?}.trim_start().strip_prefix('{')?.split_once('}')?.0;
    let field=|key:&str|object.split(',').find_map(|v|{let(k,val)=v.split_once(':')?;(k.trim().trim_matches('"')==key).then_some(val.trim())});
    if auto && field("success")!=Some("true"){return None}
    let city=field(if auto{"city"}else{"name"})?.strip_prefix('"')?.strip_suffix('"')?;
    if city.contains('\\'){return None} Place::new(city,field("latitude")?,field("longitude")?)
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    // ------------------------=
    // FUNC: location_preference_roundtrip
    // DESC: Verifies explicit and detected location selection produce valid durable forecast coordinates.
    // ------------------=
    fn location_preference_roundtrip(){
        let city=resolve(br#"{"results":[{"name":"Oslo","latitude":59.91,"longitude":10.74},{"name":"Paris","latitude":48.8,"longitude":2.3}]}"#,false).unwrap();
        let auto=resolve(br#"{"success":true,"city":"Oslo","latitude":59.91,"longitude":10.74}"#,true).unwrap();
        assert_eq!(city,auto);assert_eq!(Place::decode(auto.encode().value()),Some(city));assert_eq!(auto.path().unwrap().value(),b"/v1/forecast?latitude=59.91&longitude=10.74&current=temperature_2m,weather_code&temperature_unit=fahrenheit");
        assert!(resolve(b"{}",false).is_none());assert!(resolve(br#"{"success":false,"city":"Oslo","latitude":60,"longitude":10}"#,true).is_none());
        assert!(Place::decode(b"City\n92\n0").is_none());assert_eq!(search(b"St. John's").unwrap().value(),b"/v1/search?count=1&language=en&name=St%2E%20John%27s");
    }
}
