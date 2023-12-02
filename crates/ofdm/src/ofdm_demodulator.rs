use crate::ofdm_parameters::OfdmParameters;
use crate::circular_bucket::CircularBucket;
use crate::linear_bucket::LinearBucket;
use std::sync::Arc;
use std::cmp::Ordering;
use num::complex::Complex32;
use rustfft::{FftPlanner, Fft};
use itertools::izip;

#[derive(Debug)]
pub struct OfdmDemodulatorSettings {
    /// The rate at which to update the L1 power average of the signal. 
    /// This is a number from 0 to 1 where 1 is the fastest update rate.
    pub null_power_update_beta: f32,
    /// The number of samples in a block to calculate the L1 power average
    pub null_power_total_samples: usize,
    /// The number of blocks we stride where we only analyse one block.
    pub null_power_decimation_factor: usize,
    /// The amount of the L1 power average that the signal needs to fall below to detect the start of the NULL symbol.
    pub null_power_threshold_start: f32,
    /// The amount of the L1 power average that the signal needs to rise above to detect the end of the NULL symbol.
    pub null_power_threshold_end: f32,
    /// The rate to update the fine frequency offset during each OFDM frame. 
    /// Fine frequency offsets are smaller than the frequency spacing of one FFT bin.
    /// This is a number from 0 to 1 where 1 is the fastest update rate.
    pub fine_frequency_update_beta: f32,
    /// Whether we perform coarse frequency correction. 
    /// Coarse frequency offsets are larger than the frequency spacing of one FFT bin.
    pub coarse_frequency_is_enabled: bool,
    /// The maximum coarse frequency offset the coarse frequency correction step should search for. 
    /// This is a number from 0 to 1 where 1 is normalised to half the sampling frequency.
    pub coarse_frequency_max_range: f32,
    /// The rate to update the coarse frequency offset during each OFDM frame.
    /// This is only used when the coarse frequency offset changes in small amounts for after a stable period.
    /// This is a number from 0 to 1 where 1 is the fastest update rate.
    pub coarse_frequency_slow_update_beta: f32,
    /// During fine time correction we generate an impulse response, where the highest peak is considered the start of our phase reference symbol (PRS).
    /// This is the required height for the impulse peak to be considered valid as the start of the PRS.
    pub fine_time_impulse_peak_threshold_db: f32,
    /// This is the amount to weigh the height of the impulse peak based on its distance from the expected location.
    /// We assume that after the NULL symbol detection step that the PRS will be situated roughly in the correct position.
    /// Therefore to prevent spurious locks onto peaks that are far away from the expected position due to noise, we lower the perceived height of the peak the further away it is.
    pub fine_time_impulse_peak_distance_probability: f32,
}

impl Default for OfdmDemodulatorSettings {
    fn default() -> Self {
        Self {
            null_power_update_beta: 0.95,
            null_power_total_samples: 100,
            null_power_decimation_factor: 5,
            null_power_threshold_start: 0.35,
            null_power_threshold_end: 0.75,
            fine_frequency_update_beta: 0.95,
            coarse_frequency_is_enabled: true,
            coarse_frequency_max_range: 0.1, 
            coarse_frequency_slow_update_beta: 0.1,
            fine_time_impulse_peak_threshold_db: 20.0,
            fine_time_impulse_peak_distance_probability: 0.15,
        }
    }
}

#[derive(Debug)]
pub enum OfdmDemodulatorState {
    /// Finding the NULL symbol by analysing the average L1 power of blocks in the signal
    FindingNullPowerDip,
    /// Once the NULL symbol has been detected we read the NULL and PRS symbol
    ReadingNullAndPrs,
    /// Compensating for large frequency offsets that are greater than one FFT bin
    RunningCoarseFrequencySynchronisation,
    /// Compensating for sample offsets where we detected our NULL and PRS symbols. 
    /// This step can fail if the impulse peak is too weak or too far away from our expected location.
    /// When this occurs the demodulator will go back to finding the NULL symbol through L1 power analysis.
    RunningFineTimeSync,
    /// Once the NULL and PRS symbol have been read we read in the rest of the OFDM frame.
    ReadingSymbols,
    /// Once the OFDM frame has been read we process the symbols.
    /// This includes performing DQPSK demodulation, fine frequency compensation and data carrier remapping.
    ProcessingSymbols,
}

pub struct OfdmDemodulator {
    pub state: OfdmDemodulatorState,
    pub settings: OfdmDemodulatorSettings,
    pub params: OfdmParameters,
    /// The number of OFDM frames read successfully.
    pub total_frames_read: u32,
    /// The number of OFDM frames that desynced if the detected NULL and PRS symbols are too offset in time. 
    pub total_frames_desync: u32,
    is_found_coarse_frequency_offset: bool,
    /// The current coarse frequency offset normalised to the sampling frequency.
    pub coarse_frequency_offset: f32,
    /// The current fine frequency offset normalised to the sampling frequency.
    pub fine_frequency_offset: f32,
    /// The number of samples the incoming OFDM frame is offset by in time.
    pub fine_time_offset: isize,
    is_null_start_found: bool,
    is_null_end_found: bool,
    /// The current L1 signal average of the receiving signal.
    pub signal_l1_average: f32,
    // fft
    fft: Arc<dyn Fft<f32>>,
    ifft: Arc<dyn Fft<f32>>,
    temp_fft_buffer: Vec<Complex32>,
    // reference data
    carrier_mapper_data: Vec<usize>,
    correlation_prs_fft_data: Vec<Complex32>,
    correlation_prs_time_data: Vec<Complex32>,
    // buffers
    null_power_dip_buffer: CircularBucket<Complex32>,
    /// The buffer that holds the current predicted NULL and PRS symbols.
    pub null_prs_buffer: LinearBucket<Complex32>,
    /// The buffer that holds the fine time impulse response buffer. 
    /// There should be one dominant peak and many small sidelobes since this is the output of correlation in time.
    pub fine_time_impulse_response_buffer: Vec<f32>,
    /// The buffer that holds the coarse frequency impulse response buffer.
    /// There should be multiple peaks with the largest peak indicating the coarse frequency offset.
    /// The spacing between each sample indicates a frequency different of one FFT bin.
    pub coarse_frequency_impulse_response_buffer: Vec<f32>,
    data_time_buffer: LinearBucket<Complex32>,
    data_fft_buffer: Vec<Complex32>,
    /// The buffer that holds the constellations of DQPSK complex symbols for each data symbol.
    pub data_dqpsk_buffer: Vec<Complex32>,
    /// The buffer that holds the soft decision bits outputted for each data symbol after carrier remapping.
    pub data_out_bits_buffer: Vec<i8>,
    bits_out_callbacks: Vec<Box<dyn FnMut(&[i8]) + Send + Sync + 'static>>,
}

impl OfdmDemodulator {
    pub fn new(params: &OfdmParameters, carrier_mapper: &[usize], prs_fft: &[Complex32]) -> Self {
        assert!(params.nb_fft_data_carriers == carrier_mapper.len(), "Mismatching number of data carriers between params {} and lookup table {}", params.nb_fft_data_carriers, carrier_mapper.len());
        assert!(params.nb_fft == prs_fft.len(), "Mismatching FFT size between params {} and FFT buffer {}", params.nb_fft, prs_fft.len());

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(params.nb_fft);
        let ifft = planner.plan_fft_inverse(params.nb_fft);

        let mut demodulator = Self {
            state: OfdmDemodulatorState::FindingNullPowerDip,
            settings: OfdmDemodulatorSettings::default(),
            params: *params,
            // initial state
            total_frames_read: 0,
            total_frames_desync: 0,
            is_found_coarse_frequency_offset: false,
            coarse_frequency_offset: 0.0,
            fine_frequency_offset: 0.0,
            fine_time_offset: 0,
            is_null_start_found: false,
            is_null_end_found: false,
            signal_l1_average: 0.0,
            // fft
            fft,
            ifft,
            // data
            carrier_mapper_data: carrier_mapper.to_vec(),
            correlation_prs_fft_data: vec![Complex32::default(); params.nb_fft],
            correlation_prs_time_data: vec![Complex32::default(); params.nb_fft],
            // buffer
            null_power_dip_buffer: CircularBucket::<Complex32>::new(params.nb_null_period),
            null_prs_buffer: LinearBucket::<Complex32>::new(params.nb_null_period + params.nb_symbol_period),
            fine_time_impulse_response_buffer: vec![0.0; params.nb_fft],
            coarse_frequency_impulse_response_buffer: vec![0.0; params.nb_fft],
            temp_fft_buffer: vec![Complex32::default(); params.nb_fft],
            data_time_buffer: LinearBucket::<Complex32>::new(params.nb_input_samples),
            data_fft_buffer: vec![Complex32::default(); params.nb_symbols*params.nb_fft],
            data_dqpsk_buffer: vec![Complex32::default(); params.nb_output_samples],
            data_out_bits_buffer: vec![0i8; params.nb_output_bits],
            // callbacks
            bits_out_callbacks: vec![],
        };

        demodulator.init(prs_fft);
        demodulator
    }

    fn init(&mut self, prs_fft: &[Complex32]) {
        assert!(prs_fft.len() == self.params.nb_fft, "PRS FFT must have {} samples but got {} samples", self.params.nb_fft, prs_fft.len());

        self.correlation_prs_time_data.copy_from_slice(prs_fft);
        calculate_relative_phase(&mut self.correlation_prs_time_data);
        self.ifft.process(&mut self.correlation_prs_time_data);

        // Correlation in either time or frequency domain requires the conjugate product in the opposite domain
        // Used in coarse frequency correction
        for value in &mut self.correlation_prs_time_data {
            *value = value.conj();
        }
        // Used in fine time correction
        for i in 0..self.params.nb_fft {
            self.correlation_prs_fft_data[i] = prs_fft[i].conj();
        }
    }

    /// Registers a callback when the OFDM demodulator has successfully produced the output bits for a signal OFDM frame.
    /// Returns the soft decision bits as an array of signed 8bit value between -127 and +127.
    pub fn subscribe_bits_out(&mut self, callback: impl FnMut(&[i8]) + Send + Sync + 'static) {
        self.bits_out_callbacks.push(Box::new(callback));
    }

    /// Consumes an array of complex samples from the receiver and passes it through the demodulator.
    pub fn process(&mut self, buf: &[Complex32]) {
        self.update_signal_power_average(buf);

        let mut curr_buf = buf;
        while !curr_buf.is_empty() {
            let total_read = match self.state {
                OfdmDemodulatorState::FindingNullPowerDip                   =>   self.find_null_power_dip(curr_buf),
                OfdmDemodulatorState::ReadingNullAndPrs                     =>   self.read_null_prs(curr_buf),
                OfdmDemodulatorState::RunningCoarseFrequencySynchronisation => { self.run_coarse_frequency_synchronisation(); 0 },
                OfdmDemodulatorState::RunningFineTimeSync                   => { self.run_fine_time_sync(); 0 },
                OfdmDemodulatorState::ReadingSymbols                        =>   self.read_symbols(curr_buf),
                OfdmDemodulatorState::ProcessingSymbols                     => { self.process_symbols(); 0 },
            };
            curr_buf = &curr_buf[total_read..];
        }
    }

    fn reset_from_desync(&mut self) {
        self.state = OfdmDemodulatorState::FindingNullPowerDip;
        self.null_prs_buffer.reset();

        // NOTE: We also reset fine frequency synchronisation since an incorrect value
        // can reduce performance of fine time synchronisation using the impulse response
        self.signal_l1_average = 0.0;
        self.is_found_coarse_frequency_offset = false;
        self.fine_frequency_offset = 0.0;
        self.coarse_frequency_offset = 0.0;
        self.fine_time_offset = 0;
    }

    fn find_null_power_dip(&mut self, buf: &[Complex32]) -> usize {
        // Clause 3.12.2 - Frame synchronisation using power detection
        // we run this if we dont have an initial estimate for the prs index
        // This can occur if:
        //      1. We just started the demodulator and need a quick estimate of OFDM start
        //      2. The PRS impulse response didn't have a sufficiently large peak

        let null_start_threshold = self.signal_l1_average * self.settings.null_power_threshold_start;
        let null_end_threshold   = self.signal_l1_average * self.settings.null_power_threshold_end;

        // We analyse the average power of the signal in blocks
        let block_size = self.settings.null_power_total_samples;
        let mut total_read = 0;
        for block in buf.chunks_exact(block_size) {
            let l1_average = calculate_l1_average(block);
            total_read += block_size;
            if self.is_null_start_found {
                if l1_average > null_end_threshold {
                    self.is_null_end_found = true;
                    break;
                }
            } else {
                if l1_average < null_start_threshold {
                    self.is_null_start_found = true;
                }
            }
        }

        // We ignore the remaining buffer until there are enough samples for analysis
        if !self.is_null_end_found {
            self.null_power_dip_buffer.consume(buf, true);
            return buf.len();
        }

        // Copy null symbol into correlation buffer
        // This is done since our captured null symbol may actually contain parts of the PRS 
        // We do this so we can guarantee the full start of the PRS is attained after fine time sync
        let consumed_blocks = &buf[..total_read];
        self.null_power_dip_buffer.consume(consumed_blocks, true);
        self.null_prs_buffer.reset();
        self.null_prs_buffer.consume_from_iterator(
            self.null_power_dip_buffer.iter().copied()
        );


        self.is_null_start_found = false;
        self.is_null_end_found = false;
        self.null_power_dip_buffer.reset();
        self.state = OfdmDemodulatorState::ReadingNullAndPrs;

        total_read
    }

    fn read_null_prs(&mut self, buf: &[Complex32]) -> usize {
        let total_read = self.null_prs_buffer.consume(buf);
        if self.null_prs_buffer.is_full() {
            self.state = OfdmDemodulatorState::RunningCoarseFrequencySynchronisation;
        }
        total_read
    }

    fn run_coarse_frequency_synchronisation(&mut self) {
        // Clause: 3.13.2 Integral frequency offset estimation
        if !self.settings.coarse_frequency_is_enabled {
            self.coarse_frequency_offset = 0.0;
            self.state = OfdmDemodulatorState::RunningFineTimeSync;
            return;
        }

        let prs = &self.null_prs_buffer[span_slice(self.params.nb_null_period, self.params.nb_symbol_period)];
        let prs_fft = &prs[self.params.nb_cyclic_prefix..];

        // To mitigate effect of phase shifts we instead correlate the complex difference between consecutive FFT bins
        // arg(~z0*z1) = arg(z1)-arg(z0)
        self.temp_fft_buffer.copy_from_slice(prs_fft);
        self.fft.process(&mut self.temp_fft_buffer);
        calculate_relative_phase(&mut self.temp_fft_buffer);
        self.ifft.process(&mut self.temp_fft_buffer);

        // Correlation in frequency domain is multiplication in time domain
        // NOTE: PRS time data is already conjugate in self.init()
        for (x,y) in izip!(
            self.correlation_prs_time_data.iter().take(self.params.nb_fft), 
            self.temp_fft_buffer.iter_mut().take(self.params.nb_fft),
        ) {
            *y *= *x;
        }
        self.fft.process(&mut self.temp_fft_buffer);
        calculate_magnitude_spectrum(&self.temp_fft_buffer, &mut self.coarse_frequency_impulse_response_buffer);

        assert!(self.settings.coarse_frequency_max_range < 1.0);
        let dc_bin = (self.params.nb_fft/2) as i32;
        let max_carrier_offset_bins = (0.5 * self.settings.coarse_frequency_max_range * self.params.nb_fft as f32).floor() as i32;
        let carrier_offset_bin = (-max_carrier_offset_bins..=max_carrier_offset_bins)
            .map(|offset| {
                let fft_bin = offset+dc_bin;
                let value: f32 = self.coarse_frequency_impulse_response_buffer[fft_bin as usize];
                (offset, value)
            })
            .max_by(|(_,x), (_,y)| {
                if x > y {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            })
            .map(|(offset,_)| offset)
            .unwrap_or(0);

        let current_coarse_frequency_offset: f32 = (-carrier_offset_bin as f32) / (self.params.nb_fft as f32);
        let delta_coarse_frequency_offset = current_coarse_frequency_offset - self.coarse_frequency_offset;
        
        let large_offset_bin: f32 = 1.5;
        let large_offset_threshold = large_offset_bin/(self.params.nb_fft as f32);
        let is_large_offset = delta_coarse_frequency_offset.abs() > large_offset_threshold;

        let is_fast_update = is_large_offset || !self.is_found_coarse_frequency_offset;
        let update_beta: f32 = match is_fast_update { 
            true => 1.0, 
            false => self.settings.coarse_frequency_slow_update_beta,
        };
        let delta = update_beta*delta_coarse_frequency_offset;

        self.is_found_coarse_frequency_offset = true;
        self.coarse_frequency_offset += delta;
        self.update_fine_frequency_offset(-delta);
        self.state = OfdmDemodulatorState::RunningFineTimeSync;
    }

    fn run_fine_time_sync(&mut self) {
        let prs_data = &self.null_prs_buffer[span_slice(self.params.nb_null_period, self.params.nb_fft)];

        let total_frequency_offset = self.coarse_frequency_offset + self.fine_frequency_offset;
        self.temp_fft_buffer.copy_from_slice(prs_data);
        apply_pll(&mut self.temp_fft_buffer, total_frequency_offset);

        // Perform impulse correlation in time domain using multiplication in frequency domain
        // NOTE: Our PRS FFT reference was conjugated in self.init()
        self.fft.process(&mut self.temp_fft_buffer);
        for (x,y) in izip!(
            self.correlation_prs_fft_data.iter().take(self.params.nb_fft), 
            self.temp_fft_buffer.iter_mut().take(self.params.nb_fft),
        ) {
            *y *= *x;
        }
        self.ifft.process(&mut self.temp_fft_buffer);
        for (x,y) in izip!(
            self.temp_fft_buffer.iter().take(self.params.nb_fft),
            self.fine_time_impulse_response_buffer.iter_mut().take(self.params.nb_fft),
        ) {
            let amplitude = x.norm().log10() * 20.0;
            *y = amplitude;
        }

        let (impulse_peak_index, impulse_peak_value) = self.fine_time_impulse_response_buffer
            .iter()
            .enumerate()
            .map(|(i, peak_value)| {
                // We expect that the correlation peak will at least be somewhere near where we expect it
                // When we are still locking on, the impulse response may have many peaks due to frequency offsets
                // This causes spurious desyncs when one of these other peaks are very far away
                // Thus we weigh the value of the peak with its distance from the expected location
                let expected_peak_x = self.params.nb_cyclic_prefix;
                let distance_from_expectation = (expected_peak_x as i32 - i as i32).abs();
                let norm_distance = (distance_from_expectation as f32) / (self.params.nb_symbol_period as f32);
                let decay_weight = 1.0 - self.settings.fine_time_impulse_peak_distance_probability;
                let probability = 1.0 - decay_weight * norm_distance;
                let weighted_peak_value = probability*peak_value;
                (i, weighted_peak_value)
            })
            .max_by(|(_, x),(_, y)| {
                if x > y {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            })
            .expect("The fine time impulse buffer cannot be empty");
    
        let impulse_sum: f32 = self.fine_time_impulse_response_buffer
            .iter()
            .sum();
        let impulse_average = impulse_sum / (self.params.nb_fft as f32);

        // If the main lobe is insufficiently powerful we do not have a valid impulse response
        // This probably means we had a severe desync and should restart 
        let impulse_peak_height = impulse_peak_value - impulse_average;
        if impulse_peak_height < self.settings.fine_time_impulse_peak_threshold_db {
            self.reset_from_desync();
            self.total_frames_desync += 1;
            return;
        }

        // | [NULL] | [Cyclic prefix] | [PRS FFT]
        // The PRS correlation lobe occurs just after the cyclic prefix
        // We actually want the index at the start of the cyclic prefix, so we adjust offset for that
        let prs_start_offset = impulse_peak_index as isize - self.params.nb_cyclic_prefix as isize;
        let prs_start_index = isize::max(self.params.nb_null_period as isize + prs_start_offset, 0) as usize;
        let prs_length = isize::max(self.params.nb_symbol_period as isize - prs_start_offset, 0) as usize;
        let prs_partial_buffer = &self.null_prs_buffer[span_slice(prs_start_index, prs_length)];
        
        self.data_time_buffer.reset();
        self.data_time_buffer.consume(prs_partial_buffer);

        self.null_prs_buffer.reset();
        self.fine_time_offset = prs_start_offset;
        self.state = OfdmDemodulatorState::ReadingSymbols;
    }

    fn read_symbols(&mut self, buf: &[Complex32]) -> usize {
        let total_read = self.data_time_buffer.consume(buf);
        if self.data_time_buffer.is_full() {
            self.state = OfdmDemodulatorState::ProcessingSymbols;
        }
        total_read
    }

    fn process_symbols(&mut self) {
        // Copy the null symbol so we can use it in find_null_prs
        let null_symbol_offset = self.params.nb_symbols*self.params.nb_symbol_period;
        let null_symbol = &self.data_time_buffer[span_slice(null_symbol_offset, self.params.nb_null_period)];
        self.null_prs_buffer.reset();
        self.null_prs_buffer.consume(null_symbol);

        let net_frequency_offset = self.fine_frequency_offset + self.coarse_frequency_offset;
        apply_pll(self.data_time_buffer.iter_mut(), net_frequency_offset);

        // Clause 3.13: Frequency offset estimation and correction
        // Clause 3.13.1 - Fraction frequency offset estimation
        let total_phase_error: f32 = (0..self.params.nb_symbols)
            .map(|i| &self.data_time_buffer[chunk_slice(i, self.params.nb_symbol_period)])
            .map(|sym| calculate_cyclic_phase_error(sym, self.params.nb_cyclic_prefix))
            .sum();
        let average_phase_error = total_phase_error / (self.params.nb_symbols as f32);

        // Clause 3.13.1 - Fraction frequency offset estimation
        {
            use std::f32::consts::PI;
            let fft_bin_spacing = 1.0 / (self.params.nb_fft as f32);
            let fine_frequency_error = fft_bin_spacing/2.0 * average_phase_error/PI;
            let beta = self.settings.fine_frequency_update_beta;
            let delta = -beta*fine_frequency_error;
            self.update_fine_frequency_offset(delta);
        }

        // Clause 3.14.2 - FFT
        (0..self.params.nb_symbols)
            .for_each(|i| {
                let symbol_in = &self.data_time_buffer[chunk_slice(i, self.params.nb_symbol_period)];
                let fft_in = &symbol_in[self.params.nb_cyclic_prefix..];
                let fft_out = &mut self.data_fft_buffer[chunk_slice(i, self.params.nb_fft)];
                fft_out.copy_from_slice(fft_in);
                self.fft.process(fft_out);
            });

        // Clause 3.15 - Differential demodulator
        (0..self.params.nb_dqpsk_symbols)
            .for_each(|i| {
                let x0 = &self.data_fft_buffer[chunk_slice(i  , self.params.nb_fft)];
                let x1 = &self.data_fft_buffer[chunk_slice(i+1, self.params.nb_fft)];
                let y = &mut self.data_dqpsk_buffer[chunk_slice(i, self.params.nb_fft_data_carriers)];
                calculate_dqpsk(&self.params, x0, x1, y);
            });

        // Clause 3.16 - Data demapper
        (0..self.params.nb_dqpsk_symbols)
            .for_each(|i| {
                let x = &self.data_dqpsk_buffer[chunk_slice(i, self.params.nb_fft_data_carriers)];
                let y = &mut self.data_out_bits_buffer[chunk_slice(i, self.params.nb_fft_data_carriers*2)];
                calculate_soft_bits(&self.carrier_mapper_data, x, y);
            });

        for callback in &mut self.bits_out_callbacks {
            callback(&self.data_out_bits_buffer);
        }

        self.total_frames_read += 1;
        self.state = OfdmDemodulatorState::ReadingNullAndPrs;
    }

    fn update_signal_power_average(&mut self, buf: &[Complex32]) {
        let block_size = self.settings.null_power_total_samples;
        let stride = self.settings.null_power_decimation_factor;

        let (total_blocks, power_sum) = buf
            .chunks_exact(block_size)
            .enumerate()
            .filter(|(index,_)| index % stride == 0)
            .map(|(_,x)| calculate_l1_average(x))
            .fold((0usize, 0.0), |(total, sum),y| {
                (total + 1, sum + y)
            });

        if total_blocks == 0 {
            return;
        }

        let l1_average = power_sum / (total_blocks as f32);
        let beta = self.settings.null_power_update_beta;
        self.signal_l1_average = beta*l1_average + (1.0-beta)*self.signal_l1_average;
    }

    fn update_fine_frequency_offset(&mut self, delta: f32) {
        let fft_bin_spacing = 1.0/(self.params.nb_fft as f32) * 0.5; 
        let fft_bin_margin = 1.01;
        let fft_bin_wrap = fft_bin_spacing * fft_bin_margin;

        // TODO: If we are planning on multithreading this then we need to lock the fine frequency offset
        self.fine_frequency_offset += delta;
        self.fine_frequency_offset %= fft_bin_wrap;
    }
}

fn calculate_l1_average(block: &[Complex32]) -> f32 {
    let l1_sum: f32 = block
        .iter()
        .map(|x| x.l1_norm())
        .sum();
    l1_sum / (block.len() as f32)
}

fn calculate_relative_phase(x: &mut[Complex32]) {
    let length = x.len();
    for i in 0..(length-1) {
        let delta = x[i].conj() * x[i+1];
        x[i] = delta;
    }
    x[length-1] = Complex32 { re: 0.0, im: 0.0 };
}

fn calculate_magnitude_spectrum(x: &[Complex32], y: &mut[f32]) {
    assert!(x.len() == y.len());
    let n = x.len();
    let m = n/2;
    for i in 0..n {
        let j = (i+m) % n;
        let mag: f32 = 20.0 * x[j].norm().log10();
        y[i] = mag;
    }
}

// SOURCE: https://mooooo.ooo/chebyshev-sine-approximation 
//         Chebyshev polynomial that approximates f(x) = sin(2*pi*x) accurately within [-0.75,+0.75]
fn fast_sine(x: f32) -> f32 {
    const A0: f32 = -25.1327419281005859375;
    const A1: f32 =  64.83582305908203125;
    const A2: f32 = -67.076629638671875;
    const A3: f32 =  38.495880126953125;
    const A4: f32 = -14.049663543701171875;
    const A5: f32 =  3.161602020263671875;

    // Calculate g(x) = a5*x^10 + a4*x^8 + a3*x^6 + a2*x^4 + a1*x^2 + a0
    let z = x*x;        // z = x^2
    let b5 = A5;        // a5*z^0
    let b4 = b5*z + A4; // a5*z^1 + a4*z^0
    let b3 = b4*z + A3; // a5*z^2 + a4*z^1 + a3*z^0
    let b2 = b3*z + A2; // a5*z^3 + a4*z^2 + a3*z^1 + a2*z^0
    let b1 = b2*z + A1; // a5*z^4 + a4*z^3 + a3*z^2 + a2*z^1 + a1*z^0
    let b0 = b1*z + A0; // a5*z^5 + a4*z^4 + a3*z^3 + a2*z^2 + a1*z^1 + a0*z^0

    // Calculate f(x) = g(x) * (x-0.5) * (x+0.5) * x
    //           f(x) = g(x) * (x^2 - 0.25) * x
    //           f(x) = g(x) * (z-0.25) * x
    b0 * (z-0.25) * x
}

fn apply_pll(x: &mut [Complex32], freq_offset_normalised: f32) {
    x.iter_mut().enumerate().for_each(|(i, x)| {
        let dt = (i as f32)*freq_offset_normalised;
        // get absolute integer offset from [-0.5,+0.5]
        // let dt = dt - dt.round();
        // NOTE: Faster version of f32::round()
        let dt_offset = dt.abs() - 0.5;
        let dt_offset = dt_offset.ceil();
        let dt_offset = dt_offset*dt.signum();
        let dt = dt - dt_offset;        // translate to [-0.5,+0.5]
        let sin = fast_sine(dt);        // occupies [-0.5,+0.5]
        let cos = fast_sine(dt + 0.25); // occupies [-0.25,+0.75]
        let pll = Complex32::new(cos, sin);
        *x *= pll;
    });
}

fn calculate_cyclic_phase_error(x: &[Complex32], prefix_length: usize) -> f32 {
    let length = x.len();
    assert!(length >= prefix_length);

    let prefix = &x[0..prefix_length];
    let suffix = &x[span_slice(length-prefix_length, prefix_length)];

    let conjugate_sum: Complex32 = (0..prefix_length)
        .map(|i| suffix[i] * prefix[i].conj())
        .sum();

    conjugate_sum.im.atan2(conjugate_sum.re)
}

fn calculate_dqpsk(params: &OfdmParameters, x0: &[Complex32], x1: &[Complex32], y: &mut[Complex32]) {
    let nb_fft = params.nb_fft;
    let nb_data = params.nb_fft_data_carriers;
    let nb_data_half = nb_data/2;

    assert!(x0.len() == nb_fft, "x0 ({}) has different length to the fft ({})", x0.len(), nb_fft);
    assert!(x1.len() == nb_fft, "x1 ({}) has different length to the fft ({})", x1.len(), nb_fft);
    assert!(y.len() == nb_data, "y ({}) has different length to the number of data carriers ({})", y.len(), nb_data);
    assert!(nb_fft >= nb_data, "length of fft ({}) is less than number of required data carriers ({})", nb_fft, nb_data);
    assert!(nb_data % 2 == 0, "number of data carriers must be even ({})", nb_data);

    // x0,x1 are FFTs where [0,N] => [0,2Fs)
    // y is the DQPSK for the frequency range [-Fa,0)+(0,Fa] => [2Fs-Fa,2Fs), (0,Fa]

    // [-Fa,0) => [2Fs-Fa,2Fs)
    for i in 0..nb_data_half {
        let dqpsk_index = i;
        let fft_index = nb_fft-nb_data_half+i;
        let phase_delta = x0[fft_index] * x1[fft_index].conj();
        y[dqpsk_index] = phase_delta;
    }
    // (0,Fa] => (0,Fa]
    for i in 0..nb_data_half {
        let dqpsk_index = i + nb_data_half;
        let fft_index = 1+i;
        let phase_delta = x0[fft_index] * x1[fft_index].conj();
        y[dqpsk_index] = phase_delta;
    }
}

fn calculate_soft_bits(carrier_mapper: &[usize], x: &[Complex32], y: &mut[i8]) {
    assert!(carrier_mapper.len() == x.len(), "Carrier map and input symbols have mismatching lengths {} != {}", carrier_mapper.len(), x.len());
    assert!(x.len()*2 == y.len(), "Requires 2 soft bits for each input symbol but arrays are of lengths {} and {}", x.len(), y.len());

    let length = carrier_mapper.len();

    // Clause 3.16 - Data demapper
    for i in 0..length {
        let i_mapped = carrier_mapper[i];
        let mut vec = x[i_mapped];

        // NOTE: Use the L1 norm since it doesn't truncate like L2 norm
        //       I.e. When real=imag, then we expect b0=A, b1=A
        //            But with L2 norm, we get b0=0.707*A, b1=0.707*A
        //                with L1 norm, we get b0=A, b1=A as expected
        let amplitude = vec.re.abs().max(vec.im.abs());
        vec /= amplitude;
        
        y[i]        = quantise_to_soft_bit( vec.re);
        y[i+length] = quantise_to_soft_bit(-vec.im);
    }
}

#[inline(always)]
fn quantise_to_soft_bit(x: f32) -> i8 {
    // Clause 3.4.2 - QPSK symbol mapper
    // phi = (1-2*b0) + (1-2*b1)*1j
    // x0 = 1-2*b0, x1 = 1-2*b1
    // b = (1-x)/2

    // NOTE: Phil Karn's viterbi decoder is configured so that b => b' : (0,1) => (-A,+A)
    // Where b is the logical bit value, and b' is the value used for soft decision decoding
    // b' = (2*b-1) * A 
    // b' = (1-x-1)*A
    // b' = -A*x

    let soft_decision_viterbi_high: f32 = 127.0;
    let y = -x * soft_decision_viterbi_high;
    y as i8
}

#[inline(always)]
fn span_slice(start: usize, length: usize) -> std::ops::Range<usize> {
    start..start+length
}

#[inline(always)]
fn chunk_slice(index: usize, length: usize) -> std::ops::Range<usize> {
    let start_index = index*length;
    span_slice(start_index, length)
}


