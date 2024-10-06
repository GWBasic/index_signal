use std::{cell::RefCell, f32::consts::TAU, sync::Arc};

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
    fft: Arc<dyn Fft<f32>>,
    scratch: RefCell<Vec<Complex32>>,
    sample_provider: TSampleProvider,
    // Not sure how to remove this
    _workaround: Option<TChannelId>,
    _workaround2: Option<TError>
}

impl<TSampleProvider, TChannelId, TError> Interpolator<TSampleProvider, TChannelId, TError>
where
    TSampleProvider : SampleProvider<TChannelId, TError>,
    TChannelId : Copy
{
    pub fn new(sample_provider: TSampleProvider) -> Interpolator<TSampleProvider, TChannelId, TError> {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(2);
        let scratch_length = fft.get_inplace_scratch_len();

        Interpolator {
            fft,
            scratch: RefCell::new(vec![Complex32::new(0.0, 0.0); scratch_length]),
            sample_provider,
            _workaround: None,
            _workaround2: None
        }
    }

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

        let mut phase_between_samples = ((index.fract() / 2.0) * TAU) + 0.25 + phase;
        if phase_between_samples > TAU {
            phase_between_samples -= TAU;
        }

        let freq_part = phase_between_samples.cos() * upper_amplitude;
        return Ok(freq_part + amplitude);
    }
}
