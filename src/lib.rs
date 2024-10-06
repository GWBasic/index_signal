pub mod interpolator;

#[cfg(test)]
mod tests {
    use super::*;

    use interpolator::{Interpolator, SampleProvider};

    struct NyquistSampleProvider {}

    impl SampleProvider<&str> for NyquistSampleProvider {
        fn get_sample(&self, channel_id: &str, index: usize) -> f32 {
            assert!(channel_id.eq("test"));

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

        assert_eq!(1.0, interpolator.get_interpolated_sample("test", 0.0));
        assert_eq!(-1.0, interpolator.get_interpolated_sample("test", 1.0));
        assert_eq!(1.0, interpolator.get_interpolated_sample("test", 2.0));
        assert_eq!(-1.0, interpolator.get_interpolated_sample("test", 3.0));
    }

    #[test]
    fn partial_sample() {
        let interpolator = Interpolator::new(NyquistSampleProvider {});

        assert_eq!(0.0, interpolator.get_interpolated_sample("test", 0.5));
        assert_eq!(0.0, interpolator.get_interpolated_sample("test", 1.5));
        assert_eq!(0.0, interpolator.get_interpolated_sample("test", 2.5));
        assert_eq!(0.0, interpolator.get_interpolated_sample("test", 3.5));
    }

    // Need a test that opens a .wav file on disk
    // This is mainly to make sure I understand the memory model
}
