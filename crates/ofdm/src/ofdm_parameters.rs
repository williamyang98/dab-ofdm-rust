/// OFDM is orthogonal frequency division multiplexing.
/// Describes the structure of an OFDM frame.
/// Frame consists of one NULL symbol and N data symbols.
/// The phase reference symbol (PRS) is the first data symbol.
/// 
/// # Diagram
/// ```text
/// | Frame                  |
/// | NULL | SYM*N           |
/// | NULL | PRS | SYM*(N-1) |
/// ```
/// 
/// After demodulation using differential quadrature phase shift keying, we end up with N-1 data symbols.
#[derive(Debug, Clone, Copy)]
pub struct OfdmParameters {
    /// Number of OFDM data symbols in a transmission frame.
    pub nb_symbols: usize,
    /// Duration of NULL symbol.
    pub nb_null_period: usize,
    /// Duration of OFDM data symbol.
    pub nb_symbol_period: usize,
    /// Duration of cyclic prefix in OFDM data symbol.
    pub nb_cyclic_prefix: usize,
    /// Duration of FFT in OFDM data symbol.
    pub nb_fft: usize,
    /// Number of FFT bins that are data carriers centered around DC.
    pub nb_fft_data_carriers: usize,
    /// Number of different QPSK (quadrature phase shift key) symbols.
    pub nb_dqpsk_symbols: usize,
    /// Number of output samples, where each sample is a complex number.
    pub nb_output_samples: usize,
    /// Number of output soft decision bits.
    pub nb_output_bits: usize,
    /// Number of complex samples for the entire OFDM frame.
    pub nb_input_samples: usize,
}

impl OfdmParameters {
    /// Creates all derived parameters for OFDM from a required subset.
    pub fn new(
        nb_symbols: usize,
        nb_null_period: usize,
        nb_symbol_period: usize,
        nb_fft: usize,
        nb_fft_data_carriers: usize,
    ) -> Self 
    {
        assert!(nb_symbols >= 2, "Number of symbols must be at least 2 due to differential QPSK encoding");
        assert!(nb_symbol_period >= nb_fft, "Number of samples in symbol is less than FFT resolution");
        assert!(nb_fft >= nb_fft_data_carriers, "Number of data carriers is limited to FFT resolution");

        let nb_input_samples = nb_null_period + nb_symbol_period*nb_symbols;
        let nb_cyclic_prefix = nb_symbol_period - nb_fft;
        let nb_dqpsk_symbols = nb_symbols-1;
        let nb_output_samples = nb_dqpsk_symbols*nb_fft_data_carriers;
        let nb_output_bits = nb_output_samples*2;

        Self {
            nb_symbols,
            nb_null_period,
            nb_symbol_period,
            nb_fft,
            nb_fft_data_carriers,
            nb_cyclic_prefix,
            nb_dqpsk_symbols,
            nb_output_samples,
            nb_output_bits,
            nb_input_samples,
        }
    }

}