use ofdm::ofdm_parameters::OfdmParameters;
use dab_core::dab_transmission_modes::DabTransmissionMode; 
use dab_core::dab_parameters::get_dab_parameters;

/// The OFDM parameters associated for each transmission mode for DAB radio.
pub fn get_dab_ofdm_parameters(transmission_mode: DabTransmissionMode) -> OfdmParameters {
    let params = get_dab_parameters(transmission_mode);
    OfdmParameters::new(
        params.nb_symbols,
        params.nb_null_period,
        params.nb_symbol_period,
        params.nb_fft,
        params.nb_fft_data_carriers,
    )
}