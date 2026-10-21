//! The music player of the original (CODE 10), a NoiseTracker-style
//! ProTracker replay with a software mixer: one call mixes one tick of the
//! module into [`TICK_SAMPLES`] 8-bit samples at [`RATE`] Hz. Ported routine
//! by routine, including its quirks, and compared byte by byte with the
//! original running in an emulator (`re/tools/emu_music.py`).
//!
//! The player's tables (volumes, steps, vibrato, periods) are read from the
//! original code, which the caller passes in (CODE 10 without its header).

/// Samples per tick (one call of CODE 10 0x0).
pub const TICK_SAMPLES: usize = 279;
/// Output rate: 7/11 of the Mac's 22254.54 Hz (CODE 10 0x5ece).
pub const RATE: f64 = 14161.98;

/// Offsets of the tables in CODE 10.
const VIBRATO: usize = 0x3E;
const PERIODS: usize = 0x5E;
const STEPS: usize = 0x9D8;
const VOLUMES: usize = 0x1740;
const VOLUMES_END: usize = 0x5740;
/// Sample names get this mark once their data is converted (CODE 10 0x5804).
const CONVERTED: [u8; 4] = [0, b'A', b'R', b'E'];

#[derive(Clone, Copy, Debug, Default)]
struct Channel {
    /// The note of the row: period and sample, command, parameter.
    note: [u8; 4],
    start: Option<usize>,
    /// Length in words.
    length: u16,
    /// Volume word (with the finetune byte above it, as the original).
    volume: u16,
    loop_ptr: Option<usize>,
    /// Loop length in words.
    loop_len: u16,
    period: u16,
    tone_dir: u8,
    tone_speed: u8,
    tone_target: u16,
    vibrato: u8,
    vibrato_pos: u8,
}

#[derive(Clone, Copy, Debug, Default)]
struct Voice {
    ptr: Option<usize>,
    /// Bytes left to play; the samples are stored reversed.
    pos: u32,
    volume: u16,
    period: u16,
    loop_ptr: Option<usize>,
    loop_len: u32,
}

/// The state of CODE 10 (kept inside the segment in the original).
#[derive(Clone, Debug)]
pub struct MusicPlayer {
    /// The module with its samples converted (reversed, unsigned).
    module: Vec<u8>,
    /// CODE 10 with the volume table divided by 4 (CODE 10 0x5850).
    code: Vec<u8>,
    order: usize,
    patterns: usize,
    samples: [Option<usize>; 32],
    speed: u8,
    position: u8,
    row: u16,
    counter: u8,
    jump: bool,
    start: u8,
    channels: [Channel; 4],
    voices: [Voice; 4],
}

fn word(data: &[u8], at: usize) -> u16 {
    match data.get(at..at + 2) {
        Some(b) => u16::from_be_bytes([b[0], b[1]]),
        None => 0,
    }
}

impl MusicPlayer {
    /// Start the module at an order position (CODE 10 0x5740). The game
    /// starts at position 1 when it launches and at 0 when the music is
    /// switched on again; the song loops back to that position.
    pub fn new(module: &[u8], code: &[u8], start: u8) -> Self {
        let mut module = module.to_vec();
        // room for reads past the end, which are 0 in the emulator
        let mut code = code.to_vec();
        if code.len() < VOLUMES_END {
            code.resize(VOLUMES_END, 0);
        }
        for b in &mut code[VOLUMES..VOLUMES_END] {
            *b >>= 2;
        }
        let mk = module.get(0x438..0x43C) == Some(b"M.K.");
        let (order, patterns, count) = if mk {
            (0x3B8, 0x43C, 31)
        } else {
            (0x1D8, 0x258, 15)
        };
        // highest pattern, scanned like the original (CODE 10 0x5780)
        let mut d0: u16 = 0x7F;
        let mut d1: u8;
        let mut d2: u8 = 0;
        let mut at = order;
        d0 = d0.wrapping_sub(1);
        loop {
            d1 = *module.get(at).unwrap_or(&0);
            at += 1;
            if (d1 as i8) > (d2 as i8) {
                d2 = d1;
                d0 = d0.wrapping_sub(1);
                continue;
            }
            d0 = d0.wrapping_sub(1);
            if d0 == 0xFFFF {
                break;
            }
        }
        let mut ptr = patterns + ((d2.wrapping_add(1) as usize) << 10);
        let mut samples = [None; 32];
        for (i, entry) in (0..count).map(|i| (i, 0x14 + 30 * i)) {
            samples[i] = Some(ptr);
            let length = word(&module, entry + 0x16) as usize * 2;
            let end = ptr + length;
            samples[i + 1] = Some(end);
            if module.get(entry..entry + 4) != Some(&CONVERTED[..])
                && length > 0
                && let Some(s) = module.get_mut(ptr..end)
            {
                s.reverse();
                for b in s {
                    *b ^= 0x80;
                }
            }
            if word(&module, entry + 0x1C) == 1 && entry + 0x1E <= module.len() {
                module[entry + 0x1C..entry + 0x1E].copy_from_slice(&[0, 0]);
            }
            if entry + 4 <= module.len() {
                module[entry..entry + 4].copy_from_slice(&CONVERTED);
            }
            ptr = end;
        }
        Self {
            module,
            code,
            order,
            patterns,
            samples,
            speed: 6,
            position: start,
            row: 0,
            counter: 0,
            jump: false,
            start,
            channels: [Channel::default(); 4],
            voices: [Voice::default(); 4],
        }
    }

    /// One tick: play the row or the effects, then mix (CODE 10 0x5872).
    pub fn tick(&mut self) -> [u8; TICK_SAMPLES] {
        self.replay();
        self.mix()
    }

    /// CODE 10 0x587a.
    fn replay(&mut self) {
        self.counter = self.counter.wrapping_add(1);
        if (self.counter as i8) < (self.speed as i8) {
            for ch in 0..4 {
                self.effects(ch);
            }
        } else {
            self.counter = 0;
            self.new_row();
        }
        while self.jump {
            self.next_position();
        }
    }

    fn next_position(&mut self) {
        self.row = 0;
        self.jump = false;
        self.position = self.position.wrapping_add(1) & 0x7F;
        if self.order >= 2 && Some(&self.position) == self.module.get(self.order - 2) {
            self.position = self.start;
        }
    }

    /// CODE 10 0x58c8.
    fn new_row(&mut self) {
        let pattern = *self
            .module
            .get(self.order + self.position as usize)
            .unwrap_or(&0) as u32;
        let mut at = (pattern << 10) as u16 as u32 | ((pattern << 10) & 0xFFFF_0000);
        at = (at & 0xFFFF_0000) | ((at as u16).wrapping_add(self.row)) as u32;
        for ch in 0..4 {
            let base = self.patterns + at as usize;
            for k in 0..4 {
                self.channels[ch].note[k] = *self.module.get(base + k).unwrap_or(&0);
            }
            at = at.wrapping_add(4);
            self.play_voice(ch);
        }
        for ch in 0..4 {
            let c = self.channels[ch];
            self.voices[ch].loop_ptr = c.loop_ptr;
            self.voices[ch].loop_len = c.loop_len.wrapping_mul(2) as u32;
        }
        self.row = self.row.wrapping_add(0x10);
        if self.row == 0x400 {
            self.next_position();
        }
    }

    /// CODE 10 0x591a.
    fn play_voice(&mut self, ch: usize) {
        let note = self.channels[ch].note;
        let sample = ((note[2] & 0xF0) >> 4) | (note[0] & 0xF0);
        if sample != 0 {
            let entry = 0xC + sample as usize * 30;
            let c = &mut self.channels[ch];
            c.start = self.samples[sample as usize - 1];
            c.length = word(&self.module, entry);
            c.volume = word(&self.module, entry + 2);
            c.loop_ptr = c.start;
            c.loop_len = word(&self.module, entry + 6);
            self.voices[ch].volume = c.volume;
        }
        let period = u16::from_be_bytes([note[0], note[1]]) & 0xFFF;
        if period != 0 {
            if note[2] & 0xF == 3 {
                self.setup_tone_portamento(ch);
            } else {
                let c = &mut self.channels[ch];
                c.period = period;
                c.vibrato_pos = 0;
                let v = &mut self.voices[ch];
                v.ptr = c.start;
                v.pos = c.length.wrapping_mul(2) as u32;
                v.period = c.period;
            }
        }
        self.row_command(ch);
    }

    /// Commands that act once per row (CODE 10 0x5ca2).
    fn row_command(&mut self, ch: usize) {
        let note = self.channels[ch].note;
        let param = note[3];
        match note[2] & 0xF {
            0xB => {
                self.position = param.wrapping_sub(1);
                self.jump = !self.jump;
            }
            0xC => {
                if (param as i8) > 0x40 {
                    self.channels[ch].note[3] = 0x40;
                }
                let v = self.channels[ch].note[3] as u16;
                self.voices[ch].volume = (self.voices[ch].volume & 0xFF00) | v;
            }
            0xD => self.jump = !self.jump,
            0xF => {
                let speed = param & 0x1F;
                if speed != 0 {
                    self.counter = 0;
                    self.speed = speed;
                }
            }
            _ => {}
        }
    }

    /// Commands between rows (CODE 10 0x5a8e).
    fn effects(&mut self, ch: usize) {
        let note = self.channels[ch].note;
        if u16::from_be_bytes([note[2], note[3]]) & 0xFFF == 0 {
            return;
        }
        match note[2] & 0xF {
            0 => self.arpeggio(ch),
            1 => {
                let c = &mut self.channels[ch];
                c.period = c.period.wrapping_sub(note[3] as u16);
                if (c.period as i16).wrapping_sub(0x71) < 0 {
                    c.period = 0x71;
                }
                self.voices[ch].period = c.period;
            }
            2 => {
                let c = &mut self.channels[ch];
                c.period = c.period.wrapping_add(note[3] as u16);
                if (c.period as i16).wrapping_sub(0x358) >= 0 {
                    c.period = 0x358;
                }
                self.voices[ch].period = c.period;
            }
            3 => {
                if note[3] != 0 {
                    self.channels[ch].tone_speed = note[3];
                    self.channels[ch].note[3] = 0;
                }
                self.tone_portamento(ch);
            }
            4 => {
                if note[3] != 0 {
                    self.channels[ch].vibrato = note[3];
                }
                self.vibrato(ch);
            }
            cmd => {
                self.voices[ch].period = self.channels[ch].period;
                match cmd {
                    0xA => self.volume_slide(ch),
                    5 => {
                        self.tone_portamento(ch);
                        self.volume_slide(ch);
                    }
                    6 => {
                        self.vibrato(ch);
                        self.volume_slide(ch);
                    }
                    _ => {}
                }
            }
        }
    }

    /// CODE 10 0x5aee.
    fn arpeggio(&mut self, ch: usize) {
        let c = self.channels[ch];
        let nibble = match self.counter % 3 {
            0 => {
                self.voices[ch].period = c.period;
                return;
            }
            2 => c.note[3] & 0xF,
            _ => c.note[3] >> 4,
        } as usize;
        for i in 0..37 {
            let at = PERIODS + 2 * i;
            if (c.period as i16) >= (word(&self.code, at) as i16) {
                self.voices[ch].period = word(&self.code, at + 2 * nibble);
                return;
            }
        }
    }

    /// CODE 10 0x5b8c.
    fn setup_tone_portamento(&mut self, ch: usize) {
        let c = &mut self.channels[ch];
        let target = u16::from_be_bytes([c.note[0], c.note[1]]) & 0xFFF;
        c.tone_target = target;
        c.tone_dir = 0;
        if target == c.period {
            c.tone_target = 0;
        } else if (target as i16) < (c.period as i16) {
            c.tone_dir = 1;
        }
    }

    /// CODE 10 0x5bc0.
    fn tone_portamento(&mut self, ch: usize) {
        let c = &mut self.channels[ch];
        if c.tone_target == 0 {
            return;
        }
        let speed = c.tone_speed as u16;
        if c.tone_dir == 0 {
            c.period = c.period.wrapping_add(speed);
            if (c.tone_target as i16) <= (c.period as i16) {
                c.period = c.tone_target;
                c.tone_target = 0;
            }
        } else {
            c.period = c.period.wrapping_sub(speed);
            if (c.tone_target as i16) >= (c.period as i16) {
                c.period = c.tone_target;
                c.tone_target = 0;
            }
        }
        self.voices[ch].period = c.period;
    }

    /// CODE 10 0x5c1c.
    fn vibrato(&mut self, ch: usize) {
        let c = &mut self.channels[ch];
        let index = ((c.vibrato_pos >> 2) & 0x1F) as usize;
        let sine = *self.code.get(VIBRATO + index).unwrap_or(&0) as u16;
        let delta = (sine * (c.vibrato & 0xF) as u16) >> 6;
        self.voices[ch].period = if (c.vibrato_pos as i8) < 0 {
            c.period.wrapping_sub(delta)
        } else {
            c.period.wrapping_add(delta)
        };
        c.vibrato_pos = c.vibrato_pos.wrapping_add((c.vibrato >> 2) & 0x3C);
    }

    /// CODE 10 0x5c60.
    fn volume_slide(&mut self, ch: usize) {
        let c = &mut self.channels[ch];
        let up = c.note[3] >> 4;
        if up != 0 {
            c.volume = c.volume.wrapping_add(up as u16);
            if (c.volume as i16).wrapping_sub(0x40) >= 0 {
                c.volume = 0x40;
            }
        } else {
            c.volume = c.volume.wrapping_sub((c.note[3] & 0xF) as u16);
            if (c.volume as i16) < 0 {
                c.volume = 0;
            }
        }
        self.voices[ch].volume = c.volume;
    }

    fn sample(&self, ptr: Option<usize>, pos: u32) -> u8 {
        match ptr {
            Some(p) => *self.module.get(p + pos as usize).unwrap_or(&0),
            None => 0,
        }
    }

    /// CODE 10 0x5d0a: each voice into its own buffer, then the sum.
    fn mix(&mut self) -> [u8; TICK_SAMPLES] {
        let mut out = [0u8; TICK_SAMPLES];
        for ch in 0..4 {
            let v = self.voices[ch];
            let mut ptr = v.ptr;
            let mut pos = v.pos;
            let mut row = v.volume;
            row = row.saturating_sub(1);
            row <<= 8;
            let step_at = |period: u16| {
                let i = (period.wrapping_mul(4)) as i16 as isize;
                let at = (STEPS as isize + i) as usize;
                u32::from_be_bytes(
                    self.code
                        .get(at..at + 4)
                        .map_or([0; 4], |b| [b[0], b[1], b[2], b[3]]),
                )
            };
            let step = step_at(v.period);
            let (mut frac_step, mut int_step) = (step as u16, (step >> 16) as u16);
            let loop_step = if v.loop_len == 0 { 0 } else { step };
            let mut frac: u16 = 0;
            for o in out.iter_mut() {
                let (f, borrow) = frac.overflowing_sub(frac_step);
                frac = f;
                let low = pos as u16;
                let (r, b1) = low.overflowing_sub(int_step);
                let (r, b2) = r.overflowing_sub(borrow as u16);
                pos = (pos & 0xFFFF_0000) | r as u32;
                if b1 || b2 {
                    pos = v.loop_len;
                    frac_step = loop_step as u16;
                    int_step = (loop_step >> 16) as u16;
                    ptr = v.loop_ptr;
                }
                let index = (row & 0xFF00) | self.sample(ptr, pos) as u16;
                row = index;
                let at = VOLUMES as isize + index as i16 as isize;
                let value = if at < 0 {
                    0
                } else {
                    *self.code.get(at as usize).unwrap_or(&0)
                };
                *o = o.wrapping_add(value);
            }
            self.voices[ch].ptr = ptr;
            self.voices[ch].pos = pos;
        }
        out
    }
}
