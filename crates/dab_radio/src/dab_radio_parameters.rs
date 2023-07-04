use dab_core::dab_transmission_modes::DabTransmissionMode; 
use dab_core::dab_parameters::get_dab_parameters;

/// Parameters describing the digital audio broadcast (DAB) ensemble
/// 
/// # Common acronyms
/// | Acronym | Phrase | Description |
/// | ------- | ------ | ----------- |
/// | SYM | Orthogonal Frequency Division Multiplexing Symbol | An OFDM symbol consists of multiple complex symbols transmitted at different subcarrier frequencies at the same time. |
/// | FIC | Fast Information Channel | Carries metadata about the ensemble's structure including channel descriptons. |
/// | MSC | Main Service Channel | Carries radio data for the ensemble. This includes audio data for each channel and slideshows. |
/// | CIF | Common Interleaved Frame | The main service channel is transmitted as a series of interleaved frames that need to be deinterleaved. |
/// | FIB | Fast Information Block | The fast information channel is transmitted as groups of consecutive blocks. |
/// | FIG | Fast Information Group | The number of fast information blocks is divided into groups. The number of these groups is equal to the number of common interleaved frames |
/// 
/// # Diagram of DAB frame
/// This is the frame of a mode I transmission.
/// ```text
/// | Frame              |
/// | SYM*75             |
/// | SYM*3     | SYM*72 |
/// | FIC       | MSC    |
/// | FIG*4     | CIF*4  |
/// | [FIB*3]*4 | CIF*4  |
/// ```
#[derive(Debug, Default)]
pub struct DabRadioParameters {
    /// Number of symbols for each frame.
    pub nb_symbols: usize,
    /// Number of symbols for the fast information channel (FIC). This carries metadata about the ensemble.
    pub nb_fic_symbols: usize,
    /// Number of symbols for the main service channel (MSC). This carries radio data for each channel in the ensemble.
    pub nb_msc_symbols: usize,
    /// Number of fast information blocks (FIB) in the FIC
    pub nb_fibs_in_fic: usize,
    /// Number of common interleaved frames (CIF) in the MSC.
    pub nb_cifs_in_msc: usize,
    /// Number of bits per symbol
    pub nb_bits_per_symbol: usize,
    /// Number of bits in each frame.
    pub nb_bits_per_frame: usize,
    /// Number of bits in FIC.
    pub nb_bits_in_fic: usize,
    /// Number of bits in MSC.
    pub nb_bits_in_msc: usize,
    /// Number of bits per FIB
    pub nb_bits_per_fib: usize,
    /// Number of bits per FIG
    pub nb_bits_per_fig: usize,
    /// Number of bits per CIF
    pub nb_bits_per_cif: usize,
}

/// Returns useful parameters used in DAB digital decoding for a given transmission mode
pub fn get_dab_radio_parameters(transmission_mode: DabTransmissionMode) -> DabRadioParameters {
    let params = get_dab_parameters(transmission_mode);

    let bits_per_carrier = 2;
    let nb_symbols = params.nb_symbols-1;
    let nb_fic_symbols = params.nb_fic_symbols;
    let nb_msc_symbols = params.nb_msc_symbols;
    let nb_fibs_in_fic = params.nb_fibs_in_fic;
    let nb_cifs_in_msc = params.nb_cifs_in_msc;
    let nb_bits_per_symbol = params.nb_fft_data_carriers*bits_per_carrier;
    let nb_bits_per_frame = nb_bits_per_symbol*nb_symbols;
    let nb_bits_in_fic = nb_fic_symbols*nb_bits_per_symbol;
    let nb_bits_in_msc = nb_msc_symbols*nb_bits_per_symbol;
    let nb_bits_per_fib = nb_bits_in_fic/nb_fibs_in_fic;
    let nb_bits_per_fig = nb_bits_in_fic/nb_cifs_in_msc;
    let nb_bits_per_cif = nb_bits_in_msc/nb_cifs_in_msc;

    assert!(nb_symbols == (nb_fic_symbols + nb_msc_symbols), "Number of data symbols in frame doesn't match number of FIC and MSC symbols");
    assert!(nb_fibs_in_fic % nb_cifs_in_msc == 0, "The number of FIBs in the FIC must be a multiple of the number of CIFs in the MSC.");

    DabRadioParameters {
        nb_symbols,
        nb_fic_symbols,
        nb_msc_symbols,
        nb_fibs_in_fic,
        nb_cifs_in_msc,
        nb_bits_per_symbol,
        nb_bits_per_frame,
        nb_bits_in_fic,
        nb_bits_in_msc,
        nb_bits_per_fib,
        nb_bits_per_fig,
        nb_bits_per_cif,
    }
}

