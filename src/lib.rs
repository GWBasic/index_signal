pub mod interpolator;

#[cfg(test)]
mod tests {
    use super::*;

    use interpolator::{Interpolator, SampleProvider};

    struct NyquistSampleProvider {}

    impl SampleProvider for NyquistSampleProvider {
        fn get_sample(&self, index: usize) -> f32 {
            if index % 2 == 0 {
                1.0
            } else {
                -1.0
            }
        }
    }

    #[test]
    fn whole_sample() {
        let interpolator = Interpolator::new(NyquistSampleProvider {});

        assert_eq!(1.0, interpolator.get_interpolated_sample(0.0));
        assert_eq!(-1.0, interpolator.get_interpolated_sample(1.0));
        assert_eq!(1.0, interpolator.get_interpolated_sample(2.0));
        assert_eq!(-1.0, interpolator.get_interpolated_sample(3.0));
    }

    #[test]
    fn partial_sample() {
        let interpolator = Interpolator::new(NyquistSampleProvider {});

        assert_eq!(0.0, interpolator.get_interpolated_sample(0.5));
        assert_eq!(0.0, interpolator.get_interpolated_sample(1.5));
        assert_eq!(0.0, interpolator.get_interpolated_sample(2.5));
        assert_eq!(0.0, interpolator.get_interpolated_sample(3.5));
    }

    // Need a test that opens a .wav file on disk
    // This is mainly to make sure I understand the memory model
}
