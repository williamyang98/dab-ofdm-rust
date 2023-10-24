/// Creates the scrambling lookup table for each FFT bin of a DAB transmission used in OFDM.
/// The carrier map should provide a remapping for a subset of the FFT bins centered to the zero frequency bin.
pub fn get_dab_ofdm_carrier_map(carrier_map: &mut[usize], total_fft: usize) {
    // DOC: ETSI EN 300 401
    // Referring to clause 14.6 - Frequency interleaving
    // Before the OFDM symbol is sent for packing, the order of the carriers are scrambled
    // This is done so that selective fading doesn't destroy contiguous parts of the OFDM symbol bits
    let total_carriers = carrier_map.len();
    assert!(total_carriers > 0);
    assert!(total_fft > 0);
    assert!(total_fft % 4 == 0, "FFT length must be a multiple of 4");
    assert!(total_carriers <= total_fft, "Number of requested carriers must be less than or equal to total fft bins");

    let fft_index_dc = total_fft/2;
    let fft_index_start = fft_index_dc - total_carriers/2;
    let fft_index_end   = fft_index_dc + total_carriers/2;

    let mut carrier_map_index: usize = 0;
    let mut pi_value: usize = 0;
    for _ in 0..total_fft {
        // Referring to clause 14.6.1
        // The equation for mode I transmissions on generating this PI table is given
        // PI_TABLE is a 1 to 1 mapping for the N-fft
        let fft_index = pi_value;
        let k = total_fft/4;
        pi_value = (13*pi_value+k-1) % total_fft;

        // Referring to clause 14.6.1 
        // We are only interested in the FFT bins that we transmit in the OFDM symbol
        // -F <= f <= F where f =/= 0
        if fft_index < fft_index_start || fft_index > fft_index_end || fft_index == fft_index_dc {
            continue;
        };

        let carrier_out_index: usize = if fft_index < fft_index_dc {
            fft_index-fft_index_start
        } else {
            // NOTE: We ignore the DC bin so we adjust FFT bins above DC
            fft_index-fft_index_start-1
        };
        carrier_map[carrier_map_index] = carrier_out_index;
        carrier_map_index += 1;
    }
}