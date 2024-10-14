use std::{cell::RefCell, f32::consts::PI, marker::PhantomData, sync::Arc};

use rustfft::{num_complex::Complex32, Fft, FftPlanner};

pub type GetSampleClosure = dyn Fn(usize) -> f32;

pub trait SampleProvider<TChannelId, TError>
where TChannelId : Copy {
    // TODOs:
    // - Pass through errors instead of relying on panic
    fn get_sample(&self, channel_id: TChannelId, index: usize) -> Result<f32, TError>;
}

pub struct Interpolator<TSampleProvider, TChannelId, TError>
where
    TSampleProvider : SampleProvider<TChannelId, TError>,
    TChannelId : Copy
{
    _fft: Arc<dyn Fft<f32>>,
    _scratch: RefCell<Vec<Complex32>>,
    sample_provider: TSampleProvider,
    window_size: usize,
    max_samples: usize,

    _phantom_data: PhantomData<(TChannelId, TError)>
}

/// The sinc function. sinc(x) = sin(πx) / (πx), with sinc(0) = 1.
/// Based on conversation with ChatGPT: https://chatgpt.com/share/670d349d-c904-8007-9d60-a4ded4864cd7
fn sinc(x: f32) -> f32 {
    if x == 0.0 {
        1.0
    } else {
        (x * PI).sin() / (x * PI)
    }
}

impl<TSampleProvider, TChannelId, TError> Interpolator<TSampleProvider, TChannelId, TError>
where
    TSampleProvider : SampleProvider<TChannelId, TError>,
    TChannelId : Copy
{
    pub fn new(window_size: usize, max_samples: usize, sample_provider: TSampleProvider) -> Interpolator<TSampleProvider, TChannelId, TError> {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(2);
        let scratch_length = fft.get_inplace_scratch_len();

        Interpolator {
            _fft: fft,
            _scratch: RefCell::new(vec![Complex32::new(0.0, 0.0); scratch_length]),
            sample_provider,
            window_size,
            max_samples,
            _phantom_data: PhantomData
        }
    }

    pub fn get_interpolated_sample(&self, channel_id: TChannelId, index: f32) -> Result<f32, TError> {
        // Based on conversation with ChatGPT: https://chatgpt.com/share/670d349d-c904-8007-9d60-a4ded4864cd7
        let t_isize = index.floor() as isize;  // Integer part (nearest sample)
        let t_frac = index - index.floor();      // Fractional part
    
        let mut interpolated_value = 0.0;
        let half_window = self.window_size / 2;
    
        for i in -(half_window as isize)..=half_window as isize {
            let sample_index_isize = t_isize + i;

            if sample_index_isize < 0 {
                continue;
            }

            let sample_index = sample_index_isize as usize;
    
            // Check boundaries
            if sample_index >= self.max_samples {
                continue;
            }
    
            // Compute the sinc value for the given offset
            let sinc_value = sinc(t_frac - i as f32);

            let sample = self.sample_provider.get_sample(channel_id, sample_index)?;

            // Accumulate the weighted sum of samples
            interpolated_value += sample * sinc_value;
        }
    
        Ok(interpolated_value)
    }

    /*
    pub fn get_interpolated_sample(&self, channel_id: TChannelId, index: f32) -> Result<f32, TError> {
        let index_truncated = index.trunc();
        let index_truncated_usize = index_truncated as usize;
        if index == index_truncated {
            return self.sample_provider.get_sample(channel_id, index_truncated_usize);
        }

        // TODO: Cache the transform

        let sample0 = self.sample_provider.get_sample(channel_id, index_truncated_usize)?;
        let sample1 = self.sample_provider.get_sample(channel_id, index_truncated_usize + 1)?;

        let mut transform = vec![
            Complex32 {
                re: sample0, im: 0.0
            },
            Complex32 {
                re: sample1, im: 0.0
            }
        ];

        let mut scratch = self.scratch.borrow_mut();
        self.fft.process_with_scratch(&mut transform, &mut scratch);
        let (amplitude, _) = transform[0].to_polar();
        let amplitude = amplitude * 0.5;
        let (upper_amplitude, phase) = transform[1].to_polar();
        let upper_amplitude = upper_amplitude * 0.5;

        let mut phase_between_samples = ((index.fract() / 2.0) * TAU) + phase;
        if phase_between_samples > TAU {
            phase_between_samples -= TAU;
        }

        let freq_part = phase_between_samples.cos() * upper_amplitude;
        return Ok(freq_part + amplitude);
    }
    */
}
