use std::{
    cell::RefCell,
    f32::consts::{PI, TAU},
    marker::PhantomData,
    sync::Arc,
};

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
    fft_forward: Arc<dyn Fft<f32>>,
    scratch_forward: RefCell<Vec<Complex32>>,
    fft_inverse: Arc<dyn Fft<f32>>,
    scratch_inverse: RefCell<Vec<Complex32>>,
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

        let fft_forward = planner.plan_fft_forward(window_size);
        let scratch_forward_length = fft_forward.get_inplace_scratch_len();
        let mut scratch_forward = vec![Complex32::new(0.0, 0.0); scratch_forward_length];

        let fft_inverse = planner.plan_fft_inverse(window_size);
        let scratch_inverse_length = fft_forward.get_inplace_scratch_len();
        let mut scratch_inverse = vec![Complex32::new(0.0, 0.0); scratch_inverse_length];

        // Calculate scale
        let mut scale_transform = vec![Complex32::new(1.0, 0.0); window_size];
        fft_forward.process_with_scratch(&mut scale_transform, &mut scratch_forward);
        fft_inverse.process_with_scratch(&mut scale_transform, &mut scratch_inverse);

        Interpolator {
            fft_forward,
            scratch_forward: RefCell::new(scratch_forward),
            fft_inverse,
            scratch_inverse: RefCell::new(scratch_inverse),
            sample_provider,
            window_size,
            scale: scale_transform[0].re,
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

        // TODO: Cache half window size
        let half_window_size_usize = self.window_size / 2;
        let half_window_size_isize = half_window_size_usize as isize;

        for window_sample_index in
            (index_truncated_isize - half_window_size_isize)..(index_truncated_isize + half_window_size_isize)
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

        let mut scratch_forward = self.scratch_forward.borrow_mut();
        self.fft_forward
            .process_with_scratch(&mut transform, &mut scratch_forward);

        for freq_index in 1..=(self.window_size / 2) {
            let (freq_amplitude, phase) = transform[freq_index].to_polar();

            // Fraction of tau for each sample
            // (This can be precalculated and cached)
            let phase_shift_per_sample = TAU / (self.window_size as f32 / freq_index as f32);
            let phase_adjustment = phase_shift_per_sample * index.fract();
            let adjusted_phase = phase + phase_adjustment;

            transform[freq_index] = Complex32::from_polar(freq_amplitude, adjusted_phase);
            let opposite_freq_index = self.window_size - freq_index;
            if opposite_freq_index != freq_index {
                transform[opposite_freq_index] =
                    Complex32::from_polar(freq_amplitude, adjusted_phase * -1.0);
            }
        }

        let mut scratch_inverse = self.scratch_inverse.borrow_mut();
        self.fft_inverse
            .process_with_scratch(&mut transform, &mut scratch_inverse);

        let interpolated_sample = transform[half_window_size_usize].re / self.scale;
        return Ok(interpolated_sample);
    }
}
