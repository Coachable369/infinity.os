//! Validated settings mutations shared by editable aliases and native dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    Name { bytes: [u8; 48], length: usize },
    World { bytes: [u8; 48], length: usize },
    Primary(u32),
    Accent(u32),
    Opacity(u8),
    Blur(u8),
    LockMinutes(u8),
    Speech(bool),
    Theme(bool),
    Invalid,
}

// ------------------------=
// FUNC: color
// DESC: Accepts explicit RGB hex or common spoken colors without interpreting arbitrary model prose.
// ------------------=
fn color(value: &[u8]) -> Option<u32> {
    for (name, rgb) in [(b"blue".as_slice(), 0x168cff), (b"cyan", 0x22d3ee),
        (b"purple", 0x7c3aed), (b"green", 0x22c55e), (b"red", 0xef4444),
        (b"grey", 0x808080), (b"gray", 0x808080), (b"white", 0xffffff)] {
        if value.eq_ignore_ascii_case(name) { return Some(rgb); }
    }
    let hex = value.strip_prefix(b"#").unwrap_or(value);
    if hex.len() != 6 { return None; }
    hex.iter().try_fold(0u32, |result, byte| {
        let digit = match byte.to_ascii_lowercase() {
            b'0'..=b'9' => byte - b'0', b'a'..=b'f' => byte.to_ascii_lowercase() - b'a' + 10,
            _ => return None,
        };
        Some(result * 16 + u32::from(digit))
    }).filter(|rgb| *rgb != 0)
}

// ------------------------=
// FUNC: number
// DESC: Parses an explicit bounded decimal settings value without truncation or wrapping.
// ------------------=
fn number(value: &[u8]) -> Option<u8> {
    if value.is_empty() || value.len() > 3 { return None; }
    value.iter().try_fold(0u8, |n, b| {
        if !b.is_ascii_digit() { return None; }
        n.checked_mul(10)?.checked_add(b - b'0')
    })
}

impl Request {
    // ------------------------=
    // FUNC: spoken
    // DESC: Resolves explicit settings edits without model inference, preserving names and rejecting unknown values.
    // ------------------=
    pub fn spoken(input:&[u8])->Option<Self> {
        let mut input=input.trim_ascii();
        while input.last().is_some_and(|b|matches!(b,b'.'|b'!'|b'?')) {input=&input[..input.len()-1];}
        for prefix in [b"can you ".as_slice(),b"could you ",b"would you ",b"please "] {
            if input.get(..prefix.len()).is_some_and(|s|s.eq_ignore_ascii_case(prefix)){input=&input[prefix.len()..];}
        }
        for suffix in [b", please".as_slice(),b" please"] {
            if input.len()>=suffix.len() && input[input.len()-suffix.len()..].eq_ignore_ascii_case(suffix){input=&input[..input.len()-suffix.len()];}
        }
        let verb=[b"set ".as_slice(),b"change ",b"switch ",b"rename ",b"turn "]
            .into_iter().find(|p|input.get(..p.len()).is_some_and(|s|s.eq_ignore_ascii_case(p)))?;
        input=&input[verb.len()..];
        for prefix in [b"the ".as_slice(),b"my "] {
            if input.get(..prefix.len()).is_some_and(|s|s.eq_ignore_ascii_case(prefix)){input=&input[prefix.len()..];}
        }
        for (name,key) in [
            (b"world shift".as_slice(),b"world".as_slice()),(b"worldshift",b"world"),
            (b"computer name",b"name"),(b"machine name",b"name"),(b"computer",b"name"),
            (b"name of the computer",b"name"),(b"name of my computer",b"name"),(b"name of computer",b"name"),
            (b"computer's name",b"name"),(b"hostname",b"name"),(b"device name",b"name"),
            (b"primary color",b"primary"),(b"color scheme",b"primary"),(b"accent color",b"accent"),
            (b"glass opacity",b"opacity"),(b"background blur",b"blur"),(b"inactivity timeout",b"lockMinutes"),
            (b"speech output",b"speech"),(b"ui theme",b"theme"),(b"color theme",b"theme"),(b"theme",b"theme")
        ] {
            if !input.get(..name.len()).is_some_and(|s|s.eq_ignore_ascii_case(name)){continue;}
            let tail=&input[name.len()..];
            if !tail.is_empty() && tail[0]!=b' ' {continue;}
            let mut value=tail.trim_ascii();
            if value.get(..3).is_some_and(|s|s.eq_ignore_ascii_case(b"to ")){value=&value[3..];}
            let mut protocol=[0u8;128];
            if key.len()+1+value.len()>protocol.len(){return Some(Self::Invalid);}
            protocol[..key.len()].copy_from_slice(key);protocol[key.len()]=b':';
            protocol[key.len()+1..key.len()+1+value.len()].copy_from_slice(value);
            return Self::protocol(&protocol[..key.len()+1+value.len()]);
        }
        None
    }
    // ------------------------=
    // FUNC: protocol
    // DESC: Converts allowlisted settings keys into typed arguments; invalid values remain errors rather than inference requests.
    // ------------------=
    pub fn protocol(input: &[u8]) -> Option<Self> {
        let split = input.iter().position(|b| *b == b':')?;
        let value = input[split+1..].trim_ascii();
        Some(match &input[..split] {
            b"name" | b"world" => {
                let valid=if &input[..split]==b"name" {
                    core::str::from_utf8(value).is_ok_and(|name|name.chars().all(|c|
                        c.is_alphanumeric() || matches!(c,' ' | '-' | '_' | '.' | '\'' | '\u{2019}')))
                } else {
                    value.iter().all(|b|b.is_ascii_alphanumeric() || matches!(b,b' ' | b'-' | b'_' | b'.'))
                };
                if value.is_empty() || value.len() > 48 || !valid {
                    Self::Invalid
                } else {
                    let mut bytes = [0; 48]; bytes[..value.len()].copy_from_slice(value);
                    if &input[..split]==b"world" {Self::World{bytes,length:value.len()}}
                    else {Self::Name { bytes, length: value.len() }}
                }
            }
            b"primary" => color(value).map(Self::Primary).unwrap_or(Self::Invalid),
            b"accent" => color(value).map(Self::Accent).unwrap_or(Self::Invalid),
            b"opacity" => number(value).filter(|n| (40..=100).contains(n) && (n-40)%4 == 0).map(Self::Opacity).unwrap_or(Self::Invalid),
            b"blur" => number(value).filter(|n| *n <= 8).map(Self::Blur).unwrap_or(Self::Invalid),
            b"lockMinutes" => number(value).filter(|n| (1..=120).contains(n)).map(Self::LockMinutes).unwrap_or(Self::Invalid),
            b"speech" => if value.eq_ignore_ascii_case(b"on") { Self::Speech(true) } else if value.eq_ignore_ascii_case(b"off") { Self::Speech(false) } else { Self::Invalid },
            b"theme" => if value.eq_ignore_ascii_case(b"dark") || value.eq_ignore_ascii_case(b"Cosmic Dark") { Self::Theme(false) } else if value.eq_ignore_ascii_case(b"light") || value.eq_ignore_ascii_case(b"Diagnostic Light") { Self::Theme(true) } else { color(value).map(Self::Primary).unwrap_or(Self::Invalid) },
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    // ------------------------=
    // FUNC: validates_arguments
    // DESC: Exercises typed settings input, value bounds and injection rejection.
    // ------------------=
    #[test]
    fn validates_arguments() {
        use super::Request as R;
        assert_eq!(R::protocol(b"accent:Grey"), Some(R::Accent(0x808080)));
        assert_eq!(R::protocol(b"primary:#102030"), Some(R::Primary(0x102030)));
        assert_eq!(R::protocol(b"theme:dark"), Some(R::Theme(false)));
        assert_eq!(R::protocol(b"speech:on"), Some(R::Speech(true)));
        for input in [b"opacity:41".as_slice(), b"blur:9", b"lockMinutes:0", b"lockMinutes:256", b"name:hello;reboot", b"theme:uninstalled"] {
            assert_eq!(R::protocol(input), Some(R::Invalid));
        }
        assert_eq!(R::protocol(b"erase:everything"), None);
    }

    // ------------------------=
    // FUNC: spoken_settings_are_typed
    // DESC: Verifies voice phrasing, punctuation, preserved names and fail-closed setting values.
    // ------------------=
    #[test]
    fn spoken_settings_are_typed() {
        use super::Request as R;
        for text in [b"Change Worldshift to Research".as_slice(),b"set the world shift to Research.",b"Please switch my worldshift to Research"] {
            assert_eq!(R::spoken(text),R::protocol(b"world:Research"));
        }
        assert_eq!(R::spoken(b"Set Worldshift to reserch"),R::protocol(b"world:reserch"));
        assert_eq!(R::spoken(b"Change the name of the computer to Studio"),R::protocol(b"name:Studio"));
        assert_eq!(R::spoken(b"Can you change the computer name to Studio, please?"),R::protocol(b"name:Studio"));
        for input in ["Change the computer name to Jonathan's computer.",
            "Change the machine name to Jonathan's computer.","Change the computer name to Jonathan’s computer."] {
            assert!(matches!(R::spoken(input.as_bytes()),Some(R::Name{..})));
        }
        assert_eq!(R::spoken(b"set theme to unknown"),Some(R::Invalid));
        assert_eq!(R::spoken(b"set background blur to 255"),Some(R::Invalid));
        assert_eq!(R::spoken(b"do not change computer name to Studio"),None);
        assert_eq!(R::spoken(b"explain how to change worldshift to Research"),None);
    }
}
