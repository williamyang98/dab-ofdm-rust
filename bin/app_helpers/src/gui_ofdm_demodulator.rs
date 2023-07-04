use ofdm::ofdm_demodulator::OfdmDemodulator;
use egui::Color32;
use egui::plot::VLine;
use egui::plot::{Plot, PlotPoints, Line, LineStyle, Corner, CoordinatesFormatter, Legend, Points};

#[derive(PartialEq, Eq)]
enum SelectedPlot {
    None,
    NullPrs,
    FineTimeImpulseResponse,
    CoarseFrequencyImpulseResponse,
    DqpskConstellation,
    BitsConstellation,
}

/// Renders a OFDM demodulator.
pub struct GuiOfdmDemodulator {
    selected_dqpsk_symbol: usize,
    selected_plot: SelectedPlot,
}

impl Default for GuiOfdmDemodulator {
    fn default() -> Self {
        Self {
            selected_dqpsk_symbol: 0,
            selected_plot: SelectedPlot::DqpskConstellation,
        }
    }
}

impl GuiOfdmDemodulator {
    /// Draws everything in demodulator.
    pub fn draw_all(&mut self, demod: &mut OfdmDemodulator, ui: &mut egui::Ui) {
        ui.heading("DAB OFDM Demodulator");
        ui.separator();
        self.draw_state(demod, ui);
        ui.separator();
        self.draw_controls(demod, ui);
        ui.separator();
        self.draw_plots(demod, ui);
    }

    /// Draws current state of demodulator.
    pub fn draw_state(&self, demod: &OfdmDemodulator, ui: &mut egui::Ui) {
        let net_frequency_offset = demod.coarse_frequency_offset + demod.fine_frequency_offset;
        let sample_rate: f32 = 2.048e6;

        egui::Grid::new("Statistics")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                let mut create_label = |label: &str, description: String| {
                    ui.strong(label);
                    ui.label(description);
                    ui.end_row();
                };

                create_label("State", format!("{:?}", demod.state));
                create_label("Total frames read", format!("{}", demod.total_frames_read));
                create_label("Total frames desync", format!("{}", demod.total_frames_desync));
                create_label("Fine frequency offset", format!("{:.2}", demod.fine_frequency_offset * sample_rate));
                create_label("Coarse frequency offset", format!("{:.2}", demod.coarse_frequency_offset * sample_rate));
                create_label("Net frequency offset", format!("{:.2}", net_frequency_offset * sample_rate));
                create_label("Fine time offset", format!("{}", demod.fine_time_offset));
                create_label("Signal L1 average", format!("{}", demod.signal_l1_average));
            });
    }

    /// Draws controls for demodulator.
    pub fn draw_controls(&self, demod: &mut OfdmDemodulator, ui: &mut egui::Ui) {
        let settings = &mut demod.settings;
        ui.add(egui::Slider::new(&mut settings.null_power_threshold_start, 0.0..=settings.null_power_threshold_end).text("Null threshold start"));
        ui.add(egui::Slider::new(&mut settings.null_power_threshold_end, settings.null_power_threshold_start..=1.0).text("Null threshold end"));
        ui.add(egui::Slider::new(&mut settings.null_power_update_beta, 0.0..=1.0).text("Null power update beta"));
        ui.add(egui::Slider::new(&mut settings.fine_frequency_update_beta, 0.0..=1.0).text("Fine frequency update beta"));
        ui.add(egui::Slider::new(&mut settings.coarse_frequency_slow_update_beta, 0.0..=1.0).text("Coarse frequency update beta"));
        ui.add(egui::Slider::new(&mut settings.coarse_frequency_max_range, 0.0..=0.95).text("Coarse frequency max range"));
        ui.add(egui::Slider::new(&mut settings.fine_time_impulse_peak_threshold_db, 0.0..=100.0).text("Fine time impulse peak threshold dB"));
        ui.add(egui::Slider::new(&mut settings.fine_time_impulse_peak_distance_probability, 0.0..=1.0).text("Fine time impulse peak distance probability"));
    }

    /// Draws selected plot of some internal buffer for the demodulator.
    pub fn draw_plots(&mut self, demod: &mut OfdmDemodulator, ui: &mut egui::Ui) {
        let params = &demod.params;

        ui.horizontal(|ui| {
            let mut create_button = |value: SelectedPlot, text: &'static str| {
                let is_selected = self.selected_plot == value;
                if ui.selectable_label(is_selected, text).clicked() {
                    if is_selected {
                        self.selected_plot = SelectedPlot::None;
                    } else {
                        self.selected_plot = value;
                    }
                }
            };
            create_button(SelectedPlot::NullPrs, "NULL PRS");
            create_button(SelectedPlot::CoarseFrequencyImpulseResponse, "Coarse frequency");
            create_button(SelectedPlot::FineTimeImpulseResponse, "Fine time");
            create_button(SelectedPlot::DqpskConstellation, "DQPSK constellation");
            create_button(SelectedPlot::BitsConstellation, "Bits");
        });

        if self.selected_plot != SelectedPlot::None {
            ui.ctx().request_repaint();
        }

        match self.selected_plot {
            SelectedPlot::None => (),
            SelectedPlot::NullPrs => {
                let buffer = demod.null_prs_buffer.raw_slice();

                let real_points: PlotPoints = buffer
                    .iter()
                    .enumerate()
                    .map(|(x,y)| [ x as f64, y.re as f64 ])
                    .collect();
                let real_line = Line::new(real_points);

                let imag_points: PlotPoints = buffer
                    .iter()
                    .enumerate()
                    .map(|(x,y)| [ x as f64, y.im as f64 ])
                    .collect();
                let imag_line = Line::new(imag_points);

                let null_prs_line = VLine::new(params.nb_null_period as f64);

                Plot::new("NULL + PRS buffer")
                    .legend(Legend::default())
                    .coordinates_formatter(Corner::LeftBottom, CoordinatesFormatter::default())
                    .show(ui, |plot_ui| {
                        plot_ui.line(real_line);
                        plot_ui.line(imag_line);
                        plot_ui.vline(null_prs_line);
                    });
            },
            SelectedPlot::CoarseFrequencyImpulseResponse => {
                let plot_points: PlotPoints = demod.coarse_frequency_impulse_response_buffer
                    .iter()
                    .enumerate()
                    .map(|(x,y)| [ x as f64, *y as f64 ])
                    .collect();
                let plot_line = Line::new(plot_points)
                    .style(LineStyle::Solid);
                
                let settings = &demod.settings;
                let freq_center = params.nb_fft as f64 / 2.0;
                let freq_lock_on_width = (settings.coarse_frequency_max_range * params.nb_fft as f32) as f64;
                let freq_offset = demod.coarse_frequency_offset as f64 * params.nb_fft as f64;

                let vline_freq_center = VLine::new(freq_center);
                let vline_freq_offset = VLine::new(freq_center - freq_offset);
                let vline_freq_left  = VLine::new(freq_center - freq_lock_on_width/2.0).color(Color32::DARK_BLUE);
                let vline_freq_right = VLine::new(freq_center + freq_lock_on_width/2.0).color(Color32::DARK_BLUE);

                Plot::new("Coarse frequency response")
                    .legend(Legend::default())
                    .coordinates_formatter(Corner::LeftBottom, CoordinatesFormatter::default())
                    .show(ui, |plot_ui| {
                        plot_ui.line(plot_line);
                        plot_ui.vline(vline_freq_center);
                        plot_ui.vline(vline_freq_offset);
                        plot_ui.vline(vline_freq_left);
                        plot_ui.vline(vline_freq_right);
                    });
            },
            SelectedPlot::FineTimeImpulseResponse => {
                let plot_points: PlotPoints = demod.fine_time_impulse_response_buffer
                    .iter()
                    .enumerate()
                    .map(|(x,y)| [ x as f64, *y as f64 ])
                    .collect();
                let plot_line = Line::new(plot_points);

                let time_center = params.nb_cyclic_prefix as f64;
                let time_offset = demod.fine_time_offset as f64;

                let vline_time_center = VLine::new(time_center);
                let vline_time_offset = VLine::new(time_center + time_offset);

                Plot::new("Fine time impulse response")
                    .legend(Legend::default())
                    .coordinates_formatter(Corner::LeftBottom, CoordinatesFormatter::default())
                    .show(ui, |plot_ui| {
                        plot_ui.line(plot_line);
                        plot_ui.vline(vline_time_center);
                        plot_ui.vline(vline_time_offset);
                    });
            },
            SelectedPlot::DqpskConstellation => {
                let buffer = &demod.data_dqpsk_buffer;

                let total_symbols = params.nb_symbols-1;
                let length = params.nb_fft_data_carriers;
                let i = self.selected_dqpsk_symbol;
                let data = &buffer[i*length..(i+1)*length];

                let points: PlotPoints = data 
                    .iter()
                    .map(|x| [ x.im as f64, x.re as f64 ])
                    .collect();

                let markers = Points::new(points)
                    .name("DQPSK");

                ui.add(
                    egui::widgets::Slider::new(
                        &mut self.selected_dqpsk_symbol, 
                        0..=total_symbols-1)
                        .text("DQPSK Symbol"));
                Plot::new("DQPSK symbols")
                    .legend(Legend::default())
                    .coordinates_formatter(Corner::LeftBottom, CoordinatesFormatter::default())
                    .data_aspect(1.0)
                    .show(ui, |plot_ui| {
                        plot_ui.points(markers);
                    });
            },
            SelectedPlot::BitsConstellation => {
                let buffer = &demod.data_out_bits_buffer;

                let total_symbols = params.nb_symbols-1;
                let i = self.selected_dqpsk_symbol;
                let length = params.nb_fft_data_carriers*2;
                let data = &buffer[i*length..(i+1)*length];
                let real_data = &data[0..params.nb_fft_data_carriers];
                let imag_data = &data[params.nb_fft_data_carriers..];

                let points: PlotPoints = (0..params.nb_fft_data_carriers)
                    .map(|i| [ real_data[i] as f64, imag_data[i] as f64 ] )
                    .collect();

                let markers = Points::new(points)
                    .name("Viterbi bits");

                ui.add(
                    egui::widgets::Slider::new(
                        &mut self.selected_dqpsk_symbol, 
                        0..=total_symbols-1)
                        .text("DQPSK Symbol"));

                Plot::new("Viterbi bits")
                    .legend(Legend::default())
                    .coordinates_formatter(Corner::LeftBottom, CoordinatesFormatter::default())
                    .data_aspect(1.0)
                    .show(ui, |plot_ui| {
                        plot_ui.points(markers);
                    });
            },
        };
    }
}
