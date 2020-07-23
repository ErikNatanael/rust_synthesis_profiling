# Rust Sound Synthesis Profiling Dummy Project

The goal of this repository is to play as many sine waves as possible with as little DSP load as possible in order to maximise DSP performance.

## Method

Play 400 sine waves. Record the "DSP load" meter presented through QjackCtl and whether there are any xruns or not. Did thing improve?

JACK settings for the results below: 48kHz, 256 frames/buffer, 3 periods

Comparing exact percentages is probably only possible on the same machine.

## Results so far


I tried a few different implementations of playing sine tones:

| Implementation                          |           DSP% |
|-----------------------------------------+----------------|
| shared_wavetable_synth                  |         27-31% |
| oscen_synth                             |  runaway xruns/no sound |
| owned_wavetable_synth                   |            48% |
| dasp_synth (no wavetable)               | 35% with xruns |

After this I decided to focus on the shared_wavetable_synth

|-----------------------------------------+----------------|
| baseline (no DSP, only the loop)        |           1.1% (indistinguishable from idle) |
| DSP, but no copying to the frame buffer |          15.1% |
| shared_wavetable_synth no interpolation |          17.6% |
|                                         |                |


Trying to get the CPU usage of the shared_resources_synth implementation down.

- Changing from f64 to f32 = no improvement
- Changing from linear interpolation to no interpolations: 34% improvement
- Changing the size of the wavetable: no improvement
- Enabling lto in the release profile: slightly worse performance



## Acknowledgements

The main loop is adapted from examples from the jack crate.