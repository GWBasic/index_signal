pub mod interpolator;

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        f32::consts::PI,
        fs,
        io::{Error, ErrorKind, Result},
        path::Path,
    };

    use super::*;

    use interpolator::{Interpolator, SampleProvider};
    use wave_stream::{
        read_wav_from_file_path,
        samples_by_channel::SamplesByChannel,
        wave_header::{Channels, SampleFormat, WavHeader},
        wave_reader::{RandomAccessOpenWavReader, RandomAccessWavReader},
        write_wav_to_file_path,
    };

    fn assert(expected: f32, actual: f32, error_message: &str) {
        // Note: 24-bit audio differentiates samples at 0.00000012 precision
        let difference = (expected - actual).abs();

        // 24-bit accuracy: 0.00000012 = 1 / (2^24)
        // 16-bit accuracy: 0.00001526 = 1 / (2^16)
        // 8-bit accuracy:  0.00390625 = 1 / (2^8)

        if difference > 0.001 {
            panic!(
                "{}: Expected: {}, Actual: {}, Difference: {}",
                error_message, expected, actual, difference
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
            interpolator
                .get_interpolated_sample("test", 0.0, 0.0)
                .unwrap()
        );
        assert_eq!(
            -1.0,
            interpolator
                .get_interpolated_sample("test", 1.0, 0.0)
                .unwrap()
        );
        assert_eq!(
            1.0,
            interpolator
                .get_interpolated_sample("test", 2.0, 0.0)
                .unwrap()
        );
        assert_eq!(
            -1.0,
            interpolator
                .get_interpolated_sample("test", 3.0, 0.0)
                .unwrap()
        );
    }

    #[test]
    fn partial_sample_nyquist() {
        let interpolator = Interpolator::new(20, 200, NyquistSampleProvider {});

        assert(
            0.0,
            interpolator
                .get_interpolated_sample("test", 100.5, 0.0)
                .unwrap(),
            "Wrong value for 100.5",
        );
        assert(
            0.0,
            interpolator
                .get_interpolated_sample("test", 101.5, 0.0)
                .unwrap(),
            "Wrong value for 101.5",
        );
        assert(
            0.0,
            interpolator
                .get_interpolated_sample("test", 102.5, 0.0)
                .unwrap(),
            "Wrong value for 102.5",
        );
        assert(
            0.0,
            interpolator
                .get_interpolated_sample("test", 103.5, 0.0)
                .unwrap(),
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
            interpolator
                .get_interpolated_sample("dc", 100.5, 0.0)
                .unwrap()
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
            interpolator
                .get_interpolated_sample("test", 0.0, 0.0)
                .unwrap()
        );

        assert_eq!(
            ErrorKind::BrokenPipe,
            interpolator
                .get_interpolated_sample("test", 3.0, 0.0)
                .unwrap_err()
                .kind()
        );
        assert_eq!(
            ErrorKind::BrokenPipe,
            interpolator
                .get_interpolated_sample("test", 2.1, 0.0)
                .unwrap_err()
                .kind()
        );
        assert_eq!(
            ErrorKind::BrokenPipe,
            interpolator
                .get_interpolated_sample("test", 3.1, 0.0)
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
        relative_speed: f32,
        interpolator: &Interpolator<TSampleProvider, TChannelId, Error>,
    ) where
        TSampleProvider: SampleProvider<TChannelId, Error>,
        TChannelId: Copy + std::cmp::Eq + std::hash::Hash,
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

        print_actual_waveform(start, end, channel_id, relative_speed, interpolator);
    }

    fn print_actual_waveform<TSampleProvider, TChannelId>(
        start: f32,
        end: f32,
        channel_id: TChannelId,
        relative_speed: f32,
        interpolator: &Interpolator<TSampleProvider, TChannelId, Error>,
    ) where
        TSampleProvider: SampleProvider<TChannelId, Error>,
        TChannelId: Copy + std::cmp::Eq + std::hash::Hash,
    {
        let incr = (end - start) / (NUM_SAMPLES_IN_OUTPUT as f32);

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
            let interpolator_sample = interpolator
                .get_interpolated_sample(channel_id, index, relative_speed)
                .unwrap();
            interpolator_0.push_str(if interpolator_sample < -0.8 { "*" } else { " " });
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
            interpolator_5.push_str(if interpolator_sample >= 0.0 && interpolator_sample < 0.2 {
                "*"
            } else {
                " "
            });
            interpolator_6.push_str(if interpolator_sample >= 0.2 && interpolator_sample < 0.4 {
                "*"
            } else {
                " "
            });
            interpolator_7.push_str(if interpolator_sample >= 0.4 && interpolator_sample < 0.6 {
                "*"
            } else {
                " "
            });
            interpolator_8.push_str(if interpolator_sample >= 0.6 && interpolator_sample < 0.8 {
                "*"
            } else {
                " "
            });
            interpolator_9.push_str(if interpolator_sample >= 0.8 { "*" } else { " " });

            index += incr;
            index < end
        } {}

        let bar = "-".repeat(NUM_SAMPLES_IN_OUTPUT);

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
            0.0,
            &interpolator,
        );

        let expected = get_four_sample_wavelength_sample(10.2);
        let actual = interpolator
            .get_interpolated_sample("test", 10.2, 0.0)
            .unwrap();

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
            0.0,
            &interpolator,
        );

        let mut x = 500.0;
        while x <= 1500.0 {
            let expected_sample = get_signal_sample(x);
            let actual_sample = interpolator
                .get_interpolated_sample("test", x, 0.0)
                .unwrap();

            assert(
                expected_sample,
                actual_sample,
                &format!("When reading from a continuous sample at index {}", x),
            );

            x += 0.01;
        }
    }

    #[derive(Debug, Copy, Clone)]
    struct SineSignalProvider {
        wavelength_in_samples: f32,
    }

    impl SineSignalProvider {
        fn get_sine_signal_sample(&self, x: f32) -> f32 {
            let arg = x * (PI / (self.wavelength_in_samples / 2.0));
            // The peak of the waveform must correlate *exactly* with a sample. If the peak isn't exactly on a sample,
            // then the amplitude will be softer than intended
            // .cos() ensures that the sample at 0 is always 1.0
            let y = arg.cos();
            y
        }
    }

    impl FloatIndexSampleProvider for SineSignalProvider {
        fn get_sample_float(&self, index: f32) -> f32 {
            self.get_sine_signal_sample(index)
        }
    }

    impl SampleProvider<&str, Error> for SineSignalProvider {
        fn get_sample(&self, channel_id: &str, index: usize) -> Result<f32> {
            assert!(channel_id.eq("test"));

            Ok(self.get_sine_signal_sample(index as f32))
        }
    }

    fn test_wavelength(wavelength_in_samples: f32) {
        let sine_signal_provider = SineSignalProvider {
            wavelength_in_samples,
        };

        let interpolator = Interpolator::new(8, 2000, sine_signal_provider);

        print_waveforms(
            500.0,
            510.0,
            "test",
            Box::new(sine_signal_provider),
            0.0,
            &interpolator,
        );

        let mut x = 500.0;
        while x <= 1500.0 {
            let expected_sample = sine_signal_provider.get_sine_signal_sample(x);
            let actual_sample = interpolator
                .get_interpolated_sample("test", x, 0.0)
                .unwrap();

            assert(
                expected_sample,
                actual_sample,
                &format!("When reading from a continuous sample at index {}", x),
            );

            x += 0.01;
        }
    }

    // The wavelength test must be a sin wave that fits within a frequency slot
    // 3, 5, 6, 7 won't work because they aren't an even multiple of the sampling rate

    #[test]
    fn wavelength_2_sample() {
        test_wavelength(2.0);
    }

    #[test]
    fn wavelength_4_sample() {
        test_wavelength(4.0);
    }

    #[test]
    fn wavelength_8_sample() {
        test_wavelength(8.0);
    }

    struct RandomAccessWavReaderSampleProvider {
        random_access_wav_reader: RefCell<RandomAccessWavReader<f32>>,
    }

    impl SampleProvider<&str, Error> for RandomAccessWavReaderSampleProvider {
        fn get_sample(&self, _channel_id: &str, index: usize) -> std::result::Result<f32, Error> {
            let mut random_access_wav_reader = self.random_access_wav_reader.borrow_mut();
            let read_sample_result = random_access_wav_reader.read_sample(index);
            let samples_by_channel = read_sample_result?;
            let sample = samples_by_channel
                .front_left
                .expect("Can't read the sample");
            Ok(sample)
        }
    }

    #[test]
    fn wave_stream_supported() {
        let header = WavHeader {
            sample_format: SampleFormat::Float,
            channels: Channels::new().front_left(),
            sample_rate: 44100,
        };

        let samples = vec![
            0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, -1.0,
            0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, -1.0,
        ];

        {
            let open_wav_writer =
                write_wav_to_file_path(Path::new("delete_me.wav"), header).unwrap();

            let mut random_access_wave_writer =
                open_wav_writer.get_random_access_f32_writer().unwrap();

            for sample_ctr in 0..samples.len() {
                random_access_wave_writer
                    .write_samples(
                        sample_ctr,
                        SamplesByChannel::new().front_left(samples[sample_ctr]),
                    )
                    .unwrap()
            }
        }

        let open_wav_reader = read_wav_from_file_path(Path::new("delete_me.wav")).unwrap();
        let random_access_wav_reader_sample_provider = RandomAccessWavReaderSampleProvider {
            random_access_wav_reader: RefCell::new(
                open_wav_reader.get_random_access_f32_reader().unwrap(),
            ),
        };

        let interpolator =
            Interpolator::new(4, samples.len(), random_access_wav_reader_sample_provider);

        for sample_ctr in 0..samples.len() {
            let expected_sample = samples[sample_ctr];
            let actual_sample = interpolator
                .get_interpolated_sample("", sample_ctr as f32, 0.0)
                .unwrap();
            assert_eq!(
                expected_sample, actual_sample,
                "Wrong sample when reading from a wav file"
            );
        }

        fs::remove_file(Path::new("delete_me.wav")).unwrap();
    }

    #[test]
    fn aliasing_filter_removes_high_frequencies() {
        let sample_provider = NyquistSampleProvider {};

        let interpolator = Interpolator::new(10, 8000, sample_provider);

        // Test with relative_speed > 1 which should trigger anti-aliasing filter
        for sample_ctr in 200..300 {
            let actual_sample = interpolator
                .get_interpolated_sample("test", sample_ctr as f32, 2.0)
                .unwrap();
            assert!(
                actual_sample.abs() < 1e-6,
                "Sample should be approximately 0 due to anti-aliasing filter"
            );
        }
    }

    struct NyquistAndLowerHarmonicSampleProvider {}

    impl SampleProvider<&str, Error> for NyquistAndLowerHarmonicSampleProvider {
        fn get_sample(&self, channel_id: &str, index: usize) -> Result<f32> {
            assert!(channel_id.eq("test"));

            let mut sample = 0.0;

            if index % 2 == 0 {
                sample += 0.5;
            } else {
                sample -= 0.5;
            }

            match index % 4 {
                0 => sample += 0.5,
                1 => {}
                2 => sample -= 0.5,
                3 => {}
                _ => return Err(Error::new(ErrorKind::InvalidInput, "Unexpected input")),
            }

            Ok(sample)
        }
    }

    #[test]
    fn antialiasing_filter_keeps_lower_frequency() {
        let sample_provider = NyquistAndLowerHarmonicSampleProvider {};

        let interpolator = Interpolator::new(10, 8000, sample_provider);

        print_actual_waveform(200.0, 204.0, "test", 2.0, &interpolator);

        // Test with relative_speed > 1 which should trigger anti-aliasing filter
        for sample_ctr in 200..300 {

            let expected_sample = match sample_ctr % 4 {
                0 => 0.5,
                1 => 0.0,
                2 => -0.5,
                3 => 0.0,
                _ => panic!("CPU can't do math")
            };

            let actual_sample = interpolator
                .get_interpolated_sample("test", sample_ctr as f32, 2.0)
                .unwrap();
            assert(
                expected_sample,
                actual_sample,
                &format!(
                    "When reading from a continuous sample at index {}",
                    sample_ctr
                ),
            );
        }
    }
}
