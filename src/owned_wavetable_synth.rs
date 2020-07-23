use dasp::signal::{Signal};
use dasp::signal;

use std::rc::Rc;
use std::f64::consts::PI;

// Wavetable player as Signal (use Phase)
// Wavetable generator to create wavetable buffers (sine, saw, custom wavetables, look at SC for inspiration)
// synth using the wavetable and some filters. Connect to some reverb.

pub type Sample = f64;

pub struct Wavetable {
    buffer: Vec<Sample>,
    // Store the size as an f64 to find fractional indexes without typecasting
    size: f64,
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
            wt.buffer[i] = ((i as f64 / wt.size) * PI * 2.0).sin();
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
    fn from_freq(freq: f64, sample_rate: f64) -> Self {
        let mut phase = Phase::new();
        phase.set_freq(freq, sample_rate);
        phase
    }
    fn set_freq(&mut self, freq: f64, sample_rate: f64) {
        self.step = freq / sample_rate;
    }
}

impl Signal for Phase {
    type Frame = f64;

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

    phase: Phase,
    wavetable: Wavetable,
}

impl Oscillator
{
    pub fn new(wavetable: Wavetable) -> Self {
        Oscillator {
            phase: Phase::new(),
            wavetable
        }
    }
    pub fn from_freq(freq: f64, sample_rate: f64, wavetable: Wavetable) -> Self {
        Oscillator {
            phase: Phase::from_freq(freq, sample_rate),
            wavetable
        }
    }
}

impl Signal for Oscillator {
    type Frame = f64;

    #[inline]
    fn next(&mut self) -> Self::Frame {
        // Use the phase to index into the wavetable
        self.wavetable.get_linear_interp(self.phase.next())
    }
}

pub struct SynthesisEngine {
    oscillators: Vec<Box<dyn Signal<Frame = f64> + Send + Sync>>,
}

impl SynthesisEngine {
    pub fn new(sample_rate: f64) -> Self {
        let mut oscillators: Vec<Box<dyn Signal<Frame = f64> + Send + Sync>> = Vec::new();
        let num_oscillators = 400;
        let dampening = 1.0 / num_oscillators as f64;
        // Add a wavetable to the arena
        for n in 1..num_oscillators {
            let sig = Oscillator::from_freq(100.0, sample_rate, Wavetable::sine(4096))
                .mul_amp(signal::gen_mut(move || dampening ));
            oscillators.push(
                Box::new(sig)
            );
        }

        SynthesisEngine {
            oscillators
        }
    }

    pub fn next(&mut self) -> Sample {
        let mut amp = 0.0;
        
        for osc in &mut self.oscillators {
            amp += osc.next();
        }
        amp
    }
}