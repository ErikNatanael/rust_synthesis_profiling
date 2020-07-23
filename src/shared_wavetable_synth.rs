use dasp::signal::{Signal};

use std::f64::consts::PI;
// use std::f32::consts::PI;

// Performance: Reusing one wavetable seems to double the performance looking at jack's DSP meter

// Wavetable player as Signal (use Phase)
// Wavetable generator to create wavetable buffers (sine, saw, custom wavetables, look at SC for inspiration)
// synth using the wavetable and some filters. Connect to some reverb.

pub type Sample = f64;

pub struct Wavetable {
    buffer: Vec<Sample>,
    // Store the size as an f64 to find fractional indexes without typecasting
    size: Sample,
}

impl Wavetable {
    fn new(wavetable_size: usize) -> Self {
        let w_size = if !is_power_of_2(wavetable_size) {
            // Make a power of two by taking the log2 and discarding the fractional part of the answer and then squaring again
            ((wavetable_size as f64).log2() as usize).pow(2)
        } else {
            wavetable_size
        };
        let buffer = vec![0.0; w_size];
        Wavetable {
            buffer,
            size: wavetable_size as Sample,
        }
    }
    pub fn sine(wavetable_size: usize) -> Self {
        let mut wt = Wavetable::new(wavetable_size);
        // Fill buffer with a sine
        for i in 0..wavetable_size {
            wt.buffer[i] = ((i as Sample / wt.size) * PI * 2.0).sin();
        }
        wt
    }

    /// Linearly interpolate between the value in between which the phase points.
    /// The phase is assumed to be 0 <= phase < 1
    #[inline]
    fn get_linear_interp(&self, phase: Sample) -> Sample {
        let index = self.size * phase;
        let mix = index.fract();
        self.buffer[index.floor() as usize] * (1.0-mix) + self.buffer[index.ceil() as usize % self.buffer.len()] * mix
    }

    /// Get the closest sample with no interpolation
    #[inline]
    fn get(&self, phase: Sample) -> Sample {
        self.buffer[(self.size * phase) as usize]
    }
}

fn is_power_of_2(num: usize) -> bool {
    return num > 0 && num&(num-1) == 0;
}

struct Phase {
    value: Sample,
    step: Sample,
}

impl Phase {
    fn new() -> Self {
        Phase {
            value: 0.0,
            step: 0.0,
        }
    }
    fn from_freq(freq: Sample, sample_rate: Sample) -> Self {
        let mut phase = Phase::new();
        phase.set_freq(freq, sample_rate);
        phase
    }
    fn set_freq(&mut self, freq: Sample, sample_rate: Sample) {
        self.step = freq / sample_rate;
    }
}

impl Signal for Phase {
    type Frame = Sample;

    #[inline]
    fn next(&mut self) -> Self::Frame {
        // Use the phase to index into the wavetable
        let out = self.value;
        self.value = (self.value + self.step) % 1.0;
        out
    }
}

// It seems very hard to keep Oscillator being Signal **and** have it fetch the wavetable from
// a WavetableArena every call to next() since next() provides no state. The state has to be in the
// Signal. We could put the Wavetable inside the Osciallator, and be unable to share it between oscillators 
// or modify it. We could also put an Rc<Wavetable> in the Oscillator, but this is not Send unless we're resorting to unsafe.
pub struct Oscillator {
    step: Sample,
    phase: Sample,
    wavetable: WavetableIndex,
    amp: Sample,
}

impl Oscillator
{
    pub fn new(wavetable: WavetableIndex) -> Self {
        Oscillator {
            step: 0.0,
            phase: 0.0,
            wavetable,
            amp: 1.0,
        }
    }
    pub fn from_freq(freq: Sample, sample_rate: Sample, wavetable: WavetableIndex, amp: Sample) -> Self {
        Oscillator {
            step: freq / sample_rate,
            phase: 0.0,
            wavetable,
            amp,
        }
    }
    #[inline]
    fn next(&mut self, wavetable_arena: &WavetableArena) -> Sample {
        self.phase += self.step;
        while self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        // Use the phase to index into the wavetable
        match wavetable_arena.get(self.wavetable) {
            Some(wt) => wt.get(self.phase) * self.amp,
            None => 0.0
        }
    }
}

// We could later turn WavetableIndex into a generational index if we'd want
type WavetableIndex = usize;
struct WavetableArena {
    wavetables: Vec<Option<Wavetable>>,
    next_free_index: WavetableIndex,
    freed_indexes: Vec<WavetableIndex>,
}

impl WavetableArena {
    fn new() -> Self {
        let mut wavetables = Vec::with_capacity(100);
        for i in 0..100 {
            wavetables.push(None);
        }
        WavetableArena {
            wavetables,
            next_free_index: 0,
            freed_indexes: vec![]
        }
    }
    fn get(&self, index: WavetableIndex) -> &Option<Wavetable> {
        &self.wavetables[index]
    }
    fn add(&mut self, wavetable: Wavetable) -> WavetableIndex {
        // TODO: In order to do this safely in an audio thread we should pass the old value on to a helper thread for deallocation
        // since dropping it here would probably deallocate it.
        let old_wavetable = self.wavetables[self.next_free_index].replace(wavetable);
        let index = self.next_free_index;
        self.next_free_index += 1;
        // TODO: Check that the next free index is within the bounds of the wavetables Vec or else use the indexes that have been freed
        index
    }
}

pub struct SynthesisEngine {
    wavetable_arena: WavetableArena,
    oscillators: Vec<Oscillator>,
}

impl SynthesisEngine {
    pub fn new(sample_rate: Sample) -> Self {
        let mut wavetable_arena = WavetableArena::new();
        let sine_wt = wavetable_arena.add(Wavetable::sine(131072));
        let mut oscillators: Vec<Oscillator> = Vec::new();
        let num_oscillators = 400;
        let dampening = 1.0 / num_oscillators as Sample;
        // Add a wavetable to the arena
        for n in 0..num_oscillators {
            let sig = Oscillator::from_freq(200.0, 
                sample_rate, 
                sine_wt,
                dampening
            );
            oscillators.push(
                sig
            );
        }

        SynthesisEngine {
            wavetable_arena,
            oscillators
        }
    }

    pub fn next(&mut self) -> Sample {
        let mut amp = 0.0;
        
        for osc in &mut self.oscillators {
            amp += osc.next(&self.wavetable_arena);
        }
        amp
    }
}