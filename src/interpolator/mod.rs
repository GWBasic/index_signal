use std::{cell::RefCell, f32::consts::TAU, sync::Arc};

use rustfft::{num_complex::Complex32, Fft, FftPlanner};

pub type GetSampleClosure = dyn Fn(usize) -> f32;

pub struct Interpolator {
    fft: Arc<dyn Fft<f32>>,
    scratch: RefCell<Vec<Complex32>>,
    get_sample: Box<GetSampleClosure>,
}

impl Interpolator {
    pub fn new(get_sample: Box<GetSampleClosure>) -> Interpolator {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(2);
        let scratch_length = fft.get_inplace_scratch_len();

        Interpolator {
            fft,
            scratch: RefCell::new(vec![Complex32::new(0.0, 0.0); scratch_length]),
            get_sample,
        }
    }

    pub fn get_interpolated_sample(&self, index: f32) -> f32 {
        let index_truncated = index.trunc();
        let index_truncated_usize = index_truncated as usize;
        if index == index_truncated {
            return (self.get_sample)(index_truncated_usize);
        }

        let sample0 = (self.get_sample)(index_truncated_usize);
        let sample1 = (self.get_sample)(index_truncated_usize + 1);

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
        return freq_part + amplitude;
    }
}
