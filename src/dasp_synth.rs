use dasp::signal::{Signal};
use dasp::signal;

use std::rc::Rc;
use std::f64::consts::PI;

pub type Sample = f64;

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
            let sig = signal::rate(sample_rate).const_hz(200.0).sine().mul_amp(signal::gen(move|| dampening.clone()));
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