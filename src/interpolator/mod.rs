use std::{cell::RefCell, f32::consts::{PI, TAU}, marker::PhantomData, sync::Arc};

use rustfft::{num_complex::Complex32, Fft, FftPlanner};

pub type GetSampleClosure = dyn Fn(usize) -> f32;

pub trait SampleProvider<TChannelId, TError>
where
    TChannelId: Copy,
{
    fn get_sample(&self, channel_id: TChannelId, index: usize) -> Result<f32, TError>;
}

pub struct Interpolator<TSampleProvider, TChannelId, TError>
where
    TSampleProvider: SampleProvider<TChannelId, TError>,
    TChannelId: Copy,
{
    fft: Arc<dyn Fft<f32>>,
    scratch: RefCell<Vec<Complex32>>,
    sample_provider: TSampleProvider,
    window_size: usize,
    scale: f32,
    num_samples: usize,

    _phantom_data: PhantomData<(TChannelId, TError)>,
}

impl<TSampleProvider, TChannelId, TError> Interpolator<TSampleProvider, TChannelId, TError>
where
    TSampleProvider: SampleProvider<TChannelId, TError>,
    TChannelId: Copy,
{
    pub fn new(
        window_size: usize,
        num_samples: usize,
        sample_provider: TSampleProvider,
    ) -> Interpolator<TSampleProvider, TChannelId, TError> {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(window_size);
        let scratch_length = fft.get_inplace_scratch_len();
        let scratch = vec![Complex32::new(0.0, 0.0); scratch_length];

        // Calculate scale
        //let mut scale_transform = vec![Complex32::new(1.0, 0.0); window_size];
        //fft.process_with_scratch(&mut scale_transform, &mut scratch);
        //let (scale_denominator, _) = scale_transform[0].to_polar();

        Interpolator {
            fft,
            scratch: RefCell::new(scratch),
            sample_provider,
            window_size,
            //scale: 1.0 / scale_denominator,
            scale: 1.0 / (window_size as f32),
            num_samples,
            _phantom_data: PhantomData,
        }
    }
    pub fn get_interpolated_sample(
        &self,
        channel_id: TChannelId,
        index: f32,
    ) -> Result<f32, TError> {
        let index_truncated = index.trunc();
        if index == index_truncated {
            return self
                .sample_provider
                .get_sample(channel_id, index_truncated as usize);
        }

        let index_truncated_isize = index_truncated as isize;

        // TODO: Cache the transform

        let mut transform = Vec::with_capacity(self.window_size);

        let half_window_size = (self.window_size / 2) as isize;
        for window_sample_index in
            (index_truncated_isize - half_window_size)..(index_truncated_isize + half_window_size)
        {
            let sample =
                if window_sample_index >= 0 && window_sample_index < self.num_samples as isize {
                    self.sample_provider
                        .get_sample(channel_id, window_sample_index as usize)?
                } else {
                    0.0
                };

            transform.push(Complex32 {
                re: sample,
                im: 0.0,
            });
        }

        let mut scratch = self.scratch.borrow_mut();
        self.fft.process_with_scratch(&mut transform, &mut scratch);
        let (dc, _) = transform[0].to_polar();
        let mut amplitude_sum = dc;

        for freq_index in 1..=(self.window_size / 2) {
            let (freq_amplitude, phase) = transform[freq_index].to_polar();
            let freq_amplitude = freq_amplitude * 2.0;

            // Algorithm to get the sample's value for this frequency
            // ----
            // Convert index to fraction through the cycle
            // Add phase
            // Get cosine

            // How to get fraction through the cycle:
            // Calculate the fraction of a single sample in the wavelength
            // Add offset from this sample

            // This can be in a lookup table for speed
            let fraction_of_sample_in_wavelength = 1.0 / ((self.window_size as f32) / (freq_index as f32)) * TAU;
            let fraction_of_index_from_center = fraction_of_sample_in_wavelength * index.fract();

            let mut phase_between_samples = phase + fraction_of_index_from_center;
            
            // Special case for lowest frequency because the sample at the midpoint is halfway through the cycle
            if freq_index == 1 {
                phase_between_samples += PI;
            }

            /*


            // For performance this can go into a lookup table
            let frequency_cycles_in_transform = ((self.window_size / 2) - freq_index + 1) as f32;
            let tau_range_between_samples = TAU / frequency_cycles_in_transform;

            let mut phase_between_samples =
                ((index.fract() / 2.0) * tau_range_between_samples) + phase;
            if phase_between_samples > TAU {
                phase_between_samples -= TAU;
            }
*/
            let freq_part = phase_between_samples.cos() * freq_amplitude;
            amplitude_sum += freq_part;
        }

        return Ok(amplitude_sum * self.scale);
    }
}
