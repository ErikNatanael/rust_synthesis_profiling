use std::io;
use std::str::FromStr;

mod shared_wavetable_synth;
mod dasp_synth;
mod oscen_synth;
mod owned_wavetable_synth;
use shared_wavetable_synth::{SynthesisEngine, Sample};
// use dasp_synth::{SynthesisEngine, Sample};
// use oscen_synth::{SynthesisEngine, Sample};
// use owned_wavetable_synth::{SynthesisEngine, Sample};


/// Simple Phase struct for benchmarking different approaches to phase calculations in isolation
struct Phase {
    phase: f64,
    step: f64,
}

impl Phase {
    pub fn increase_modulo(&mut self) {
        self.phase = (self.phase + self.step) % 1.0_f64;
    }
    pub fn increase_while(&mut self) {
        self.phase += self.step;
        while self.phase > 1.0 {
            self.phase -= 1.0;
        }
    }
    pub fn increase_modulo_flex_fold(&mut self, fold: f64) {
        self.phase = (self.phase + self.step) % fold;
    }
    pub fn increase_while_flex_fold(&mut self, fold: f64) {
        self.phase += self.step;
        while self.phase > fold {
            self.phase -= fold;
        }
    }
    pub fn increase_baked_faust(&mut self) {
        self.phase = self.phase + self.step - (self.phase + self.step).floor();
    }
    pub fn use_phase(&self) -> f64 {
        self.phase.sin()
    }
}

fn main() {
    // Isolated benchmarks
    // Phase calculation benchmarks
    let mut phase = Phase { phase: 0.0, step: 442.0 / 44100.0 };
    let mut temp = 0.0;
    use std::time::Instant;
    let now_modulo = Instant::now();
    for _ in 0..16777216 {
        phase.increase_modulo();
        phase.use_phase();
    }
    println!("Modulo phase increase: {:?}", now_modulo.elapsed().as_secs_f64());
    println!("Phase: {}", phase.phase);
    let now_while = Instant::now();
    for _ in 0..16777216 {
        phase.increase_while();
        phase.use_phase();
    }
    println!("While loop phase increase: {:?}", now_while.elapsed().as_secs_f64());
    println!("Phase: {}", phase.phase);
    let now_modulo_fold = Instant::now();
    for _ in 0..16777216 {
        phase.increase_modulo_flex_fold(3.14159);
        phase.use_phase();
    }
    println!("Modulo phase increase flexible fold: {:?}", now_modulo_fold.elapsed().as_secs_f64());
    println!("Phase: {}", phase.phase);
    let now_while_fold = Instant::now();
    for _ in 0..16777216 {
        phase.increase_while_flex_fold(3.14159);
        phase.use_phase();
    }
    println!("While loop phase increase flexible fold: {:?}", now_while_fold.elapsed().as_secs_f64());
    println!("Phase: {}", phase.phase);

    let now_faust = Instant::now();
    for _ in 0..16777216 {
        phase.increase_baked_faust();
        phase.use_phase();
    }
    println!("Faust approach phase increase: {:?}", now_faust.elapsed().as_secs_f64());
    // This is required for the "faust" approach not to be optimised away so I added it after the other
    // benchmarks as well for fairness.
    println!("Phase: {}", phase.phase); 

    // 1. open a client
    let (client, _status) =
        jack::Client::new("ftrace_sonifier", jack::ClientOptions::NO_START_SERVER).unwrap();

    // 2. register port
    let mut out_port_l = client
        .register_port("out_l", jack::AudioOut::default())
        .unwrap();
    let mut out_port_r = client
        .register_port("out_r", jack::AudioOut::default())
        .unwrap();

    let mut output_port_names = vec![];
    output_port_names.push(out_port_l.name().unwrap());
    output_port_names.push(out_port_r.name().unwrap());
    // Double just because the headphone output is channels 3 and 4 on my system
    output_port_names.push(out_port_l.name().unwrap());
    output_port_names.push(out_port_r.name().unwrap());

        
    // 3. define process callback handler
    let sample_rate = client.sample_rate();

    let mut synthesis_engine = SynthesisEngine::new(sample_rate as Sample);



    let process = jack::ClosureProcessHandler::new(
        move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
            // This gets called once for every block

            // Get output buffer
            let out_l = out_port_l.as_mut_slice(ps);
            let out_r = out_port_r.as_mut_slice(ps);

            // Write output
            for (l, r) in out_l.iter_mut().zip(out_r.iter_mut()) {

                let mut frame = [0.0; 2];

                let modular_sample = synthesis_engine.next();
                frame[0] += modular_sample;
                frame[1] += modular_sample;

                // Write the sound to the channel buffer
                *l = frame[0] as f32;
                *r = frame[1] as f32;
            }

            // Continue as normal
            jack::Control::Continue
        },
    );

    // 4. activate the client
    let async_client = client.activate_async((), process).unwrap();
    // processing starts here

    // Get the system ports
    let system_ports = async_client.as_client().ports(
        Some("system:playback_.*"),
        None,
        jack::PortFlags::empty(),
    );

    // Connect outputs from this client to the system playback inputs
    for i in 0..output_port_names.len() {
        if i >= system_ports.len() {
            break;
        }
        match async_client
            .as_client()
            .connect_ports_by_name(&output_port_names[i], &system_ports[i])
        {
            Ok(_) => (),
            Err(e) => println!("Unable to connect to port with error {}", e),
        }
    }

    while let Some(f) = read_freq() {
        println!("input: {}", f);
    }

    // 6. Optional deactivate. Not required since active_client will deactivate on
    // drop, though explicit deactivate may help you identify errors in
    // deactivate.
    async_client.deactivate().unwrap();
}

/// Attempt to read a frequency from standard in. Will block until there is
/// user input. `None` is returned if there was an error reading from standard
/// in, or the retrieved string wasn't a compatible u16 integer.
fn read_freq() -> Option<f64> {
    let mut user_input = String::new();
    match io::stdin().read_line(&mut user_input) {
        Ok(_) => u16::from_str(&user_input.trim()).ok().map(|n| n as f64),
        Err(_) => None,
    }
}