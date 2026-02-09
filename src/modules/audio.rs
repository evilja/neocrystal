use rodio::*;
use std::io::BufReader;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;
use symphonia::core::{
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};
use std::fs::File;

pub fn audio_duration(path: &str) -> Duration {
    let file = File::open(path).ok().unwrap();
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.split('.').last() {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .ok().unwrap();

    let format = probed.format;
    let track = format.default_track().unwrap();

    let sample_rate = track.codec_params.sample_rate.unwrap();
    let frames = track.codec_params.n_frames.unwrap();

    Duration::from_secs_f64(frames as f64 / sample_rate as f64)
}



pub enum AudioCommand {
    Play(String),
    SetVolume(f32),
    Pause,
    Resume,
    SeekForward,
    SeekBackward,
    Stop,
}

pub enum AudioReportAction {
    EOF,
    Pause,
    Duration(String, Duration),
}

struct Orchestrator { r: bool }
impl Orchestrator { 
    pub fn res(&mut self) { self.r = false; } 
    pub fn act(&mut self) -> bool { if self.r { return false } else { self.r = true; return true } }
}

pub fn play_audio(
    receiver: Receiver<AudioCommand>,
    transmitter: Sender<AudioReportAction>,
) -> Result<String, Box<dyn std::error::Error>> {
    let stream_handle: OutputStream =
        rodio::OutputStreamBuilder::open_default_stream().expect("open default audio stream");
    let sink = rodio::Sink::connect_new(&stream_handle.mixer());
    let mut cached: String = "uinit".to_string();
    let mut orc = Orchestrator { r: false };
    let mut cached_duration: Duration = Duration::ZERO;
    let mut sleep_duration: u8 = 0;
    loop {
        if let Ok(ac_command) = receiver.recv_timeout(Duration::from_millis(200)) {
            match ac_command {
                AudioCommand::Pause => {
                    sink.pause();
                    transmitter.send(AudioReportAction::Pause)?; // send a time in the past to indicate paused state
                }
                AudioCommand::Resume => {
                    if cached == "uinit" {
                        continue;
                    }
                    sink.play();
                    sleep_duration = 100;
                }

                AudioCommand::SetVolume(value) => {
                    sink.set_volume(value);
                }
                AudioCommand::Stop => {
                    sink.stop();
                    break;
                }
                AudioCommand::Play(value) => {
                    let file: File = File::open(value.clone())?;
                    let source: Decoder<BufReader<File>> = Decoder::new(BufReader::new(file))?;
                    sink.clear();
                    thread::sleep(Duration::from_millis(20));
                    sink.append(source);
                    thread::sleep(Duration::from_millis(20));
                    cached_duration = audio_duration(&value);
                    sink.play();
                    cached = value;
                    sleep_duration = 100;
                    orc.res();
                }
                AudioCommand::SeekForward => {
                    if sink.get_pos() + Duration::from_secs(5) >= cached_duration
                        && cached_duration != Duration::ZERO
                    {
                        // If the new position is beyond the end of the track, seek to the end
                        let _ = sink.try_seek(cached_duration - Duration::from_secs(1));
                    } else {
                        match sink.try_seek(sink.get_pos() + Duration::from_secs(5)) {
                            _ => (),
                        }
                    }
                }
                AudioCommand::SeekBackward => {
                    if cached == "uinit".to_string() {
                        continue;
                    }
                    let cachegetpos = sink.get_pos();
                    let file: File = File::open(cached.clone())?;
                    let source: Decoder<BufReader<File>> = Decoder::new(BufReader::new(file))?;
                    sink.clear();

                    sink.append(source);
                    if cachegetpos <= Duration::from_secs(5) {
                        ();
                    } else {
                        sink.try_seek(cachegetpos - Duration::from_secs(5))?;
                    }
                    sink.play();
                }
            }
        }
        if cached == "uinit".to_string() || sink.is_paused() {
            continue;
        }
        thread::sleep(Duration::from_millis(sleep_duration as u64));
        if sink.empty() && orc.act() && !sink.is_paused() {
            transmitter.send(AudioReportAction::EOF).unwrap();
        } else if !sink.is_paused() {
            transmitter.send(AudioReportAction::Duration(
                cached.clone(), 
                duration_autobuild(cached_duration, sink.get_pos()),
            ))?;
        }
        sleep_duration = 0;
    }
    Ok("Stopped".to_string())
}

fn duration_autobuild(cached: Duration, pos: Duration) -> Duration {
    if pos >= cached {
        return Duration::ZERO;
    }
    cached - pos
}


