pub const AI_NAME_CAPACITY: usize = 32;
pub const AI_FACT_CAPACITY: usize = 6;
pub const AI_FACT_TEXT_CAPACITY: usize = 96;
pub const AI_MEMORY_SLOT_BYTES: usize = 640;
pub const AI_MEMORY_STATE_BYTES: usize = 16 + AI_MEMORY_SLOT_BYTES * 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryResponseKind {
    NameStored,
    FactStored,
    NameRecalled,
    FactsSummarized,
    FactRecalled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryFact {
    bytes: [u8; AI_FACT_TEXT_CAPACITY],
    length: u8,
}

impl MemoryFact {
    // ------------------------=
    // FUNC: empty
    // DESC: Creates one unused bounded semantic-memory fact.
    // ------------------=
    pub const fn empty() -> Self {
        Self {
            bytes: [0; AI_FACT_TEXT_CAPACITY],
            length: 0,
        }
    }

    // ------------------------=
    // FUNC: text
    // DESC: Returns the initialized fact bytes.
    // ------------------=
    pub fn text(&self) -> &[u8] {
        &self.bytes[..self.length as usize]
    }

    // ------------------------=
    // FUNC: set
    // DESC: Replaces this fact with validated printable text.
    // ------------------=
    fn set(&mut self, text: &[u8]) -> bool {
        let text = trim_sentence(text);
        if text.is_empty()
            || text.len() > AI_FACT_TEXT_CAPACITY
            || text.iter().any(|byte| !(b' '..=b'~').contains(byte))
        {
            return false;
        }
        self.bytes.fill(0);
        self.bytes[..text.len()].copy_from_slice(text);
        self.length = text.len() as u8;
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AiMemory {
    name: [u8; AI_NAME_CAPACITY],
    name_length: u8,
    facts: [MemoryFact; AI_FACT_CAPACITY],
    fact_count: u8,
}

impl AiMemory {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a bounded assistant memory with the default Infinity identity.
    // ------------------=
    pub const fn new() -> Self {
        let mut name = [0; AI_NAME_CAPACITY];
        name[0] = b'I';
        name[1] = b'n';
        name[2] = b'f';
        name[3] = b'i';
        name[4] = b'n';
        name[5] = b'i';
        name[6] = b't';
        name[7] = b'y';
        Self {
            name,
            name_length: 8,
            facts: [MemoryFact::empty(); AI_FACT_CAPACITY],
            fact_count: 0,
        }
    }

    // ------------------------=
    // FUNC: name
    // DESC: Returns the durable user-assigned assistant name.
    // ------------------=
    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_length as usize]
    }

    // ------------------------=
    // FUNC: fact_count
    // DESC: Reports the number of retained important facts.
    // ------------------=
    pub const fn fact_count(&self) -> usize {
        self.fact_count as usize
    }

    // ------------------------=
    // FUNC: fact
    // DESC: Returns one retained fact by chronological index.
    // ------------------=
    pub fn fact(&self, index: usize) -> Option<&[u8]> {
        (index < self.fact_count()).then(|| self.facts[index].text())
    }

    // ------------------------=
    // FUNC: respond
    // DESC: Applies memory commands or answers recall questions into a bounded response.
    // ------------------=
    pub fn respond(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> Option<(usize, bool, MemoryResponseKind)> {
        let input = trim_ascii(input);
        for prefix in [
            b"set your name to ".as_slice(),
            b"your name is ".as_slice(),
            b"call yourself ".as_slice(),
        ] {
            if let Some(value) = strip_prefix_case(input, prefix) {
                let value = trim_sentence(value);
                if self.set_name(value) {
                    let length = write_parts(
                        output,
                        &[
                            b"Understood. My name is ",
                            self.name(),
                            b". I'll remember that.",
                        ],
                    );
                    return Some((length, true, MemoryResponseKind::NameStored));
                }
            }
        }
        if let Some(fact) = strip_prefix_case(input, b"remember that ") {
            let fact = trim_sentence(fact);
            if self.remember(fact) {
                let length = write_parts(output, &[b"I'll remember: ", fact, b"."]);
                return Some((length, true, MemoryResponseKind::FactStored));
            }
        }
        if contains_case(input, b"what is your name") || contains_case(input, b"who are you") {
            let length = write_parts(
                output,
                &[
                    b"My name is ",
                    self.name(),
                    b". I'm your local InfinityOS assistant.",
                ],
            );
            return Some((length, false, MemoryResponseKind::NameRecalled));
        }
        if contains_case(input, b"what do you remember")
            || contains_case(input, b"tell me what you remember")
        {
            if self.fact_count == 0 {
                return Some((
                    write_parts(output, &[b"I don't have any saved facts for you yet."]),
                    false,
                    MemoryResponseKind::FactsSummarized,
                ));
            }
            let mut length = write_parts(output, &[b"I remember: "]);
            for index in 0..self.fact_count() {
                if index != 0 {
                    length += write_at(output, length, b"; ");
                }
                length += write_at(output, length, self.facts[index].text());
            }
            return Some((length, false, MemoryResponseKind::FactsSummarized));
        }
        if input.ends_with(b"?") {
            if let Some(fact) = self.best_recalled_fact(input) {
                let length = write_parts(output, &[b"You told me: ", fact, b"."]);
                return Some((length, false, MemoryResponseKind::FactRecalled));
            }
        }
        None
    }

    // ------------------------=
    // FUNC: encode_into
    // DESC: Serializes memory into one fixed architecture-neutral user slot.
    // ------------------=
    pub fn encode_into(&self, output: &mut [u8]) -> bool {
        if output.len() != AI_MEMORY_SLOT_BYTES {
            return false;
        }
        output.fill(0);
        output[0] = self.name_length;
        output[1] = self.fact_count;
        output[2..2 + AI_NAME_CAPACITY].copy_from_slice(&self.name);
        for index in 0..AI_FACT_CAPACITY {
            let at = 40 + index * (AI_FACT_TEXT_CAPACITY + 1);
            output[at] = self.facts[index].length;
            output[at + 1..at + 1 + AI_FACT_TEXT_CAPACITY]
                .copy_from_slice(&self.facts[index].bytes);
        }
        true
    }

    // ------------------------=
    // FUNC: decode_from
    // DESC: Validates and restores one fixed user memory slot.
    // ------------------=
    pub fn decode_from(input: &[u8]) -> Option<Self> {
        if input.len() != AI_MEMORY_SLOT_BYTES
            || input[0] as usize > AI_NAME_CAPACITY
            || input[1] as usize > AI_FACT_CAPACITY
        {
            return None;
        }
        let mut memory = Self::new();
        memory.name.fill(0);
        memory.name.copy_from_slice(&input[2..2 + AI_NAME_CAPACITY]);
        memory.name_length = input[0];
        memory.fact_count = input[1];
        if memory.name_length == 0
            || memory
                .name()
                .iter()
                .any(|byte| !(b' '..=b'~').contains(byte))
        {
            return None;
        }
        memory.facts = [MemoryFact::empty(); AI_FACT_CAPACITY];
        for index in 0..AI_FACT_CAPACITY {
            let at = 40 + index * (AI_FACT_TEXT_CAPACITY + 1);
            let length = input[at] as usize;
            if length > AI_FACT_TEXT_CAPACITY
                || (index < memory.fact_count()
                    && (length == 0
                        || input[at + 1..at + 1 + length]
                            .iter()
                            .any(|byte| !(b' '..=b'~').contains(byte))))
            {
                return None;
            }
            memory.facts[index]
                .bytes
                .copy_from_slice(&input[at + 1..at + 1 + AI_FACT_TEXT_CAPACITY]);
            memory.facts[index].length = length as u8;
        }
        Some(memory)
    }

    // ------------------------=
    // FUNC: set_name
    // DESC: Stores a validated printable assistant name.
    // ------------------=
    fn set_name(&mut self, value: &[u8]) -> bool {
        if value.is_empty()
            || value.len() > AI_NAME_CAPACITY
            || value.iter().any(|byte| !(b' '..=b'~').contains(byte))
        {
            return false;
        }
        self.name.fill(0);
        self.name[..value.len()].copy_from_slice(value);
        self.name_length = value.len() as u8;
        true
    }

    // ------------------------=
    // FUNC: remember
    // DESC: Retains one fact, de-duplicating it and evicting the oldest fact at capacity.
    // ------------------=
    fn remember(&mut self, fact: &[u8]) -> bool {
        if self.facts[..self.fact_count()]
            .iter()
            .any(|stored| equals_case(stored.text(), fact))
        {
            return true;
        }
        if self.fact_count() == AI_FACT_CAPACITY {
            self.facts.copy_within(1..AI_FACT_CAPACITY, 0);
            self.fact_count -= 1;
        }
        if !self.facts[self.fact_count()].set(fact) {
            return false;
        }
        self.fact_count += 1;
        true
    }

    // ------------------------=
    // FUNC: best_recalled_fact
    // DESC: Selects a saved fact sharing at least two meaningful words with a question.
    // ------------------=
    fn best_recalled_fact(&self, question: &[u8]) -> Option<&[u8]> {
        let mut best = None;
        let mut best_score = 1usize;
        for fact in &self.facts[..self.fact_count()] {
            let score = meaningful_word_overlap(question, fact.text());
            if score > best_score {
                best_score = score;
                best = Some(fact.text());
            }
        }
        best
    }
}

// ------------------------=
// FUNC: meaningful_word_overlap
// DESC: Counts distinct-enough content-word matches between a question and one fact.
// ------------------=
fn meaningful_word_overlap(left: &[u8], right: &[u8]) -> usize {
    let mut score = 0;
    for word in left.split(|byte| !byte.is_ascii_alphanumeric()) {
        if word.len() < 3 || is_stop_word(word) {
            continue;
        }
        if right
            .split(|byte| !byte.is_ascii_alphanumeric())
            .any(|candidate| equals_case(word, candidate))
        {
            score += 1;
        }
    }
    score
}

// ------------------------=
// FUNC: is_stop_word
// DESC: Excludes common grammar words from semantic recall matching.
// ------------------=
fn is_stop_word(word: &[u8]) -> bool {
    [
        b"what".as_slice(),
        b"when",
        b"where",
        b"which",
        b"that",
        b"this",
        b"your",
        b"you",
        b"the",
        b"and",
        b"does",
        b"remember",
    ]
    .iter()
    .any(|candidate| equals_case(word, candidate))
}

// ------------------------=
// FUNC: trim_ascii
// DESC: Removes surrounding ASCII whitespace.
// ------------------=
fn trim_ascii(input: &[u8]) -> &[u8] {
    let start = input
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(input.len());
    let end = input
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map(|index| index + 1)
        .unwrap_or(start);
    &input[start..end]
}

// ------------------------=
// FUNC: trim_sentence
// DESC: Removes surrounding whitespace and trailing sentence punctuation from a stored value.
// ------------------=
fn trim_sentence(input: &[u8]) -> &[u8] {
    let input = trim_ascii(input);
    input
        .strip_suffix(b".")
        .or_else(|| input.strip_suffix(b"!"))
        .unwrap_or(input)
}

// ------------------------=
// FUNC: equals_case
// DESC: Compares two ASCII byte strings without case sensitivity.
// ------------------=
fn equals_case(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
}

// ------------------------=
// FUNC: strip_prefix_case
// DESC: Removes an ASCII prefix without case sensitivity.
// ------------------=
fn strip_prefix_case<'a>(input: &'a [u8], prefix: &[u8]) -> Option<&'a [u8]> {
    (input.len() >= prefix.len() && equals_case(&input[..prefix.len()], prefix))
        .then(|| &input[prefix.len()..])
}

// ------------------------=
// FUNC: contains_case
// DESC: Finds one ASCII phrase without allocating.
// ------------------=
fn contains_case(input: &[u8], needle: &[u8]) -> bool {
    input
        .windows(needle.len())
        .any(|window| equals_case(window, needle))
}

// ------------------------=
// FUNC: write_at
// DESC: Copies as much text as fits at one bounded output position.
// ------------------=
fn write_at(output: &mut [u8], at: usize, value: &[u8]) -> usize {
    let length = value.len().min(output.len().saturating_sub(at));
    output[at..at + length].copy_from_slice(&value[..length]);
    length
}

// ------------------------=
// FUNC: write_parts
// DESC: Joins response fragments into a bounded caller-owned output buffer.
// ------------------=
fn write_parts(output: &mut [u8], parts: &[&[u8]]) -> usize {
    output.fill(0);
    let mut length = 0;
    for part in parts {
        length += write_at(output, length, part);
    }
    length
}
