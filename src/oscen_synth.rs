use oscen::filters::Lpf;
use oscen::operators::Modulator;
use oscen::oscillators::{SineOsc, SquareOsc};
use oscen::signal::*;

pub type Sample = f64;

pub struct SynthesisEngine {
    rack: Rack,
    sample_rate: Real,
}

impl SynthesisEngine {
    pub fn new(sample_rate: f64) -> Self {
        // Build the Synth.
        // A Rack is a collection of synth modules.
        let mut rack = Rack::new(vec![]);

        let num_oscillators = 400;
        let amp = 1.0 / num_oscillators as f64;
        for i in 0..num_oscillators {
            let sine = SineOsc::new().amplitude(amp).hz(200).rack(&mut rack);
        }

        SynthesisEngine {
            rack,
            sample_rate
        }
    }
    pub fn next(&mut self) -> f64 {
        self.rack.signal(self.sample_rate)
    }
}