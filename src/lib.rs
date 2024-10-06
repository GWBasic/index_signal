pub mod interpolator;

#[cfg(test)]
mod tests {
    use std::io::{Error, ErrorKind, Result};

    use super::*;

    use interpolator::{Interpolator, SampleProvider};

    struct NyquistSampleProvider {}

    impl SampleProvider<&str, Error> for NyquistSampleProvider {
        fn get_sample(&self, channel_id: &str, index: usize) -> Result<f32> {
            assert!(channel_id.eq("test"));

            if index % 2 == 0 {
                Ok(1.0)
            } else {
                Ok(-1.0)
            }
        }
    }

    #[test]
    fn whole_sample() {
        let interpolator = Interpolator::new(NyquistSampleProvider {});

        assert_eq!(1.0, interpolator.get_interpolated_sample("test", 0.0).unwrap());
        assert_eq!(-1.0, interpolator.get_interpolated_sample("test", 1.0).unwrap());
        assert_eq!(1.0, interpolator.get_interpolated_sample("test", 2.0).unwrap());
        assert_eq!(-1.0, interpolator.get_interpolated_sample("test", 3.0).unwrap());
    }

    #[test]
    fn partial_sample() {
        let interpolator = Interpolator::new(NyquistSampleProvider {});

        assert_eq!(0.0, interpolator.get_interpolated_sample("test", 0.5).unwrap());
        assert_eq!(0.0, interpolator.get_interpolated_sample("test", 1.5).unwrap());
        assert_eq!(0.0, interpolator.get_interpolated_sample("test", 2.5).unwrap());
        assert_eq!(0.0, interpolator.get_interpolated_sample("test", 3.5).unwrap());
    }

    struct ErrorSampleProvider {}

    impl SampleProvider<&str, Error> for ErrorSampleProvider {
        fn get_sample(&self, channel_id: &str, index: usize) -> Result<f32> {
            assert!(channel_id.eq("test"));

            if index == 3 {
                Err(Error::from(ErrorKind::BrokenPipe))
            } else {
                Ok(index as f32)
            }
        }
    }

    #[test]
    fn errors_passthrough() {
        let interpolator = Interpolator::new(ErrorSampleProvider {});

        assert_eq!(0.0, interpolator.get_interpolated_sample("test", 0.0).unwrap());

        assert_eq!(ErrorKind::BrokenPipe, interpolator.get_interpolated_sample("test", 3.0).unwrap_err().kind());
        assert_eq!(ErrorKind::BrokenPipe, interpolator.get_interpolated_sample("test", 2.1).unwrap_err().kind());
        assert_eq!(ErrorKind::BrokenPipe, interpolator.get_interpolated_sample("test", 3.1).unwrap_err().kind());
    }
}
