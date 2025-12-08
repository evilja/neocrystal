use std::thread;
use std::time::{Duration};
use std::fs::File;
use std::io::BufReader;
use rodio::*;
use std::sync::mpsc::{Receiver, Sender};
use std::path::Path;
use mp3_duration;

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
    Pause,
    Duration(String, Duration),
}


pub fn play_audio(receiver: Receiver<AudioCommand>, transmitter: Sender<AudioReportAction>) -> Result<String, Box<dyn std::error::Error>> {
    let stream_handle: OutputStream     = rodio::OutputStreamBuilder::open_default_stream().expect("open default audio stream");
    let sink                            = rodio::Sink::connect_new(&stream_handle.mixer());
    let mut cached: String              = "uinit".to_string();
    let mut cached_duration: Duration   = Duration::ZERO;
    loop {
        if let Ok(ac_command) = receiver.recv_timeout(Duration::from_millis(200)) {
            match ac_command {
                AudioCommand::Pause => {
                    sink.pause();
                    transmitter.send(AudioReportAction::Pause)?; // send a time in the past to indicate paused state
                },
                AudioCommand::Resume => {
                    if cached == "uinit" {
                        continue;
                    }
                    sink.play();
                    match transmitter.send(AudioReportAction::Duration(cached.clone(), duration_autobuild(cached_duration, sink.get_pos()))) {
                        Ok(()) => (),
                        Err(_) => (),
                    }
                },

                AudioCommand::SetVolume(value) => {
                    sink.set_volume(value);
                },
                AudioCommand::Stop => {
                    sink.stop();
                    break;
                },
                AudioCommand::Play(value) => {
                    let file: File = File::open(value.clone())?;
                    let source: Decoder<BufReader<File>> = Decoder::new(BufReader::new(file))?;
                    sink.clear();

                    sink.append(source);
                    cached_duration = mp3_duration::from_path(Path::new(value.as_str()))?;
                    sink.play();
                    cached = value;
                },
                AudioCommand::SeekForward => {
                    if sink.get_pos() + Duration::from_secs(5) >= cached_duration && cached_duration != Duration::ZERO {
                        // If the new position is beyond the end of the track, seek to the end
                        let _ = sink.try_seek(cached_duration - Duration::from_secs(1));
                    } else {
                        match sink.try_seek(sink.get_pos()+Duration::from_secs(5)) {
                            _ => (),
                        }
                    }
                },
                AudioCommand::SeekBackward =>{
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
                },
            }
        }
        if cached == "uinit".to_string() || sink.is_paused() {
            continue;
        }
        thread::sleep(Duration::from_millis(100));
        match transmitter.send(AudioReportAction::Duration(cached.clone(), duration_autobuild(cached_duration, sink.get_pos()))) {
            Ok(()) => (),
            Err(_) => (),
        }
    }
    Ok("Stopped".to_string())
}


fn duration_autobuild(cached: Duration, pos: Duration) -> Duration {
    if pos >= cached {
        return Duration::ZERO;
    }
    cached - pos

} 
