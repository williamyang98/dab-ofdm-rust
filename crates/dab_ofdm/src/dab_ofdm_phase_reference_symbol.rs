use num::complex::Complex;
use dab_core::dab_transmission_modes::{DabTransmissionMode};

// DOC: ETSI EN 300 401
// Referring to clause 14.3.2 - Phase reference symbol 
// The phase reference symbol is construction using two tables
// Table 23 which contains PRS segments, and Table 24 which contains a list of h-values
// 
// DOC: docs/DAB_implementation_in_SDR_detailed.pdf
// For the other transmission modes including I,II,III,IV refer to appendix B
// This detailed document provides the required tables for these other transmission modes as well

/// A phase reference symbol consists of multiple segments.
/// The parameters i and n are used to calculate the phase at that FFT bin in conjuction with the H_table.
struct PrsSegment {
    fft_bin_start: i32,
    fft_bin_end: i32,
    h_table_row: usize,
    phase_multiple: usize,
}

const PRS_MODE_I: [PrsSegment; 48] = [
    PrsSegment { fft_bin_start: -768, fft_bin_end:-737, h_table_row: 0, phase_multiple: 1 },
    PrsSegment { fft_bin_start: -736, fft_bin_end:-705, h_table_row: 1, phase_multiple: 2 },
    PrsSegment { fft_bin_start: -704, fft_bin_end:-673, h_table_row: 2, phase_multiple: 0 },
    PrsSegment { fft_bin_start: -672, fft_bin_end:-641, h_table_row: 3, phase_multiple: 1 },
    PrsSegment { fft_bin_start: -640, fft_bin_end:-609, h_table_row: 0, phase_multiple: 3 },
    PrsSegment { fft_bin_start: -608, fft_bin_end:-577, h_table_row: 1, phase_multiple: 2 },
    PrsSegment { fft_bin_start: -576, fft_bin_end:-545, h_table_row: 2, phase_multiple: 2 },
    PrsSegment { fft_bin_start: -544, fft_bin_end:-513, h_table_row: 3, phase_multiple: 3 },
    PrsSegment { fft_bin_start: -512, fft_bin_end:-481, h_table_row: 0, phase_multiple: 2 },
    PrsSegment { fft_bin_start: -480, fft_bin_end:-449, h_table_row: 1, phase_multiple: 1 },
    PrsSegment { fft_bin_start: -448, fft_bin_end:-417, h_table_row: 2, phase_multiple: 2 },
    PrsSegment { fft_bin_start: -416, fft_bin_end:-385, h_table_row: 3, phase_multiple: 3 },
    PrsSegment { fft_bin_start: -384, fft_bin_end:-353, h_table_row: 0, phase_multiple: 1 },
    PrsSegment { fft_bin_start: -352, fft_bin_end:-321, h_table_row: 1, phase_multiple: 2 },
    PrsSegment { fft_bin_start: -320, fft_bin_end:-289, h_table_row: 2, phase_multiple: 3 },
    PrsSegment { fft_bin_start: -288, fft_bin_end:-257, h_table_row: 3, phase_multiple: 3 },
    PrsSegment { fft_bin_start: -256, fft_bin_end:-225, h_table_row: 0, phase_multiple: 2 },
    PrsSegment { fft_bin_start: -224, fft_bin_end:-193, h_table_row: 1, phase_multiple: 2 },
    PrsSegment { fft_bin_start: -192, fft_bin_end:-161, h_table_row: 2, phase_multiple: 2 },
    PrsSegment { fft_bin_start: -160, fft_bin_end:-129, h_table_row: 3, phase_multiple: 1 },
    PrsSegment { fft_bin_start: -128, fft_bin_end: -97, h_table_row: 0, phase_multiple: 1 },
    PrsSegment { fft_bin_start:  -96, fft_bin_end: -65, h_table_row: 1, phase_multiple: 3 },
    PrsSegment { fft_bin_start:  -64, fft_bin_end: -33, h_table_row: 2, phase_multiple: 1 },
    PrsSegment { fft_bin_start:  -32, fft_bin_end:  -1, h_table_row: 3, phase_multiple: 2 },
    PrsSegment { fft_bin_start:    1, fft_bin_end:  32, h_table_row: 0, phase_multiple: 3 },
    PrsSegment { fft_bin_start:   33, fft_bin_end:  64, h_table_row: 3, phase_multiple: 1 },
    PrsSegment { fft_bin_start:   65, fft_bin_end:  96, h_table_row: 2, phase_multiple: 1 },
    PrsSegment { fft_bin_start:   97, fft_bin_end: 128, h_table_row: 1, phase_multiple: 1 },
    PrsSegment { fft_bin_start:  129, fft_bin_end: 160, h_table_row: 0, phase_multiple: 2 },
    PrsSegment { fft_bin_start:  161, fft_bin_end: 192, h_table_row: 3, phase_multiple: 2 },
    PrsSegment { fft_bin_start:  193, fft_bin_end: 224, h_table_row: 2, phase_multiple: 1 },
    PrsSegment { fft_bin_start:  225, fft_bin_end: 256, h_table_row: 1, phase_multiple: 0 },
    PrsSegment { fft_bin_start:  257, fft_bin_end: 288, h_table_row: 0, phase_multiple: 2 },
    PrsSegment { fft_bin_start:  289, fft_bin_end: 320, h_table_row: 3, phase_multiple: 2 },
    PrsSegment { fft_bin_start:  321, fft_bin_end: 352, h_table_row: 2, phase_multiple: 3 },
    PrsSegment { fft_bin_start:  353, fft_bin_end: 384, h_table_row: 1, phase_multiple: 3 },
    PrsSegment { fft_bin_start:  385, fft_bin_end: 416, h_table_row: 0, phase_multiple: 0 },
    PrsSegment { fft_bin_start:  417, fft_bin_end: 448, h_table_row: 3, phase_multiple: 2 },
    PrsSegment { fft_bin_start:  449, fft_bin_end: 480, h_table_row: 2, phase_multiple: 1 },
    PrsSegment { fft_bin_start:  481, fft_bin_end: 512, h_table_row: 1, phase_multiple: 3 },
    PrsSegment { fft_bin_start:  513, fft_bin_end: 544, h_table_row: 0, phase_multiple: 3 },
    PrsSegment { fft_bin_start:  545, fft_bin_end: 576, h_table_row: 3, phase_multiple: 3 },
    PrsSegment { fft_bin_start:  577, fft_bin_end: 608, h_table_row: 2, phase_multiple: 3 },
    PrsSegment { fft_bin_start:  609, fft_bin_end: 640, h_table_row: 1, phase_multiple: 0 },
    PrsSegment { fft_bin_start:  641, fft_bin_end: 672, h_table_row: 0, phase_multiple: 3 },
    PrsSegment { fft_bin_start:  673, fft_bin_end: 704, h_table_row: 3, phase_multiple: 0 },
    PrsSegment { fft_bin_start:  705, fft_bin_end: 736, h_table_row: 2, phase_multiple: 1 },
    PrsSegment { fft_bin_start:  737, fft_bin_end: 768, h_table_row: 1, phase_multiple: 1 },
];

const PRS_MODE_II: [PrsSegment; 12] = [
    PrsSegment { fft_bin_start: -192, fft_bin_end:-161, h_table_row: 0, phase_multiple: 2 },
    PrsSegment { fft_bin_start: -160, fft_bin_end:-129, h_table_row: 1, phase_multiple: 3 },
    PrsSegment { fft_bin_start: -128, fft_bin_end: -97, h_table_row: 2, phase_multiple: 2 },
    PrsSegment { fft_bin_start:  -96, fft_bin_end: -65, h_table_row: 3, phase_multiple: 2 },
    PrsSegment { fft_bin_start:  -64, fft_bin_end: -33, h_table_row: 0, phase_multiple: 1 },
    PrsSegment { fft_bin_start:  -32, fft_bin_end:  -1, h_table_row: 1, phase_multiple: 2 },
    PrsSegment { fft_bin_start:    1, fft_bin_end:  32, h_table_row: 2, phase_multiple: 0 },
    PrsSegment { fft_bin_start:   33, fft_bin_end:  64, h_table_row: 1, phase_multiple: 2 },
    PrsSegment { fft_bin_start:   65, fft_bin_end:  96, h_table_row: 0, phase_multiple: 2 },
    PrsSegment { fft_bin_start:   97, fft_bin_end: 128, h_table_row: 3, phase_multiple: 1 },
    PrsSegment { fft_bin_start:  129, fft_bin_end: 160, h_table_row: 2, phase_multiple: 0 },
    PrsSegment { fft_bin_start:  161, fft_bin_end: 192, h_table_row: 1, phase_multiple: 3 },
];

const PRS_MODE_III: [PrsSegment; 6] = [
    PrsSegment { fft_bin_start: -96, fft_bin_end: -65, h_table_row: 0, phase_multiple: 2 },
    PrsSegment { fft_bin_start: -64, fft_bin_end: -33, h_table_row: 1, phase_multiple: 3 },
    PrsSegment { fft_bin_start: -32, fft_bin_end:  -1, h_table_row: 2, phase_multiple: 0 },
    PrsSegment { fft_bin_start:   1, fft_bin_end:  32, h_table_row: 3, phase_multiple: 2 },
    PrsSegment { fft_bin_start:  33, fft_bin_end:  64, h_table_row: 2, phase_multiple: 2 },
    PrsSegment { fft_bin_start:  65, fft_bin_end:  96, h_table_row: 1, phase_multiple: 2 },
];

const PRS_MODE_IV: [PrsSegment; 24] = [
    PrsSegment { fft_bin_start: -384, fft_bin_end: -353, h_table_row: 0, phase_multiple: 0 },
    PrsSegment { fft_bin_start: -352, fft_bin_end: -321, h_table_row: 1, phase_multiple: 1 },
    PrsSegment { fft_bin_start: -320, fft_bin_end: -289, h_table_row: 2, phase_multiple: 1 },
    PrsSegment { fft_bin_start: -288, fft_bin_end: -257, h_table_row: 3, phase_multiple: 2 },
    PrsSegment { fft_bin_start: -256, fft_bin_end: -225, h_table_row: 0, phase_multiple: 2 },
    PrsSegment { fft_bin_start: -224, fft_bin_end: -193, h_table_row: 1, phase_multiple: 2 },
    PrsSegment { fft_bin_start: -192, fft_bin_end: -161, h_table_row: 2, phase_multiple: 0 },
    PrsSegment { fft_bin_start: -160, fft_bin_end: -129, h_table_row: 3, phase_multiple: 3 },
    PrsSegment { fft_bin_start: -128, fft_bin_end:  -97, h_table_row: 0, phase_multiple: 3 },
    PrsSegment { fft_bin_start:  -96, fft_bin_end:  -65, h_table_row: 1, phase_multiple: 1 },
    PrsSegment { fft_bin_start:  -64, fft_bin_end:  -33, h_table_row: 2, phase_multiple: 3 },
    PrsSegment { fft_bin_start:  -32, fft_bin_end:   -1, h_table_row: 3, phase_multiple: 2 },
    PrsSegment { fft_bin_start:    1, fft_bin_end:   32, h_table_row: 0, phase_multiple: 0 },
    PrsSegment { fft_bin_start:   33, fft_bin_end:   64, h_table_row: 3, phase_multiple: 1 },
    PrsSegment { fft_bin_start:   65, fft_bin_end:   96, h_table_row: 2, phase_multiple: 0 },
    PrsSegment { fft_bin_start:   97, fft_bin_end:  128, h_table_row: 1, phase_multiple: 2 },
    PrsSegment { fft_bin_start:  129, fft_bin_end:  160, h_table_row: 0, phase_multiple: 0 },
    PrsSegment { fft_bin_start:  161, fft_bin_end:  192, h_table_row: 3, phase_multiple: 1 },
    PrsSegment { fft_bin_start:  193, fft_bin_end:  224, h_table_row: 2, phase_multiple: 2 },
    PrsSegment { fft_bin_start:  225, fft_bin_end:  256, h_table_row: 1, phase_multiple: 2 },
    PrsSegment { fft_bin_start:  257, fft_bin_end:  288, h_table_row: 0, phase_multiple: 2 },
    PrsSegment { fft_bin_start:  289, fft_bin_end:  320, h_table_row: 3, phase_multiple: 1 },
    PrsSegment { fft_bin_start:  321, fft_bin_end:  352, h_table_row: 2, phase_multiple: 3 },
    PrsSegment { fft_bin_start:  353, fft_bin_end:  384, h_table_row: 1, phase_multiple: 0 },
];

const H_TABLE: [[usize;32]; 4] = [
    [0, 2, 0, 0, 0, 0, 1, 1, 2, 0, 0, 0, 2, 2, 1, 1, 0, 2, 0, 0, 0, 0, 1, 1, 2, 0, 0, 0, 2, 2, 1, 1],
    [0, 3, 2, 3, 0, 1, 3, 0, 2, 1, 2, 3, 2, 3, 3, 0, 0, 3, 2, 3, 0, 1, 3, 0, 2, 1, 2, 3, 2, 3, 3, 0],
    [0, 0, 0, 2, 0, 2, 1, 3, 2, 2, 0, 2, 2, 0, 1, 3, 0, 0, 0, 2, 0, 2, 1, 3, 2, 2, 0, 2, 2, 0, 1, 3],
    [0, 1, 2, 1, 0, 3, 3, 2, 2, 3, 2, 1, 2, 1, 3, 2, 0, 1, 2, 1, 0, 3, 3, 2, 2, 3, 2, 1, 2, 1, 3, 2],
];

/// Creates the FFT result of the phase reference symbol in OFDM for a given transmission mode for DAB radio.
pub fn get_dab_ofdm_phase_reference_symbol_fft(prs_fft: &mut[Complex<f32>], transmission_mode: DabTransmissionMode) {
    let prs_segments: &[PrsSegment] = match transmission_mode {
        DabTransmissionMode::I   => &PRS_MODE_I,
        DabTransmissionMode::II  => &PRS_MODE_II,
        DabTransmissionMode::III => &PRS_MODE_III,
        DabTransmissionMode::IV  => &PRS_MODE_IV,
    };

    let total_fft = prs_fft.len();

    // NOTE: PRS symbol is symmetrical along frequency axis and FFT buffer should have the DC bin at the start
    let total_segments = prs_segments.len();
    let total_carriers = (prs_segments[total_segments-1].fft_bin_end - prs_segments[0].fft_bin_start + 1) as usize;
    assert!(prs_segments[total_segments-1].fft_bin_end == -prs_segments[0].fft_bin_start, "FFT bins must be centered and symmetrical");
    assert!(total_fft >= total_carriers, "PRS FFT buffer is not large enough to fit phase reference symbol. {} < {}", total_fft, total_carriers);

    // Zero out FFT bins that might not be initialised in this call
    for value in prs_fft.iter_mut() {
        value.re = 0.0;
        value.im = 0.0;
    }

    // DOC: ETSI EN 300 401
    // Referring to clause 14.3.2 - Phase reference symbol 
    // The equation for constructing the PRS in terms of a list of phases for each subcarrier is given
    // In our demodulator code this is equivalent to the FFT result
    for segment in prs_segments {
        let fft_bins = segment.fft_bin_start..=segment.fft_bin_end;
        for (h_table_column, fft_bin) in fft_bins.enumerate() {
            let h_value = H_TABLE[segment.h_table_row][h_table_column];
            let phase_multiple = h_value+segment.phase_multiple;

            use std::f32::consts::FRAC_PI_2;
            let phase = FRAC_PI_2 * (phase_multiple as f32);
            let prs = Complex::<f32>::cis(phase);
            
            let fft_index: i32;
            // -F/2 <= f < 0
            if fft_bin < 0 {
                fft_index = fft_bin + (total_fft as i32);
            // 0 < f <= F/2
            } else {
                fft_index = fft_bin;
            }
            prs_fft[fft_index as usize] = prs;
        }
    }
}
