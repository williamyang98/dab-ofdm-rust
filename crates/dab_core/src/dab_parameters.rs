use crate::dab_transmission_modes::DabTransmissionMode;

/// Refer to the structs OfdmParameters and DabRadioParameters for an explanation of what these constants mean.
/// This contains the all the information required for OFDM demodulation and digital decoding.
pub struct DabParameters {
    /// Number of OFDM data symbols in a transmission frame.
    pub nb_symbols: usize,
    /// Duration of NULL symbol.
    pub nb_null_period: usize,
    /// Duration of OFDM data symbol.
    pub nb_symbol_period: usize,
    /// Duration of cyclic prefix in OFDM data symbol.
    pub nb_fft: usize,
    /// Number of FFT bins that are data carriers centered around DC.
    pub nb_fft_data_carriers: usize,
    /// Number of symbols for the fast information channel (FIC). This carries metadata about the ensemble.
    pub nb_fic_symbols: usize,
    /// Number of symbols for the main service channel (MSC). This carries radio data for each channel in the ensemble.
    pub nb_msc_symbols: usize,
    /// Number of fast information blocks (FIB) in the FIC
    pub nb_fibs_in_fic: usize,
    /// Number of common interleaved frames (CIF) in the MSC.
    pub nb_cifs_in_msc: usize,
}

/// These constants are defined for a sampling frequency of 2.048MHz.
pub fn get_dab_parameters(transmission_mode: DabTransmissionMode) -> DabParameters {
    let params = match transmission_mode {
        DabTransmissionMode::I => DabParameters { 
            nb_symbols: 76, 
            nb_null_period: 2656, 
            nb_symbol_period: 2552, 
            nb_fft: 2048, 
            nb_fft_data_carriers: 1536, 
            nb_fic_symbols: 3, 
            nb_msc_symbols: 72, 
            nb_fibs_in_fic: 12, 
            nb_cifs_in_msc: 4, 
        },
        DabTransmissionMode::II => DabParameters { 
            nb_symbols: 76, 
            nb_null_period: 664, 
            nb_symbol_period: 638, 
            nb_fft: 512, 
            nb_fft_data_carriers: 384, 
            nb_fic_symbols: 3, 
            nb_msc_symbols: 72, 
            nb_fibs_in_fic: 3, 
            nb_cifs_in_msc: 1, 
        },
        DabTransmissionMode::III => DabParameters { 
            nb_symbols: 153, 
            nb_null_period: 345, 
            nb_symbol_period: 319, 
            nb_fft: 256, 
            nb_fft_data_carriers: 192, 
            nb_fic_symbols: 8, 
            nb_msc_symbols: 144, 
            nb_fibs_in_fic: 4, 
            nb_cifs_in_msc: 1, 
        },
        DabTransmissionMode::IV => DabParameters { 
            nb_symbols: 76, 
            nb_null_period: 1328, 
            nb_symbol_period: 1276, 
            nb_fft: 1024, 
            nb_fft_data_carriers: 768, 
            nb_fic_symbols: 3, 
            nb_msc_symbols: 72, 
            nb_fibs_in_fic: 6, 
            nb_cifs_in_msc: 2, 
        },
    };

    assert!(params.nb_symbols >= 2, "Number of symbols must be at least 2 due to differential QPSK encoding");
    assert!(params.nb_symbol_period >= params.nb_fft, "Number of samples in symbol is less than FFT resolution");
    assert!(params.nb_fft >= params.nb_fft_data_carriers, "Number of data carriers is limited to FFT resolution");
    assert!((params.nb_symbols-1) == (params.nb_fic_symbols + params.nb_msc_symbols), "Number of data symbols after DQPSK doesn't match number of FIC and MSC symbols");
    assert!(params.nb_fibs_in_fic % params.nb_cifs_in_msc == 0, "The number of FIBs in the FIC must be a multiple of the number of CIFs in the MSC.");

    params
}