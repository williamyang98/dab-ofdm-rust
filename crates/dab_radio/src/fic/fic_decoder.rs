use crate::dab_radio_parameters::DabRadioParameters;

pub struct FicDecoder {
    params: DabRadioParameters,
}

impl FicDecoder {
    pub fn decode_fic(&mut self, buf: &[i8]) {
        assert!(buf.len() == self.params.nb_bits_in_fic);
        for fig in buf.chunks_exact(self.params.nb_bits_per_fig) {
            self.decode_fig(fig);
        }
    }

    fn decode_fig(&mut self, buf: &[i8]) {
        assert!(buf.len() == self.params.nb_bits_per_fig);
    }
}