# Introduction
[![x86-windows](https://github.com/williamyang98/dab-ofdm-rust/actions/workflows/x86-windows.yml/badge.svg)](https://github.com/williamyang98/dab-ofdm-rust/actions/workflows/x86-windows.yml)
[![x86-ubuntu](https://github.com/williamyang98/dab-ofdm-rust/actions/workflows/x86-ubuntu.yml/badge.svg)](https://github.com/williamyang98/dab-ofdm-rust/actions/workflows/x86-ubuntu.yml)

A rust port of the OFDM demodulator for DAB radio found at [williamyang98/DAB-Radio](https://github.com/williamyang98/DAB-Radio). It is intended to be used as a direct replacement for <code>ofdm_demod_gui.exe</code>, found [here](https://github.com/williamyang98/DAB-Radio/tree/master/examples).

It is slower than the C++ version due to the lack of SIMD acceleration. This serves as a test project for the Rust programming language.

# Instructions
## Building
<code>cargo build --release --bin ofdm_demod</code>

## Running
Refer to the instructions found [here](https://github.com/williamyang98/DAB-Radio/tree/master/src/examples) for <code>ofdm_demod_gui</code>. 

This should be used with the companion applications from the releases page [here](https://github.com/williamyang98/DAB-Radio/releases). If you do not have an SDR dongle you can download sample data instead.

You can run from the SDR dongle.

<code>./rtl_sdr | ./target/release/ofdm_demod | ./basic_radio_app_no_demod</code>

Or you can run from sample data found [here](https://github.com/williamyang98/DAB-Radio/releases/tag/raw-iq-data).

<code>./target/release/ofdm_demod -i ./baseband_9C_0.raw | ./basic_radio_app_no_demod</code>

If you are only interested in testing the OFDM demodulator you can redirect the output to <code>/dev/null</code>.

<code>./target/release/ofdm_demod -i ./baseband_9C_0.raw > /dev/null</code>
# Gallery
![Screenshot](/docs/screenshot_ofdm_demod.png)