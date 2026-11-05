//! Bounded session intent normalization and non-model login announcements.
// ------------------------=
// FUNC: logout
// DESC: Accepts direct self-signout intent with polite wrappers, excluding negation, questions about logout and other accounts.
// ------------------=
pub fn logout(input: &[u8]) -> bool {
    let mut text=input.trim_ascii();
    while text.last().is_some_and(|b|matches!(b,b'.'|b'!'|b'?')) {text=&text[..text.len()-1];}
    for prefix in [b"infinity ".as_slice(),b"could you ",b"can you ",b"would you ",b"please "] {
        if text.get(..prefix.len()).is_some_and(|s|s.eq_ignore_ascii_case(prefix)){text=&text[prefix.len()..];}
    }
    if text.len()>7&&text[text.len()-7..].eq_ignore_ascii_case(b" please"){text=&text[..text.len()-7];}
    [b"logout".as_slice(),b"log out",b"log me out",b"sign out",b"sign me out",b"log out of infinityos",b"sign me out of infinityos"]
        .iter().any(|phrase|text.eq_ignore_ascii_case(phrase))
}
// ------------------------=
// FUNC: welcome
// DESC: Formats a bounded plain speech greeting from the authenticated display name, never treating names as prompts or controls.
// ------------------=
pub fn welcome(name:&[u8],output:&mut [u8;160])->usize {
    output.fill(0);let prefix=b"Welcome back";output[..prefix.len()].copy_from_slice(prefix);let mut n=prefix.len();
    let mut first=true;
    for &b in name.iter().take(128) {
        if !(32..=126).contains(&b){continue;}
        if first&&b==b' ' {continue;}
        if first {output[n]=b',';output[n+1]=b' ';n+=2;first=false;}
        output[n]=b;n+=1;
    }
    while n>prefix.len()&&output[n-1]==b' ' {n-=1;}
    output[n]=b'.';n+1
}
