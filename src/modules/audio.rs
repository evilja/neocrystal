use std::time::{Duration, Instant};
use std::fs::File;
use std::io::BufReader;
use rodio::*;
use std::sync::mpsc::{Receiver, Sender};
use std::path::Path;
use mp3_duration;



pub fn play_audio(receiver: Receiver<(&'static str, String)>, transmitter: Sender<Instant>) -> Result<String, Box<dyn std::error::Error>> {
    let stream_handle: OutputStream     = rodio::OutputStreamBuilder::open_default_stream().expect("open default audio stream");
    let sink                            = rodio::Sink::connect_new(&stream_handle.mixer());
    let mut cached: String              = "uinit".to_string();
    loop {
        if let Ok((command, value)) = receiver.recv_timeout(Duration::from_millis(100)) {
            match command {
                "pause" => {
                    sink.pause();
                    transmitter.send(Instant::now())?; // send a time in the past to indicate paused state
                },
                "resume" => {
                    sink.play();
                    match transmitter.send(Instant::now() + mp3_duration::from_path(Path::new(cached.as_str())).unwrap() + Duration::from_secs(2) - sink.get_pos()) {
                        Ok(()) => (),
                        Err(_) => (),
                    }
                },

                "set_volume" => {
                    sink.set_volume(value.parse::<f32>().unwrap_or(0.5));
                },
                "stop" => {
                    sink.stop();
                    break;
                },
                "volume_df" => {
                    sink.set_volume(0.5);
                },
                "play_track" => {
                    let file: File = File::open(value.clone())?;
                    let source: Decoder<BufReader<File>> = Decoder::new(BufReader::new(file))?;
                    sink.clear();

                    sink.append(source);
                    let when_ends: Instant = Instant::now() + mp3_duration::from_path(Path::new(value.as_str()))? + Duration::from_secs(2);
                    match transmitter.send(when_ends) {
                        Ok(()) => (),
                        Err(_) => (),
                    }
                    sink.play();
                    cached = value;
                },
                "forward" => {
                    sink.try_seek(sink.get_pos()+Duration::from_secs(5))?;
                    match transmitter.send(Instant::now() + mp3_duration::from_path(Path::new(cached.as_str())).unwrap() + Duration::from_secs(2) - sink.get_pos()) {
                        Ok(()) => (),
                        Err(_) => (),
                    }
                },
                "back" =>{
                    let cachegetpos = sink.get_pos();
                    let file: File = File::open(cached.clone())?;
                    let source: Decoder<BufReader<File>> = Decoder::new(BufReader::new(file))?;
                    sink.clear();

                    sink.append(source);
                    if cachegetpos <= Duration::from_secs(5) {
                        sink.try_seek(Duration::from_secs(0))?;
                    } else {
                        sink.try_seek(cachegetpos - Duration::from_secs(5))?;
                    }
                    match transmitter.send(Instant::now() + mp3_duration::from_path(Path::new(cached.as_str())).unwrap() + Duration::from_secs(2) - sink.get_pos()) {
                        Ok(()) => (),
                        Err(_) => (),
                    }
                    sink.play();
                },
                _ => return Err("Unknown command".into()),
            }
        }
    }
    Ok("Stopped".to_string())
}
