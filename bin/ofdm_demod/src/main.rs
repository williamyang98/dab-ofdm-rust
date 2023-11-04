use app_helpers::gui_ofdm_demodulator::GuiOfdmDemodulator;
use app_helpers::barrier::Barrier; 
use ofdm::ofdm_demodulator::OfdmDemodulator;
use dab_core::dab_transmission_modes::DabTransmissionMode;
use std::io::{Read, Write, BufWriter};
use std::sync::{Arc, RwLock};
use num::complex::Complex32;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct AppArguments {
    /// DAB transmission mode. Valid modes are \[1,2,3,4\] 
    #[arg(short, long, default_value_t = 1)]
    mode: u32,
    /// Number of samples to read in chunks from input file
    #[arg(short, long, default_value_t = 4096*8)]
    number_of_input_samples: usize,
    /// Input filepath. If not provided uses stdin by default.
    #[arg(short, long)]
    input_filepath: Option<String>,
    /// Output filepath. If not provided uses stdout by default.
    #[arg(short, long)]
    output_filepath: Option<String>,
    /// Start the application without a GUI
    #[arg(long)]
    nogui: bool,
}

struct AppGui {
    ref_demodulator: Arc<RwLock<OfdmDemodulator>>,
    ui_demodulator: GuiOfdmDemodulator,
}

fn main() -> Result<(), String> {
    let args = AppArguments::parse();

    // Parse arguments
    let transmission_mode = match args.mode {
        1 => DabTransmissionMode::I,
        2 => DabTransmissionMode::II,
        3 => DabTransmissionMode::III,
        4 => DabTransmissionMode::IV,
        mode => return Err(format!("Invalid transmission mode index {}", mode)),
    };
    let number_of_input_samples = match args.number_of_input_samples {
        length if length == 0 => return Err("Number of input samples cannot be zero.".into()),
        length => length,
    };
    let mut input_file: Box<dyn Read + Send + Sync> = match &args.input_filepath {
        None => Box::new(std::io::stdin()),
        Some(filepath) => match std::fs::File::open(filepath) {
            Ok(file) => Box::new(file),
            Err(err) => return Err(format!("Failed to open input file {}: {}", filepath, err)),
        },
    };
    let mut output_file: Box<dyn Write + Send + Sync> = match &args.output_filepath {
        None => Box::new(BufWriter::new(std::io::stdout())),
        Some(filepath) => match std::fs::File::create(filepath) {
            Ok(file) => Box::new(BufWriter::new(file)),
            Err(err) => return Err(format!("Failed to open file {}: {}", filepath, err)),
        },
    };

    // Setup OFDM demodulator
    use dab_ofdm::dab_ofdm_carrier_map::get_dab_ofdm_carrier_map;
    use dab_ofdm::dab_ofdm_phase_reference_symbol::get_dab_ofdm_phase_reference_symbol_fft;
    use dab_ofdm::dab_ofdm_parameters::get_dab_ofdm_parameters;
    let ofdm_params = get_dab_ofdm_parameters(transmission_mode);
    let mut carrier_map = vec![0usize; ofdm_params.nb_fft_data_carriers];
    let mut prs_fft = vec![Complex32::default(); ofdm_params.nb_fft];
    get_dab_ofdm_carrier_map(&mut carrier_map, ofdm_params.nb_fft);
    get_dab_ofdm_phase_reference_symbol_fft(&mut prs_fft, transmission_mode);
    let ofdm_demodulator = Arc::new(RwLock::new(OfdmDemodulator::new(&ofdm_params, &carrier_map, &prs_fft)));

    // Setup input and output buffers
    let bytes_per_sample = 2;
    let mut input_bytes_buffer = vec![0u8; number_of_input_samples*bytes_per_sample];
    let mut input_samples_buffer = vec![Complex32::default(); number_of_input_samples];
    let intermediate_buffer = Arc::new(RwLock::new(vec![0i8; ofdm_params.nb_output_bits]));
    let intermediate_buffer_barrier = Arc::new(Barrier::new(false));

    // Setup threads
    let reader_thread = std::thread::spawn({
        let ofdm_demodulator = ofdm_demodulator.clone();
        let intermediate_buffer_barrier = intermediate_buffer_barrier.clone();
        move || {
            loop {
                let total_samples = match input_file.read(&mut input_bytes_buffer) {
                    Ok(0) => {
                        eprintln!("[reader_thread] Finished reading samples from input");
                        break;
                    },
                    Ok(length) => length/bytes_per_sample,
                    Err(err) => {
                        eprintln!("[reader_thread] Error while reading from input: {}", err);
                        break;
                    },
                };
                input_bytes_buffer[0..total_samples*bytes_per_sample]
                    .chunks_exact(bytes_per_sample)
                    .enumerate()
                    .for_each(|(i, x)| {
                    let dc_offset = 128.0;
                        input_samples_buffer[i].re = x[0] as f32 - dc_offset;
                        input_samples_buffer[i].im = x[1] as f32 - dc_offset;
                    });
                if let Err(err) = intermediate_buffer_barrier.wait(|is_full| !is_full) {
                    eprintln!("[reader_thread] Intermediate buffer stopped responding: {:?}", err);
                    break;
                }
                ofdm_demodulator.write().unwrap().process(&input_samples_buffer[..total_samples]);
            }
            if let Err(err) = intermediate_buffer_barrier.close() {
                eprintln!("[reader_thread] Error while closing intermediate buffer: {:?}", err);
            } else {
                eprintln!("[reader_thread] Successfully closed intermediate buffer");
            }
        }
    });

    // This callback is invoked through ofdm_demod.process(...) in the same thread
    ofdm_demodulator.write().unwrap().subscribe_bits_out({
        let intermediate_buffer = intermediate_buffer.clone();
        let intermediate_buffer_barrier = intermediate_buffer_barrier.clone();
        move |x: &[i8]| {
            let soft_bits = &mut *intermediate_buffer.write().unwrap();
            soft_bits.copy_from_slice(x);
            if let Err(err) = intermediate_buffer_barrier.set(true) {
                eprintln!("[reader_thread_bits_out] Intermediate buffer couldn't be updated: {:?}", err);
            }
        }
    });

    let writer_thread = std::thread::spawn({
        let intermediate_buffer = intermediate_buffer.clone();
        let intermediate_buffer_barrier = intermediate_buffer_barrier.clone();
        move || {
            loop {
                if let Err(err) = intermediate_buffer_barrier.wait(|is_full| *is_full) {
                    eprintln!("[writer_thread] Intermediate buffer stopped responding: {:?}", err);
                    break;
                }
                let soft_bits = &*intermediate_buffer.read().unwrap();
                let data_out = unsafe { 
                    std::slice::from_raw_parts(soft_bits.as_ptr() as *const u8, soft_bits.len()) 
                };
                if let Err(err) = output_file.write_all(data_out) {
                    eprintln!("[writer_thread] Error while writing to output: {}", err);
                    break;
                }
                if let Err(err) = intermediate_buffer_barrier.set(false) {
                    eprintln!("[writer_thread] Intermediate buffer couldn't be released: {:?}", err);
                    break;
                }
            }
            if let Err(err) = intermediate_buffer_barrier.close() {
                eprintln!("[writer_thread] Error while closing intermediate buffer: {:?}", err);
            } else {
                eprintln!("[writer_thread] Successfully closed intermediate buffer");
            }
        }
    });

    // Handle closing
    if !args.nogui {
        if let Err(err) = launch_gui(ofdm_demodulator.clone()) {
            eprintln!("[main_thread] Error while running gui: {}", err);
        }
        if let Err(err) = intermediate_buffer_barrier.close() {
            eprintln!("[main_thread] Error while closing intermediate buffer: {:?}", err);
        } else {
            eprintln!("[main_thread] Successfully closed intermediate buffer");
        }
    }
    if let Err(err) = reader_thread.join() {
        eprintln!("[main_thread] Reader thread should terminate gracefully: {:?}", err);
    };
    if let Err(err) = writer_thread.join() {
        eprintln!("[main_thread] Writer thread should terminate gracefully: {:?}", err);
    }
    Ok(())
}

fn launch_gui(demod: Arc<RwLock<OfdmDemodulator>>) -> Result<(), eframe::Error> {
    let app_name = "DAB OFDM Demodulator";
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::Vec2::new(500.0, 900.0)),
        ..Default::default()
    };

    let app_gui = AppGui {
        ref_demodulator: demod,
        ui_demodulator: GuiOfdmDemodulator::default(),
    };

    eframe::run_native(
        app_name,
        native_options,
        Box::new(move |_cc| Box::new(app_gui)),
    )
}

impl eframe::App for AppGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let demod = &mut *self.ref_demodulator.write().unwrap();
            self.ui_demodulator.draw_all(demod, ui);
        });
    }
}