//! Exact, bounded arithmetic for explicitly supplied decision matrices.
pub const MAX_OPTIONS: usize = 8;
pub const MAX_CRITERIA: usize = 8;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { Syntax, Weights, Scores, Eligibility, NoEligibleOption }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ranking { pub scores: [u32; MAX_OPTIONS], pub count: usize, pub eligible: u8, pub best: u8 }
impl Ranking {
    // ------------------------=
    // FUNC: winner
    // DESC: Returns a one-based unique winner; an exact tie is never silently broken.
    // ------------------=
    pub fn winner(self) -> Option<u8> { (self.best.count_ones()==1).then_some(self.best.trailing_zeros() as u8+1) }
}

// ------------------------=
// FUNC: numbers
// DESC: Parses a bounded unsigned comma-separated tuple, rejecting empty values, signs, and excess elements.
// ------------------=
fn numbers(input: &[u8], out: &mut [u16], maximum: u16) -> Result<usize, Error> {
    let mut count=0;
    for part in input.split(|b| *b==b',') {
        if count==out.len() || part.is_empty() { return Err(Error::Syntax); }
        let mut value=0u16;
        for byte in part {
            if !byte.is_ascii_digit() { return Err(Error::Syntax); }
            value=value.checked_mul(10).and_then(|v|v.checked_add((byte-b'0') as u16)).ok_or(Error::Syntax)?;
        }
        if value>maximum { return Err(Error::Syntax); }
        out[count]=value; count+=1;
    }
    Ok(count)
}

// ------------------------=
// FUNC: rank
// DESC: Calculates weighted sums from supplied scores, applies explicit hard-constraint eligibility, and preserves exact ties.
// ------------------=
pub fn rank(input: &[u8]) -> Result<Option<Ranking>, Error> {
    let mut weights=None; let mut scores=None; let mut eligibility=None;
    for word in input.split(|byte|byte.is_ascii_whitespace()) {
        let (slot,value)=if let Some(value)=word.strip_prefix(b"weights=") { (&mut weights,value) }
            else if let Some(value)=word.strip_prefix(b"scores=") { (&mut scores,value) }
            else if let Some(value)=word.strip_prefix(b"eligible=") { (&mut eligibility,value) }
            else {continue;};
        if slot.replace(value).is_some() { return Err(Error::Syntax); }
    }
    if weights.is_none()&&scores.is_none()&&eligibility.is_none() { return natural(input); }
    let mut weight=[0u16;MAX_CRITERIA];
    let criteria=numbers(weights.ok_or(Error::Weights)?,&mut weight,100).map_err(|_|Error::Weights)?;
    if weight[..criteria].iter().map(|v|*v as u32).sum::<u32>()!=100 { return Err(Error::Weights); }
    let mut result=Ranking { scores:[0;MAX_OPTIONS], count:0, eligible:0, best:0 };
    for row in scores.ok_or(Error::Scores)?.split(|byte|*byte==b'/') {
        if result.count==MAX_OPTIONS { return Err(Error::Scores); }
        let mut values=[0u16;MAX_CRITERIA];
        if numbers(row,&mut values,1000).map_err(|_|Error::Scores)?!=criteria { return Err(Error::Scores); }
        result.scores[result.count]=(0..criteria).map(|i|weight[i] as u32*values[i] as u32).sum();
        result.eligible|=1<<result.count; result.count+=1;
    }
    if result.count<2 { return Err(Error::Scores); }
    if let Some(eligibility)=eligibility {
        let mut values=[0u16;MAX_OPTIONS];
        if numbers(eligibility,&mut values,1).map_err(|_|Error::Eligibility)?!=result.count { return Err(Error::Eligibility); }
        result.eligible=0;
        for (index,value) in values[..result.count].iter().enumerate() { if *value==1 { result.eligible|=1<<index; } }
    }
    if result.eligible==0 { return Err(Error::NoEligibleOption); }
    let best=(0..result.count).filter(|i|result.eligible&(1<<i)!=0).map(|i|result.scores[i]).max().ok_or(Error::NoEligibleOption)?;
    for index in 0..result.count { if result.eligible&(1<<index)!=0&&result.scores[index]==best {result.best|=1<<index;} }
    Ok(Some(result))
}


// ------------------------=
// FUNC: word
// DESC: Removes sentence separators while preserving interior decimal marks, signs, and percent suffixes for strict numeric validation.
// ------------------=
fn word(bytes: &[u8]) -> &[u8] {
    let mut text=bytes;
    while text.first().is_some_and(|b|matches!(b,b','|b':'|b';')) {text=&text[1..];}
    while text.last().is_some_and(|b|matches!(b,b','|b':'|b';'|b'.')) {text=&text[..text.len()-1];}
    text
}

// ------------------------=
// FUNC: scalar
// DESC: Parses one unsigned whole-number score or percentage without rounding, locale guesses, or implicit units.
// ------------------=
fn scalar(text: &[u8], maximum: u16) -> Result<u16,Error> {
    let text=word(text);
    let text=text.strip_suffix(b"%").unwrap_or(text);
    let mut value=[0u16;1];
    numbers(text,&mut value,maximum)?;
    Ok(value[0])
}

// ------------------------=
// FUNC: natural
// DESC: Grounds explicit named percentage weights and numbered option score clauses; incomplete or ambiguous numeric matrices never receive invented scores.
// ------------------=
fn natural(input: &[u8]) -> Result<Option<Ranking>,Error> {
    if !input.split(|byte|byte.is_ascii_whitespace()).any(|token|word(token).eq_ignore_ascii_case(b"weight")) {return Ok(None);}
    let mut tokens=[b"".as_slice();512]; let mut separators=[0u8;512];let mut length=0;
    for token in input.split(|byte|byte.is_ascii_whitespace()).filter(|token|!word(token).is_empty()) {
        if length==tokens.len() {return Err(Error::Syntax);}
        separators[length]=if token.ends_with(b":") {1} else if token.last().is_some_and(|byte|matches!(byte,b'.'|b'?'|b'!')) {2}else{0};
        tokens[length]=word(token);length+=1;
    }
    let tokens=&tokens[..length];
    let mut labels=[b"".as_slice();MAX_CRITERIA]; let mut weights=[0u16;MAX_CRITERIA];let mut criteria=0;
    for window in tokens.windows(3) {
        if !window[1].eq_ignore_ascii_case(b"weight") || !window[2].ends_with(b"%") {continue;}
        let label=window[0];
        if label.is_empty()||label.len()>32||!label.iter().all(|byte|byte.is_ascii_alphabetic()) {return Err(Error::Weights);}
        if criteria==MAX_CRITERIA||labels[..criteria].iter().any(|known|known.eq_ignore_ascii_case(label)) {return Err(Error::Weights);}
        labels[criteria]=label;weights[criteria]=scalar(window[2],100).map_err(|_|Error::Weights)?;criteria+=1;
    }
    if criteria==0 {return Ok(None);}
    if weights[..criteria].iter().map(|v|*v as u32).sum::<u32>()!=100 {return Err(Error::Weights);}
    let mut result=Ranking {scores:[0;MAX_OPTIONS],count:0,eligible:0,best:0};
    let mut seen=0u8;let mut position=0;
    while position+1<tokens.len() {
        if !tokens[position].eq_ignore_ascii_case(b"option") {position+=1;continue;}
        let definition=separators[position+1]==1 || (separators[position+1]==0 && tokens.get(position+2)
            .is_some_and(|token|labels[..criteria].iter().any(|label|label.eq_ignore_ascii_case(token))));
        if !definition {position+=1;continue;}
        let number=scalar(tokens[position+1],MAX_OPTIONS as u16).map_err(|_|Error::Scores)?;
        if number==0 {return Err(Error::Scores);}
        let index=number as usize-1;
        if seen&(1<<index)!=0 {return Err(Error::Scores);}
        let mut values=[0u16;MAX_CRITERIA];let mut fields=0u8;position+=2;
        while position+1<tokens.len() && fields.count_ones()<criteria as u32 {
            if tokens[position].eq_ignore_ascii_case(b"option") {break;}
            if let Some(column)=labels[..criteria].iter().position(|label|label.eq_ignore_ascii_case(tokens[position])) {
                if fields&(1<<column)!=0 {return Err(Error::Scores);}
                values[column]=scalar(tokens[position+1],1000).map_err(|_|Error::Scores)?;
                fields|=1<<column;position+=2;
            } else {position+=1;}
        }
        if fields.count_ones()!=criteria as u32 {return Err(Error::Scores);}
        result.scores[index]=(0..criteria).map(|column|weights[column] as u32*values[column] as u32).sum();
        seen|=1<<index;result.count=result.count.max(index+1);
    }
    if result.count<2||seen.count_ones()!=result.count as u32 {return Err(Error::Scores);}
    result.eligible=seen;
    let highest=result.scores[..result.count].iter().copied().max().ok_or(Error::Scores)?;
    for index in 0..result.count {if result.scores[index]==highest {result.best|=1<<index;}}
    Ok(Some(result))
}
