use std::thread::sleep;
use symphonia::core::{audio::SignalSpec, codecs::DecoderOptions, errors::{Error, Result}, formats::{FormatOptions, FormatReader, SeekMode, SeekTo, Track}, io::MediaSourceStream, meta::MetadataOptions, probe::Hint, units::{Duration, Time}};
use symphonia::core::codecs::CODEC_TYPE_NULL;
use crossbeam_channel::{Receiver, Sender};

use crate::{url_source::UrlSource, pulseaudio::{self, AudioOutput, PulseAudioOutput}};

#[derive(PartialEq, Clone, Debug)]
pub enum PlayerActions {
    Pause,
    Resume,
    Seek(f64),
}

#[derive(PartialEq, Clone, Debug)]
pub enum PlayerStatus {
    SendPlaying(bool),
    SendDuration(f64),
    SendPosition(f64)
}

pub struct PlayerEngine {
    reader: Option<Box<dyn FormatReader>>,
    rx: Receiver<PlayerActions>,
    // tx: Sender<PlayerActions>,
    // rx_status: Receiver<PlayerStatus>,
    tx_status: Sender<PlayerStatus>,
}

#[derive(Clone, Debug)]
pub struct PlayerState {
    pub playing: bool,
    pub duration: f64,
    pub position: f64
}

impl PlayerEngine {
    pub fn new(
        // tx: Sender<PlayerActions>,
        rx: Receiver<PlayerActions>,
        tx_status: Sender<PlayerStatus>,
        // rx_status: Receiver<PlayerStatus>
        ) -> Self {
        Self {
            reader: None,
            rx,
            // tx,
            tx_status,
            // rx_status
        }
    }

    pub fn open(&mut self, path: &str) -> Result<i32> {
        let r = UrlSource::new(path);
        // let source = Box::new(ReadOnlySource::new(r));
        let source = Box::new(r);

        let hint = Hint::new();
        let mss = MediaSourceStream::new(source, Default::default());

        let format_opts =
            FormatOptions { enable_gapless: true, ..Default::default() };
        let metadata_opts: MetadataOptions = Default::default();
        let track = None;

        match symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts) {
            Ok(probed) => {
                let decode_opts = Default::default();
                self.reader = Some(probed.format);
                self.play(track, &decode_opts)
            },
            Err(e) => {
                println!("input not supported: {:#?}", e);
                Err(e)
            },
        }
    }

    fn play(&mut self,
        track_num: Option<usize>,
        decode_opts: &DecoderOptions,
    ) -> Result<i32> {

        if let Some(reader) = self.reader.as_mut() {

            let track = track_num
                .and_then(|t| reader.tracks().get(t))
                .or_else(|| first_supported_track(reader.tracks()));

            let track_id = track.unwrap().id;
            let mut audio_output = None;
            let result = self.play_track(
                &mut audio_output,
                track_id,
                decode_opts);

            // Flush the audio output to finish playing back any leftover samples.
            if let Some(audio_output) = audio_output.as_mut() {
                audio_output.flush()
            }
            result
        } else {

            Ok(0)
        }
    }

    fn play_track(
        &mut self,
        audio_output: &mut Option<Box<dyn AudioOutput>>,
        track_id: u32,
        decode_opts: &DecoderOptions,
    ) -> Result<i32> {

        let rx = self.rx.clone();
        let (tb, dur, mut decoder) = if let Some(r) = self.reader.as_mut() {
            let track = match r.tracks().iter().find(|track| track.id == track_id) {
                Some(track) => track,
                _ => return Ok(0),
            };

            // Create a decoder for the track.
            let decoder = symphonia::default::get_codecs().make(&track.codec_params, decode_opts)?;

            // Get the selected track's timebase and duration.
            let tb = track.codec_params.time_base;
            let dur = track.codec_params.n_frames.map(|frames| track.codec_params.start_ts + frames);
            (tb, dur, decoder)
        } else {
            return Err(symphonia::core::errors::Error::IoError(std::io::Error::new(std::io::ErrorKind::Other, "")));
        };

        let mut playing = true;
        // Decode and play the packets belonging to the selected track.
        let result = loop {
            let action = match rx.try_recv() {
                Ok(a) => {
                    println!("Action recv: {:#?}", a);
                    Some(a)
                },
                Err(_e) => {
                    None
                },
            };

            let a = action.clone();
            if a.is_some() && (a.unwrap() == PlayerActions::Pause) {
                playing = false;
                let s = self.tx_status.send(PlayerStatus::SendPlaying(false));
            }

            let a = action.clone();
            if a.is_some() && (a.unwrap() == PlayerActions::Resume) {
                playing = true;
                let s = self.tx_status.send(PlayerStatus::SendPlaying(true));
            }

            {
                if !playing {
                    sleep(std::time::Duration::from_millis(200));
                    continue;
                }
            }

            // Get the next packet from the format reader.
            let packet = if let Some(reader) = self.reader.as_mut() {
                match reader.next_packet() {
                    Ok(packet) => packet,
                    Err(err) => break Err(err),
                }
            } else {
                return Err(symphonia::core::errors::Error::IoError(std::io::Error::new(std::io::ErrorKind::Other, "")));
            };

            // If the packet does not belong to the selected track, skip it.
            if packet.track_id() != track_id {
                continue;
            }

            // Decode the packet into audio samples.
            match decoder.decode(&packet) {
                Ok(decoded) => {
                    // If the audio output is not open, try to open it.
                    if audio_output.is_none() {
                        // Get the audio buffer specification. This is a description of the decoded
                        // audio buffer's sample format and sample rate.
                        let spec = *decoded.spec();

                        // Get the capacity of the decoded buffer. Note that this is capacity, not
                        // length! The capacity of the decoded buffer is constant for the life of the
                        // decoder, but the length is not.
                        let duration = decoded.capacity() as u64;

                        // Try to open the audio output.
                        audio_output.replace(try_open(spec, duration).unwrap());
                    }
                    else {
                        // TODO: Check the audio spec. and duration hasn't changed.
                    }

                    let ts = packet.ts();
                    let (position, duration) = update_progress(ts, dur, tb);
                    {
                        let _ = self.tx_status.send(PlayerStatus::SendDuration(duration));
                        let _ = self.tx_status.send(PlayerStatus::SendPosition(position));
                    }

                    if let Some(audio_output) = audio_output {
                        audio_output.write(decoded).unwrap()
                    }

                    let a = action.clone();
                    if a.is_some() {
                        let a = a.as_ref().unwrap();
                        match a{
                            PlayerActions::Seek(ref t) => {
                                let ts: Time = t.clone().into(); // packet.ts() + 30;
                                if let Some(reader) = self.reader.as_mut() {
                                    let r = reader.seek(SeekMode::Accurate, SeekTo::Time{ time: ts, track_id: Some(0) });
                                    println!("Seek result: {:#?}", r);
                                }
                            },
                            _ => {}
                        }
                    }
                }

                Err(Error::DecodeError(err)) => {
                    // Decode errors are not fatal. Print the error message and try to decode the next
                    // packet as usual.
                    println!("decode error: {}", err);
                }
                Err(err) => break Err(err),
            }
        };

        // Return if a fatal error occured.
        ignore_end_of_stream_error(result)?;
        Ok(0)
    }


    // pub fn position_display(&self) -> String {
    //     let position = self.state.read().unwrap().position;
    //     let hours = position / (60.0 * 60.0);
    //     let mins = (position as u64 % (60 * 60)) / 60;
    //     let secs = (position as u64 % 60) as f64 + (position - (position as u64) as f64);
    //     format!("{}:{:0>2}:{:0>4.1}", hours, mins, secs)
    // }

    fn print_progress(&mut self, ts: u64, dur: Option<u64>, tb: Option<symphonia::core::formats::prelude::TimeBase>) {
        if let Some(tb) = tb {
            let t = tb.calc_time(ts);

            let hours = t.seconds / (60 * 60);
            let mins = (t.seconds % (60 * 60)) / 60;
            let secs = f64::from((t.seconds % 60) as u32) + t.frac;

            println!("\r\u{25b6}\u{fe0f}  {}:{:0>2}:{:0>4.1}", hours, mins, secs);

            let d = tb.calc_time(dur.unwrap_or(0));

            let hours = d.seconds / (60 * 60);
            let mins = (d.seconds % (60 * 60)) / 60;
            let secs = f64::from((d.seconds % 60) as u32) + d.frac;

            println!("::::> {}:{:0>2}:{:0>4.1}", hours, mins, secs);
        }
    }

}

fn ignore_end_of_stream_error(result: Result<()>) -> Result<()> {
    match result {
        Err(Error::IoError(err))
            if err.kind() == std::io::ErrorKind::UnexpectedEof
                && err.to_string() == "end of stream" =>
            {
                // Do not treat "end of stream" as a fatal error. It's the currently only way a
                // format reader can indicate the media is complete.
                Ok(())
            }
        _ => result,
    }
}

pub fn try_open(spec: SignalSpec, duration: Duration) -> pulseaudio::Result<Box<dyn AudioOutput>> {
    PulseAudioOutput::try_open(spec, duration)
}

fn update_progress(ts: u64, dur: Option<u64>, tb: Option<symphonia::core::formats::prelude::TimeBase>) -> (f64, f64) {
    if let Some(tb) = tb {
        let t = tb.calc_time(ts);
        let position = t.frac + t.seconds as f64;

        let d = tb.calc_time(dur.unwrap_or(0));
        let duration = d.frac + d.seconds as f64;

        (position, duration)
    } else {
        (0.0, 0.0)
    }
}

fn first_supported_track(tracks: &[Track]) -> Option<&Track> {
    tracks.iter().find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
}
