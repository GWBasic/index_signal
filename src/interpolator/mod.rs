use std::{cell::RefCell, collections::HashMap, marker::PhantomData, rc::Rc, sync::Arc};

use rustfft::{num_complex::Complex32, Fft, FftPlanner};

pub type GetSampleClosure = dyn Fn(usize) -> f32;

pub trait SampleProvider<TChannelId, TError>
where
    TChannelId: Copy,
{
    fn get_sample(&self, channel_id: TChannelId, index: usize) -> Result<f32, TError>;
}

struct TransformCacheEntry {
    index: usize,
    transform: Vec<Complex32>,
}

struct FFTCacheEntry {
    pub fft_forward: Arc<dyn Fft<f32>>,
    pub scratch_forward: RefCell<Vec<Complex32>>,
    pub forward_scale: f32,
    pub fft_inverse: Arc<dyn Fft<f32>>,
    pub scratch_inverse: RefCell<Vec<Complex32>>,
    pub inverse_scale: f32,
}

pub struct Interpolator<TSampleProvider, TChannelId, TError>
where
    TSampleProvider: SampleProvider<TChannelId, TError>,
    TChannelId: Copy + std::cmp::Eq + std::hash::Hash,
{
    planner: RefCell<FftPlanner<f32>>,
    fft_cache: RefCell<HashMap<usize, Rc<FFTCacheEntry>>>,
    sample_provider: TSampleProvider,
    window_size: usize,
    num_samples: usize,
    phase_shifts_per_sample: Vec<f32>,
    transform_cache: RefCell<HashMap<TChannelId, TransformCacheEntry>>,

    _phantom_data: PhantomData<TError>,
}

impl<TSampleProvider, TChannelId, TError> Interpolator<TSampleProvider, TChannelId, TError>
where
    TSampleProvider: SampleProvider<TChannelId, TError>,
    TChannelId: Copy + std::cmp::Eq + std::hash::Hash,
{
    pub fn new(
        window_size: usize,
        num_samples: usize,
        sample_provider: TSampleProvider,
    ) -> Interpolator<TSampleProvider, TChannelId, TError> {
        let mut planner = FftPlanner::new();
        let fft_cache_entry = Self::construct_fft_cache_entry(&mut planner, window_size);

        // Calculate phase shifts per sample: Transform sine waves of 1.0, shift by one sample, transform back
        let mut phase_transform = vec![Complex32::from_polar(1.0, 0.0); window_size];
        phase_transform[0] = Complex32::from_polar(0.0, 0.0);
        fft_cache_entry
            .fft_inverse
            .process_with_scratch(&mut phase_transform, &mut fft_cache_entry.scratch_inverse.borrow_mut());

        let first_sample = phase_transform.remove(0);
        phase_transform.push(first_sample);
        fft_cache_entry
            .fft_forward
            .process_with_scratch(&mut phase_transform, &mut fft_cache_entry.scratch_forward.borrow_mut());

        let mut phase_shifts_per_sample = Vec::with_capacity(window_size / 2);
        for freq_index in 0..=(window_size / 2) {
            let (_, phase_shift_for_frequency) = phase_transform[freq_index].to_polar();
            phase_shifts_per_sample.push(phase_shift_for_frequency);
        }

        let fft_cache = RefCell::new(HashMap::new());
        fft_cache.borrow_mut().insert(window_size, Rc::new(fft_cache_entry));

        Interpolator {
            planner: RefCell::new(planner),
            fft_cache,
            sample_provider,
            window_size,
            num_samples,
            phase_shifts_per_sample,
            transform_cache: RefCell::new(HashMap::new()),
            _phantom_data: PhantomData,
        }
    }

    fn construct_fft_cache_entry(planner: &mut FftPlanner<f32>, window_size: usize) -> FFTCacheEntry {

        let fft_forward = planner.plan_fft_forward(window_size);
        let scratch_forward_length = fft_forward.get_inplace_scratch_len();
        let mut scratch_forward = vec![Complex32::new(0.0, 0.0); scratch_forward_length];

        let fft_inverse = planner.plan_fft_inverse(window_size);
        let scratch_inverse_length = fft_forward.get_inplace_scratch_len();
        let mut scratch_inverse = vec![Complex32::new(0.0, 0.0); scratch_inverse_length];

        // Calculate scale: Transform a DC signal of 1.0 back and forth to determine scale
        let mut scale_transform = vec![Complex32::new(1.0, 0.0); window_size];
        fft_forward.process_with_scratch(&mut scale_transform, &mut scratch_forward);
        let (forward_scale, _) = scale_transform[0].to_polar();

        fft_inverse.process_with_scratch(&mut scale_transform, &mut scratch_inverse);
        let inverse_scale = scale_transform[0].re;

        FFTCacheEntry {
            fft_forward,
            scratch_forward: RefCell::new(scratch_forward),
            forward_scale,
            fft_inverse,
            scratch_inverse: RefCell::new(scratch_inverse),
            inverse_scale,
        }
    }

    fn get_fft_cache_entry(&self, window_size: usize) -> Rc<FFTCacheEntry> {
        let mut fft_cache = self.fft_cache.borrow_mut();
        if let Some(cache_entry) = fft_cache.get(&window_size) {
            return cache_entry.clone();
        }

        let fft_cache_entry = Self::construct_fft_cache_entry(&mut self.planner.borrow_mut(), window_size);
        let fft_cache_entry = Rc::new(fft_cache_entry);
        fft_cache.insert(window_size, fft_cache_entry.clone());
        fft_cache_entry
    }

    pub fn get_interpolated_sample(
        &self,
        channel_id: TChannelId,
        index: f32,
        relative_speed: f32,
    ) -> Result<f32, TError> {
        if relative_speed <= 1.0 {
            self.get_interpolated_sample_no_aliasing_filter(channel_id, index)
        } else {
            self.get_interpolated_sample_with_aliasing_filter(channel_id, index, relative_speed)
        }
    }

    fn get_interpolated_sample_no_aliasing_filter(
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
        let half_window_size_usize = self.window_size / 2;
        let half_window_size_isize = half_window_size_usize as isize;

        let mut transform = {
            let mut transform_cache = self.transform_cache.borrow_mut();

            // Check cache first
            if let Some(cache_entry) = transform_cache.get(&channel_id) {
                if cache_entry.index == index_truncated as usize {
                    cache_entry.transform.clone()
                } else {
                    // Index doesn't match, need to compute new transform
                    self.compute_transform(
                        &mut transform_cache,
                        channel_id,
                        index_truncated_isize,
                        half_window_size_isize,
                    )?
                }
            } else {
                self.compute_transform(
                    &mut transform_cache,
                    channel_id,
                    index_truncated_isize,
                    half_window_size_isize,
                )?
            }
        };

        for freq_index in 1..=(self.window_size / 2) {
            let (freq_amplitude, phase) = transform[freq_index].to_polar();

            // Adjust phase for frequency
            let phase_shift_for_sample = self.phase_shifts_per_sample[freq_index];
            let phase_adjustment = phase_shift_for_sample * index.fract();
            let adjusted_phase = phase + phase_adjustment;

            transform[freq_index] = Complex32::from_polar(freq_amplitude, adjusted_phase);
            let opposite_freq_index = self.window_size - freq_index;
            if opposite_freq_index != freq_index {
                transform[opposite_freq_index] =
                    Complex32::from_polar(freq_amplitude, adjusted_phase * -1.0);
            }
        }

        let fft_cache_entry = self.get_fft_cache_entry(self.window_size);
        let mut scratch_inverse = fft_cache_entry.scratch_inverse.borrow_mut();
        fft_cache_entry.fft_inverse
            .process_with_scratch(&mut transform, &mut scratch_inverse);

        let interpolated_sample = transform[half_window_size_usize].re / fft_cache_entry.inverse_scale;
        Ok(interpolated_sample)
    }

    // Helper function to compute and cache transform
    fn compute_transform(
        &self,
        transform_cache: &mut HashMap<TChannelId, TransformCacheEntry>,
        channel_id: TChannelId,
        index_truncated_isize: isize,
        half_window_size_isize: isize,
    ) -> Result<Vec<Complex32>, TError> {
        let mut new_transform = Vec::with_capacity(self.window_size);

        for window_sample_index in (index_truncated_isize - half_window_size_isize)
            ..(index_truncated_isize + half_window_size_isize)
        {
            let sample =
                if window_sample_index >= 0 && window_sample_index < self.num_samples as isize {
                    self.sample_provider
                        .get_sample(channel_id, window_sample_index as usize)?
                } else {
                    0.0
                };

            new_transform.push(Complex32 {
                re: sample,
                im: 0.0,
            });
        }

        let fft_cache_entry = self.get_fft_cache_entry(self.window_size);
        let mut scratch_forward = fft_cache_entry.scratch_forward.borrow_mut();
        fft_cache_entry.fft_forward
            .process_with_scratch(&mut new_transform, &mut scratch_forward);

        // Store in cache
        transform_cache.insert(
            channel_id,
            TransformCacheEntry {
                index: index_truncated_isize as usize,
                transform: new_transform.clone(),
            },
        );

        Ok(new_transform)
    }

    fn get_interpolated_sample_with_aliasing_filter(
        &self,
        channel_id: TChannelId,
        index: f32,
        relative_speed: f32,
    ) -> Result<f32, TError> {
        let mut oversample_rate = relative_speed;
        let mut oversampling_ratio = 1;
        while oversample_rate > 1.0 {
            oversampling_ratio *= 2;
            oversample_rate /= 2.0;
        }

        // Freeze values
        let oversample_rate = oversample_rate;
        let oversampling_ratio = oversampling_ratio;

        let mut transform = Vec::with_capacity(oversampling_ratio);
        let start_index = index - (oversample_rate * (oversampling_ratio as f32 / 2.0));

        for i in 0..oversampling_ratio {
            let sample_index = start_index + (i as f32 * oversample_rate);
            let sample =
                self.get_interpolated_sample_no_aliasing_filter(channel_id, sample_index)?;
            transform.push(Complex32 {
                re: sample,
                im: 0.0,
            });
        }

        let fft_cache_entry = self.get_fft_cache_entry(oversampling_ratio);
        fft_cache_entry.fft_forward.process_with_scratch(&mut transform, &mut fft_cache_entry.scratch_forward.borrow_mut());

        let (unscaled_sample, _) = transform[0].to_polar();
        let sample = unscaled_sample / fft_cache_entry.forward_scale;
        Ok(sample)
    }
}
