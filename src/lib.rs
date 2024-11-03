pub mod interpolator;

#[cfg(test)]
mod tests {
    use std::{
        f32::consts::PI,
        io::{Error, ErrorKind, Result},
    };

    use super::*;

    use interpolator::{Interpolator, SampleProvider};

    fn assert(expected: f32, actual: f32, error_message: &str) {
        // Note: 24-bit audio differentiates samples at 0.00000012 precision
        let difference = (expected - actual).abs();

        // 24-bit accuracy: 0.00000012 = 1 / (2^24)
        // 16-bit accuracy: 0.00001526 = 1 / (2^16)

        if difference > 0.00001526 {
            panic!(
                "{}: Expected: {}, Actual: {}",
                error_message, expected, actual
            );
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

        assert_eq!(
            1.0,
            interpolator.get_interpolated_sample("test", 0.0).unwrap()
        );
        assert_eq!(
            -1.0,
            interpolator.get_interpolated_sample("test", 1.0).unwrap()
        );
        assert_eq!(
            1.0,
            interpolator.get_interpolated_sample("test", 2.0).unwrap()
        );
        assert_eq!(
            -1.0,
            interpolator.get_interpolated_sample("test", 3.0).unwrap()
        );
    }

    #[test]
    fn partial_sample_nyquist() {
        let interpolator = Interpolator::new(20, 200, NyquistSampleProvider {});

        assert(
            0.0,
            interpolator.get_interpolated_sample("test", 100.5).unwrap(),
            "Wrong value for 100.5",
        );
        assert(
            0.0,
            interpolator.get_interpolated_sample("test", 101.5).unwrap(),
            "Wrong value for 101.5",
        );
        assert(
            0.0,
            interpolator.get_interpolated_sample("test", 102.5).unwrap(),
            "Wrong value for 102.5",
        );
        assert(
            0.0,
            interpolator.get_interpolated_sample("test", 103.5).unwrap(),
            "Wrong value for 103.5",
        );
    }

    struct DCSampleProvider {
        pub result: f32,
    }

    impl SampleProvider<&str, Error> for DCSampleProvider {
        fn get_sample(&self, channel_id: &str, _index: usize) -> Result<f32> {
            assert!(channel_id.eq("dc"));
            Ok(self.result)
        }
    }

    #[test]
    fn dc() {
        let interpolator = Interpolator::new(20, 200, DCSampleProvider { result: 0.75 });

        assert_eq!(
            0.75,
            interpolator.get_interpolated_sample("dc", 100.5).unwrap()
        );
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

        assert_eq!(
            0.0,
            interpolator.get_interpolated_sample("test", 0.0).unwrap()
        );

        assert_eq!(
            ErrorKind::BrokenPipe,
            interpolator
                .get_interpolated_sample("test", 3.0)
                .unwrap_err()
                .kind()
        );
        assert_eq!(
            ErrorKind::BrokenPipe,
            interpolator
                .get_interpolated_sample("test", 2.1)
                .unwrap_err()
                .kind()
        );
        assert_eq!(
            ErrorKind::BrokenPipe,
            interpolator
                .get_interpolated_sample("test", 3.1)
                .unwrap_err()
                .kind()
        );
    }

    const NUM_SAMPLES_IN_OUTPUT: usize = 120;

    trait FloatIndexSampleProvider {
        fn get_sample_float(&self, index: f32) -> f32;
    }

    fn print_waveforms<TSampleProvider, TChannelId>(
        start: f32,
        end: f32,
        channel_id: TChannelId,
        sample_provider: Box<dyn FloatIndexSampleProvider>,
        interpolator: &Interpolator<TSampleProvider, TChannelId, Error>,
    ) where
        TSampleProvider: SampleProvider<TChannelId, Error>,
        TChannelId: Copy,
    {
        let incr = (end - start) / (NUM_SAMPLES_IN_OUTPUT as f32);

        let mut sample_provider_0 = String::with_capacity(NUM_SAMPLES_IN_OUTPUT);
        let mut sample_provider_1 = String::with_capacity(NUM_SAMPLES_IN_OUTPUT);
        let mut sample_provider_2 = String::with_capacity(NUM_SAMPLES_IN_OUTPUT);
        let mut sample_provider_3 = String::with_capacity(NUM_SAMPLES_IN_OUTPUT);
        let mut sample_provider_4 = String::with_capacity(NUM_SAMPLES_IN_OUTPUT);
        let mut sample_provider_5 = String::with_capacity(NUM_SAMPLES_IN_OUTPUT);
        let mut sample_provider_6 = String::with_capacity(NUM_SAMPLES_IN_OUTPUT);
        let mut sample_provider_7 = String::with_capacity(NUM_SAMPLES_IN_OUTPUT);
        let mut sample_provider_8 = String::with_capacity(NUM_SAMPLES_IN_OUTPUT);
        let mut sample_provider_9 = String::with_capacity(NUM_SAMPLES_IN_OUTPUT);

        let mut interpolator_0 = String::with_capacity(NUM_SAMPLES_IN_OUTPUT);
        let mut interpolator_1 = String::with_capacity(NUM_SAMPLES_IN_OUTPUT);
        let mut interpolator_2 = String::with_capacity(NUM_SAMPLES_IN_OUTPUT);
        let mut interpolator_3 = String::with_capacity(NUM_SAMPLES_IN_OUTPUT);
        let mut interpolator_4 = String::with_capacity(NUM_SAMPLES_IN_OUTPUT);
        let mut interpolator_5 = String::with_capacity(NUM_SAMPLES_IN_OUTPUT);
        let mut interpolator_6 = String::with_capacity(NUM_SAMPLES_IN_OUTPUT);
        let mut interpolator_7 = String::with_capacity(NUM_SAMPLES_IN_OUTPUT);
        let mut interpolator_8 = String::with_capacity(NUM_SAMPLES_IN_OUTPUT);
        let mut interpolator_9 = String::with_capacity(NUM_SAMPLES_IN_OUTPUT);

        let mut index = start;
        while {
            let sample_provider_sample = sample_provider.get_sample_float(index);
            sample_provider_0.push_str(if sample_provider_sample < -0.8 {
                "*"
            } else {
                " "
            });
            sample_provider_1.push_str(
                if sample_provider_sample >= -0.8 && sample_provider_sample < -0.6 {
                    "*"
                } else {
                    " "
                },
            );
            sample_provider_2.push_str(
                if sample_provider_sample >= -0.6 && sample_provider_sample < -0.4 {
                    "*"
                } else {
                    " "
                },
            );
            sample_provider_3.push_str(
                if sample_provider_sample >= -0.4 && sample_provider_sample < -0.2 {
                    "*"
                } else {
                    " "
                },
            );
            sample_provider_4.push_str(
                if sample_provider_sample >= -0.2 && sample_provider_sample < 0.0 {
                    "*"
                } else {
                    " "
                },
            );
            sample_provider_5.push_str(
                if sample_provider_sample >= 0.0 && sample_provider_sample < 0.2 {
                    "*"
                } else {
                    " "
                },
            );
            sample_provider_6.push_str(
                if sample_provider_sample >= 0.2 && sample_provider_sample < 0.4 {
                    "*"
                } else {
                    " "
                },
            );
            sample_provider_7.push_str(
                if sample_provider_sample >= 0.4 && sample_provider_sample < 0.6 {
                    "*"
                } else {
                    " "
                },
            );
            sample_provider_8.push_str(
                if sample_provider_sample >= 0.6 && sample_provider_sample < 0.8 {
                    "*"
                } else {
                    " "
                },
            );
            sample_provider_9.push_str(if sample_provider_sample >= 0.8 {
                "*"
            } else {
                " "
            });

            let interpolator_sample = interpolator
                .get_interpolated_sample(channel_id, index)
                .unwrap();
            interpolator_0.push_str(if interpolator_sample < -0.8 {
                "*"
            } else {
                " "
            });
            interpolator_1.push_str(
                if interpolator_sample >= -0.8 && interpolator_sample < -0.6 {
                    "*"
                } else {
                    " "
                },
            );
            interpolator_2.push_str(
                if interpolator_sample >= -0.6 && interpolator_sample < -0.4 {
                    "*"
                } else {
                    " "
                },
            );
            interpolator_3.push_str(
                if interpolator_sample >= -0.4 && interpolator_sample < -0.2 {
                    "*"
                } else {
                    " "
                },
            );
            interpolator_4.push_str(
                if interpolator_sample >= -0.2 && interpolator_sample < 0.0 {
                    "*"
                } else {
                    " "
                },
            );
            interpolator_5.push_str(
                if interpolator_sample >= 0.0 && interpolator_sample < 0.2 {
                    "*"
                } else {
                    " "
                },
            );
            interpolator_6.push_str(
                if interpolator_sample >= 0.2 && interpolator_sample < 0.4 {
                    "*"
                } else {
                    " "
                },
            );
            interpolator_7.push_str(
                if interpolator_sample >= 0.4 && interpolator_sample < 0.6 {
                    "*"
                } else {
                    " "
                },
            );
            interpolator_8.push_str(
                if interpolator_sample >= 0.6 && interpolator_sample < 0.8 {
                    "*"
                } else {
                    " "
                },
            );
            interpolator_9.push_str(if interpolator_sample >= 0.8 {
                "*"
            } else {
                " "
            });

            index += incr;
            index < end
        } {}

        let bar = "-".repeat(NUM_SAMPLES_IN_OUTPUT);

        println!("Expected");
        println!("{}", bar);
        println!("{}", sample_provider_0);
        println!("{}", sample_provider_1);
        println!("{}", sample_provider_2);
        println!("{}", sample_provider_3);
        println!("{}", sample_provider_4);
        println!("{}", sample_provider_5);
        println!("{}", sample_provider_6);
        println!("{}", sample_provider_7);
        println!("{}", sample_provider_8);
        println!("{}", sample_provider_9);
        println!("{}", bar);
        println!();
        println!("Actual");
        println!("{}", bar);
        println!("{}", interpolator_0);
        println!("{}", interpolator_1);
        println!("{}", interpolator_2);
        println!("{}", interpolator_3);
        println!("{}", interpolator_4);
        println!("{}", interpolator_5);
        println!("{}", interpolator_6);
        println!("{}", interpolator_7);
        println!("{}", interpolator_8);
        println!("{}", interpolator_9);
        println!("{}", bar);
    }

    struct FourSampleWavelengthSignalProvider {}

    fn get_four_sample_wavelength_sample(x: f32) -> f32 {
        let arg = x * PI / 2.0;
        let y = arg.cos();
        y
    }

    impl FloatIndexSampleProvider for FourSampleWavelengthSignalProvider {
        fn get_sample_float(&self, index: f32) -> f32 {
            get_four_sample_wavelength_sample(index)
        }
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

        print_waveforms(
            0.0,
            50.0,
            "test",
            Box::new(FourSampleWavelengthSignalProvider {}),
            &interpolator,
        );

        let expected = get_four_sample_wavelength_sample(10.2);
        let actual = interpolator.get_interpolated_sample("test", 10.2).unwrap();

        assert(expected, actual, "Wrong value for a four-sample window");
    }

    struct SignalSampleProvider {}

    fn get_signal_sample(x: f32) -> f32 {
        let y = x.sin() + (x / 3.0).sin() + (x / 1.6).sin() + (x / 5.2).cos();
        y / 4.0
    }

    impl FloatIndexSampleProvider for SignalSampleProvider {
        fn get_sample_float(&self, index: f32) -> f32 {
            get_signal_sample(index)
        }
    }

    impl SampleProvider<&str, Error> for SignalSampleProvider {
        fn get_sample(&self, channel_id: &str, index: usize) -> Result<f32> {
            assert!(channel_id.eq("test"));

            Ok(get_signal_sample(index as f32))
        }
    }

    #[test]
    fn continuous_signal() {
        let interpolator = Interpolator::new(120, 2000, SignalSampleProvider {});

        print_waveforms(
            500.0,
            600.0,
            "test",
            Box::new(SignalSampleProvider {}),
            &interpolator,
        );

        let mut x = 500.0;
        while x <= 1500.0 {
            let expected_sample = get_signal_sample(x);
            let actual_sample = interpolator.get_interpolated_sample("test", x).unwrap();

            assert(
                expected_sample,
                actual_sample,
                &format!("When reading from a continuous sample at index {}", x),
            );
            //println!("Expected: {}, Actual: {} ({})", expected_sample, actual_sample, x);

            x += 0.01;
        }
    }
}
