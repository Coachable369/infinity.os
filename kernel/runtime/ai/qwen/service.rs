//! One local generation job, owned by the AI service and advanced cooperatively.
use super::{
    gguf::{Error, Model},
    tokenizer::{Entry, Tokenizer},
    transformer::{Engine, Progress, Weights, CONTEXT, KV_FLOATS, WORK_FLOATS},
};
pub struct Service {
    ministral: bool,
    hermes: bool,
    engine: Engine<'static, 'static>,
    tokenizer: Tokenizer<'static, 'static>,
    prompt: [u32; CONTEXT],
    prompt_count: usize,
    consumed: usize,
    output: [u8; 16384],
    output_count: usize,
    generated: usize,
    busy: bool,
    pub reused_tokens: usize,
    pub prefill_tokens: usize,
    pub allocated_bytes: usize,
    worker_start: [u64; 3],
}
impl Service {
    // ------------------------=
    // FUNC: load
    // DESC: Verifies the pinned artifact before binding an exclusively owned boot-reserved arena.
    // ------------------=
    pub fn load(bytes: &'static [u8], arena: &'static mut [u8]) -> Result<Self, Error> {
        Self::load_checked(bytes, arena, false)
    }
    // ------------------------=
    // FUNC: load_hermes
    // DESC: Loads Hermes through a separate pinned-artifact entry point, never weakening Qwen validation.
    // ------------------=
    pub fn load_hermes(bytes: &'static [u8], arena: &'static mut [u8]) -> Result<Self, Error> {
        Self::load_checked(bytes, arena, true)
    }
    // ------------------------=
    // FUNC: load_checked
    // DESC: Binds isolated persistent model memory after exact artifact validation.
    // ------------------=
    fn load_checked(
        bytes: &'static [u8],
        arena: &'static mut [u8],
        hermes: bool,
    ) -> Result<Self, Error> {
        use sha2::{Digest, Sha256};
        const HASH: [u8; 32] = [
            0xd9, 0x8c, 0xdc, 0xbd, 0x03, 0xe1, 0x7c, 0xe4, 0x76, 0x81, 0x43, 0x5b, 0x51, 0x50,
            0xe3, 0x4c, 0x14, 0x17, 0xf5, 0x0b, 0x5c, 0x00, 0x19, 0xdd, 0x56, 0x0e, 0x48, 0x82,
            0xc5, 0x74, 0x57, 0x85,
        ];
        let ministral = bytes.len() == 2_147_023_008;
        let expected = if hermes {
            [
                0x91, 0x77, 0x6f, 0xe0, 0xf6, 0xcd, 0x74, 0x83, 0xd9, 0xd5, 0xe0, 0x61, 0x62, 0xfd,
                0xd1, 0xf8, 0xf0, 0x26, 0x2c, 0x15, 0xce, 0xd2, 0x69, 0x79, 0x1b, 0x4d, 0x96, 0xa6,
                0x55, 0xe8, 0xa5, 0xa2,
            ]
        } else if ministral {
            [
                0x9e, 0xd1, 0x50, 0xd4, 0x36, 0x7e, 0x68, 0xdf, 0x0a, 0xc8, 0xe1, 0x54, 0x0f, 0x6d,
                0xdc, 0x65, 0xb4, 0x2d, 0x0e, 0xe2, 0x63, 0x78, 0x32, 0x9d, 0x1e, 0xcb, 0xca, 0x60,
                0xf9, 0x3f, 0xc5, 0xf8,
            ]
        } else {
            HASH
        };
        if (hermes && bytes.len() != 2_019_373_888)
            || (!hermes && !ministral && bytes.len() != 5_027_783_488)
            || Sha256::digest(bytes).as_slice() != expected
        {
            return Err(Error::Format);
        }
        let model = Model::parse(bytes)?;
        let weights = if hermes {
            Weights::load_hermes(model)?
        } else {
            Weights::load(model)?
        };
        let count = |key| -> Result<usize, Error> {
            let (kind, mut values) = model.metadata(key)?;
            if kind != 9 || values.u32()? != 8 {
                return Err(Error::Format);
            }
            usize::try_from(values.u64()?).map_err(|_| Error::Overflow)
        };
        let tokens = count(b"tokenizer.ggml.tokens")?;
        let merges = count(b"tokenizer.ggml.merges")?;
        let floats = KV_FLOATS + WORK_FLOATS;
        let required = floats
            .checked_mul(4)
            .and_then(|n| n.checked_add((tokens + merges) * core::mem::size_of::<Entry>()))
            .ok_or(Error::Overflow)?;
        if arena.len() < required || arena.as_ptr() as usize % 16 != 0 {
            return Err(Error::Format);
        }
        // Initialization occurs once at boot, never in generation or paint paths.
        arena[..floats * 4].fill(0);
        let pointer = arena.as_mut_ptr();
        // SAFETY: arena is exclusively owned for the service lifetime; the checked
        // regions are aligned, disjoint, and remain inside this boot allocation.
        let kv = unsafe { core::slice::from_raw_parts_mut(pointer.cast::<f32>(), KV_FLOATS) };
        let work = unsafe {
            core::slice::from_raw_parts_mut(pointer.add(KV_FLOATS * 4).cast::<f32>(), WORK_FLOATS)
        };
        let entries = unsafe { pointer.add(floats * 4).cast::<Entry<'static>>() };
        for i in 0..tokens + merges {
            unsafe {
                entries.add(i).write(super::tokenizer::EMPTY);
            }
        }
        let vocabulary = unsafe { core::slice::from_raw_parts_mut(entries, tokens) };
        let merge_index = unsafe { core::slice::from_raw_parts_mut(entries.add(tokens), merges) };
        let tokenizer = Tokenizer::load(model, vocabulary, merge_index)?;
        let engine = Engine::new(weights, kv, work)?;
        Ok(Self {
            ministral,
            hermes,
            engine,
            tokenizer,
            prompt: [0; CONTEXT],
            prompt_count: 0,
            consumed: 0,
            output: [0; 16384],
            output_count: 0,
            generated: 0,
            busy: false,
            reused_tokens: 0,
            prefill_tokens: 0,
            allocated_bytes: bytes.len() + arena.len() + core::mem::size_of::<Self>(),
            worker_start: [0; 3],
        })
    }
    // ------------------------=
    // FUNC: submit
    // DESC: Formats an explicit non-thinking user turn and schedules native generation.
    // ------------------=
    pub fn submit(&mut self, text: &[u8]) -> Result<(), Error> {
        if self.busy || text.is_empty() {
            return Err(Error::Unsupported);
        }
        let text = core::str::from_utf8(text).map_err(|_| Error::Format)?;
        self.engine.profile = super::metrics::Profile::new();
        self.worker_start = super::workers::kernel_profile_ns();
        let previous = self.prompt_count;
        let result = self.append_turn(text);
        if let Err(error) = result {
            self.prompt_count = previous;
            return Err(error);
        }
        // Completed positions are an immutable prefix of the retained token history.
        // An interrupted token may have partially written KV rows; begin overwrites
        // that position across every layer before it becomes visible to attention.
        self.consumed = self.engine.resume_prefix();
        self.reused_tokens = self.consumed;
        self.prefill_tokens = self.prompt_count - self.consumed;
        self.output_count = 0;
        self.generated = 0;
        self.engine.begin_with_prediction(
            self.prompt[self.consumed],
            self.consumed + 1 == self.prompt_count,
        )?;
        self.consumed += 1;
        self.busy = true;
        Ok(())
    }
    // ------------------------=
    // FUNC: profile
    // DESC: Returns request-local BSP phase and AP kernel timings without allocation.
    // ------------------=
    pub fn profile(&self) -> (super::metrics::Profile, [u64; 3]) {
        let now = super::workers::kernel_profile_ns();
        (self.engine.profile, core::array::from_fn(|i| now[i].saturating_sub(self.worker_start[i])))
    }
    // ------------------------=
    // FUNC: append_turn
    // DESC: Retains prior turns as tokens and bounds the complete conversation to the context window.
    // ------------------=
    fn append_turn(&mut self, text: &str) -> Result<(), Error> {
        if self.ministral {
            if self.prompt_count == 0 {
                self.control(b"<s>")?;
            } else {
                self.control(b"</s>")?;
            }
            self.control(b"[INST]")?;
            self.text(text)?;
            self.control(b"[/INST]")?;
            if self.prompt_count >= CONTEXT - 1 {
                return Err(Error::Overflow);
            }
            return Ok(());
        }
        if self.prompt_count != 0 {
            self.control(b"<|im_end|>")?;
            self.text("\n")?;
        }
        self.control(b"<|im_start|>")?;
        self.text("user\n")?;
        self.text(text)?;
        self.control(b"<|im_end|>")?;
        self.text("\n")?;
        self.control(b"<|im_start|>")?;
        self.text("assistant\n")?;
        if self.hermes {
            return if self.prompt_count < CONTEXT - 1 {
                Ok(())
            } else {
                Err(Error::Overflow)
            };
        }
        self.control(b"<think>")?;
        self.text("\n\n")?;
        self.control(b"</think>")?;
        self.text("\n\n")?;
        if self.prompt_count >= CONTEXT - 1 {
            return Err(Error::Overflow);
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: control
    // DESC: Appends a trusted chat-template control token.
    // ------------------=
    fn control(&mut self, value: &[u8]) -> Result<(), Error> {
        let token = self.tokenizer.special(value)?;
        *self
            .prompt
            .get_mut(self.prompt_count)
            .ok_or(Error::Overflow)? = token;
        self.prompt_count += 1;
        Ok(())
    }
    // ------------------------=
    // FUNC: text
    // DESC: Appends tokenized ordinary template or user text.
    // ------------------=
    fn text(&mut self, value: &str) -> Result<(), Error> {
        self.prompt_count += self
            .tokenizer
            .encode(value, &mut self.prompt[self.prompt_count..])?;
        Ok(())
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Stops the active job without emitting a fabricated response.
    // ------------------=
    pub fn cancel(&mut self) {
        self.engine.cancel();
        self.busy = false;
    }
    // ------------------------=
    // FUNC: clear_conversation
    // DESC: Discards conversation tokens and output when the authenticated principal changes.
    // ------------------=
    pub fn clear_conversation(&mut self) {
        self.cancel();
        self.prompt.fill(0);
        self.prompt_count = 0;
        self.output.fill(0);
        self.output_count = 0;
        self.engine.reset();
    }
    // ------------------------=
    // FUNC: busy
    // DESC: Reports whether bounded inference work remains queued.
    // ------------------=
    pub fn busy(&self) -> bool {
        self.busy
    }
    // ------------------------=
    // FUNC: output
    // DESC: Exposes only bytes produced by model-selected tokens.
    // ------------------=
    pub fn output(&self) -> &[u8] {
        &self.output[..self.output_count]
    }
    // ------------------------=
    // FUNC: poll
    // DESC: Advances one bounded engine slice and returns whether visible output changed.
    // ------------------=
    pub fn poll(&mut self) -> Result<bool, Error> {
        if !self.busy {
            return Ok(false);
        }
        let next = match self.engine.step()? {
            Progress::Token(next) => next,
            Progress::Prefilled => {
                self.engine.begin_with_prediction(
                    self.prompt[self.consumed],
                    self.consumed + 1 == self.prompt_count,
                )?;
                self.consumed += 1;
                return Ok(false);
            }
            _ => return Ok(false),
        };
        if self.consumed < self.prompt_count {
            self.engine.begin(self.prompt[self.consumed])?;
            self.consumed += 1;
            return Ok(false);
        }
        if (self.ministral && next == 2)
            || (self.hermes && next == 128039)
            || (!self.hermes && !self.ministral && (next == 151645 || next == 151643))
            || self.prompt_count >= CONTEXT
        {
            self.busy = false;
            return Ok(false);
        }
        let mut piece = [0u8; 1024];
        let length = self.tokenizer.decode(next, &mut piece)?;
        if length > self.output.len() - self.output_count {
            self.busy = false;
            return Ok(false);
        }
        self.output[self.output_count..self.output_count + length]
            .copy_from_slice(&piece[..length]);
        self.output_count += length;
        self.generated += 1;
        self.prompt[self.prompt_count] = next;
        self.prompt_count += 1;
        self.consumed = self.prompt_count;
        if self.engine.begin(next)? == Progress::ContextFull {
            self.busy = false;
        }
        Ok(true)
    }
}
