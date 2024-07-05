use crate::pulseaudio::AudioOutput;
use crate::pulseaudio::Result;
use symphonia::core::audio::AudioBufferRef;
use cpal::traits::HostTrait;
use cpal::traits::DeviceTrait;


pub struct CpalAudioOutput {

}

impl AudioOutput for CpalAudioOutput {
    fn write(&mut self, decoded: AudioBufferRef<'_>) -> crate::pulseaudio::Result<()> {
        todo!()
    }

    fn flush(&mut self) {
        todo!()
    }
}

fn init_cpal() -> (cpal::Device, cpal::SupportedStreamConfig) {
    let device = cpal::default_host().default_output_device()
        .expect("no output device available");

    // Create an output stream for the audio so we can play it
    // NOTE: If system doesn't support the file's sample rate, the program will panic when we try to play,
    //       so we'll need to resample the audio to a supported config
    let supported_config_range = device.supported_output_configs()
        .expect("error querying audio output configs")
        .next()
        .expect("no supported audio config found");

    // Pick the best (highest) sample rate
    (device, supported_config_range.with_max_sample_rate())
}


