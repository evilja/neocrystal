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
    let mut cached: String = "uinit".to_string();
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
                "stop" => {
                    sink.stop();
                    break;
                },
                "volume_df" => {
                    sink.set_volume(0.5);
                },
                "volume_up" => {
                    match sink.volume() {            // these manual updates look dumb but volume +-0.1 doesn't work as intended
                        0.0 => sink.set_volume(0.1), // because you can't represent 0.1 using a.2^x with a,x being integer (gives weird float values)
                        0.1 => sink.set_volume(0.2), // ruining the volume control entirely.
                        0.2 => sink.set_volume(0.3),
                        0.3 => sink.set_volume(0.4),
                        0.4 => sink.set_volume(0.5),
                        0.5 => sink.set_volume(0.6),
                        0.6 => sink.set_volume(0.7),
                        0.7 => sink.set_volume(0.8),
                        0.8 => sink.set_volume(0.9),
                        0.9 => sink.set_volume(1.0),
                        1.0 => (),
                        _ => sink.set_volume(0.5),
                    }
                    
                },
                "volume_down" => {
                    match sink.volume() {
                        1.0 => sink.set_volume(0.9),
                        0.9 => sink.set_volume(0.8),
                        0.8 => sink.set_volume(0.7),
                        0.7 => sink.set_volume(0.6),
                        0.6 => sink.set_volume(0.5),
                        0.5 => sink.set_volume(0.4),
                        0.4 => sink.set_volume(0.3),
                        0.3 => sink.set_volume(0.2),
                        0.2 => sink.set_volume(0.1),
                        0.1 => sink.set_volume(0.0),
                        0.0 => (),
                        _ => sink.set_volume(0.5),
                    }
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
                _ => return Err("Unknown command".into()),
            }
        }
    }
    Ok("Stopped".to_string())
}
