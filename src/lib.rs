pub mod interpolator;

#[cfg(test)]
mod tests {
    use std::{f32::consts::PI, io::{Error, ErrorKind, Result}};

    use super::*;

    use interpolator::{Interpolator, SampleProvider};

    fn assert(expected: f32, actual: f32, error_message: &str) {
        // Note: 24-bit audio differentiates samples at 0.00000012 precision
        let difference = (expected - actual).abs();

        if difference > 0.00000012 {
            panic!("{}: Expected: {}, Actual: {}", error_message, expected, actual);
        }
    }

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
        let interpolator = Interpolator::new(20, 200, NyquistSampleProvider {});

        assert_eq!(1.0, interpolator.get_interpolated_sample("test", 0.0).unwrap());
        assert_eq!(-1.0, interpolator.get_interpolated_sample("test", 1.0).unwrap());
        assert_eq!(1.0, interpolator.get_interpolated_sample("test", 2.0).unwrap());
        assert_eq!(-1.0, interpolator.get_interpolated_sample("test", 3.0).unwrap());
    }

    #[test]
    fn partial_sample_nyquist() {
        let interpolator = Interpolator::new(20, 200, NyquistSampleProvider {});

        assert(0.0, interpolator.get_interpolated_sample("test", 100.5).unwrap(), "Wrong value for 100.5");
        assert(0.0, interpolator.get_interpolated_sample("test", 101.5).unwrap(), "Wrong value for 101.5");
        assert(0.0, interpolator.get_interpolated_sample("test", 102.5).unwrap(), "Wrong value for 102.5");
        assert(0.0, interpolator.get_interpolated_sample("test", 103.5).unwrap(), "Wrong value for 103.5");
    }

    struct DCSampleProvider {
        pub result: f32
    }

    impl SampleProvider<&str, Error> for DCSampleProvider {
        fn get_sample(&self, channel_id: &str, _index: usize) -> Result<f32> {
            assert!(channel_id.eq("dc"));
            Ok(self.result)
        }
    }

    #[test]
    fn dc() {
        let interpolator = Interpolator::new(20, 200, DCSampleProvider {result: 0.75});

        assert_eq!(0.75, interpolator.get_interpolated_sample("dc", 100.5).unwrap());
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
        let interpolator = Interpolator::new(20, 200, ErrorSampleProvider {});

        assert_eq!(0.0, interpolator.get_interpolated_sample("test", 0.0).unwrap());

        assert_eq!(ErrorKind::BrokenPipe, interpolator.get_interpolated_sample("test", 3.0).unwrap_err().kind());
        assert_eq!(ErrorKind::BrokenPipe, interpolator.get_interpolated_sample("test", 2.1).unwrap_err().kind());
        assert_eq!(ErrorKind::BrokenPipe, interpolator.get_interpolated_sample("test", 3.1).unwrap_err().kind());
    }

    struct FourSampleWavelengthSignalProvider {}

    fn get_four_sample_wavelength_sample(x: f32) -> f32 {
        let arg = x * PI / 2.0;
        let y = arg.sin();
        y
    }

    impl SampleProvider<&str, Error> for FourSampleWavelengthSignalProvider {
        fn get_sample(&self, channel_id: &str, index: usize) -> Result<f32> {
            assert!(channel_id.eq("test"));

            Ok(get_four_sample_wavelength_sample(index as f32))
        }
    }

    #[test]
    fn four_sample_wavelength() {
        let interpolator = Interpolator::new(4, 2000, FourSampleWavelengthSignalProvider {});

        let expected = get_four_sample_wavelength_sample(10.2);
        let actual = interpolator.get_interpolated_sample("test", 10.2).unwrap();

        assert(expected, actual, "Wrong value for a four-sample window");
    }

    struct SignalSampleProvider {}

    fn get_signal_sample(x: f32) -> f32 {
        let y = x.sin() + (x/3.0).sin() + (x/1.6).sin() + (x/5.2).cos();
        //let y = (x/5.2).cos();
        y / 4.0
    }

    impl SampleProvider<&str, Error> for SignalSampleProvider {
        fn get_sample(&self, channel_id: &str, index: usize) -> Result<f32> {
            assert!(channel_id.eq("test"));

            Ok(get_signal_sample(index as f32))
        }
    }

    #[test]
    fn continuous_signal() {
        let interpolator = Interpolator::new(60, 2000, SignalSampleProvider {});

        let mut x = 500.0;
        while x <= 1500.0 {
            let expected_sample = get_signal_sample(x);
            let actual_sample = interpolator.get_interpolated_sample("test", x).unwrap();

            assert(expected_sample, actual_sample, &format!("When reading from a continuous sample at index {}", x));
            //println!("Expected: {}, Actual: {} ({})", expected_sample, actual_sample, x);

            x += 0.01;
        }

        //assert_eq!(1, 0);
    }
}
