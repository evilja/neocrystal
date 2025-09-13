
extern crate pancurses;
extern crate glob;
use std::thread;
use std::time::{Duration, Instant};
use std::fs::File;
use std::io::BufReader;
use rodio::*;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::path::Path;
use mp3_duration;
use pancurses::{initscr, Input};
use glob::glob;
use discord_presence::{Client};
#[derive(Clone)]
struct Songs {
    songs: Vec<String>,
    current_song: usize,
    current_name: String,
    typical_page_size: usize,
}

impl Songs {
    fn _all_songs(&self) -> Vec<String> {
        return self.songs.clone();
    }
    fn _current_index(&self) -> usize {
        return self.current_song.clone();
    }
    fn current_name(&self) -> String {
        return self.current_name.clone();
    }
    fn set_by_pindex(&mut self, index: usize, page: usize) -> usize {
        self.current_song = index + ((page-1) * self.typical_page_size);
        self.current_name = self.songs[self.current_song].clone();
        index
    }
    fn set_by_next(&mut self) -> usize {
        self.set_by_pindex(self.current_song+1, 1)
    }
    fn stop(&mut self) {
        self.current_song = 0;
        self.current_name = "Nothing".to_string();
    }
}

fn rpc_handler(comm_recv: Receiver<(String, &'static str)>) {
    let mut drpc = Client::new(1003981361079668829);
    drpc.on_ready(|_ctx| {
        println!("READY!");
    })
    .persist();

    drpc.on_error(|ctx| {
        eprintln!("An error occured, {:?}", ctx.event);
    })
    .persist();
    drpc.start();
    loop {
        match comm_recv.recv() {
            Ok((x, y)) => {
                if x == "stop" {
                    break;
                }
                loop {
                    match drpc.set_activity(|act| {
                    act.state(y)
                        .details(x.clone().replace("music/", "").replace("music\\", "").replace(".mp3", ""))
                        .assets(|ass| {
                            ass.small_image("github")
                                .small_text("github.com/evilja/neo-crystal-plus")
                                .large_image("default")
                                .large_text("Crystal+ by Myisha")
                        })
                    }) {
                        Ok(_) => break,
                        Err(_) => thread::sleep(Duration::from_secs(1)),
                    }
                }
            }
            Err(_) => thread::sleep(Duration::from_secs(1)),
        }
    }
}

#[inline]
fn calc() {}

fn crystal_manager(tx: Sender<(&'static str, String)>, comm_rx: Receiver<&'static str>) -> bool {
    let version = "v1.0".to_string();
    let (rpctx, rpcrx): (Sender<(String, &'static str)>, Receiver<(String, &'static str)>) = mpsc::channel();
    let mut page = 1;
    let mut fun_index = 0;
    let mut window = initscr();
    let mut specialinteraction = false;
    let mut local_volume_counter = 0.5;
    let mut isloop = false;
    let mut reinit_rpc = true;
    let _rpc_thread = thread::spawn(move || {
        rpc_handler(rpcrx);
    });
    let mut songs = Songs{
        songs: glob("music/*.mp3").unwrap().filter_map(Result::ok).map(|p| p.display().to_string()).collect::<Vec<String>>(),
        current_song: 0,
        current_name: "Nothing".to_string(),
        typical_page_size: 14,
    };
    (pancurses::curs_set(0), window.keypad(true), pancurses::noecho(), window.nodelay(true));
    window.resize(20, 50);
    (
        pancurses::start_color(),
        pancurses::init_pair(1, pancurses::COLOR_GREEN, pancurses::COLOR_BLACK),
        pancurses::init_pair(2, pancurses::COLOR_RED, pancurses::COLOR_BLACK),
        pancurses::init_pair(0, pancurses::COLOR_WHITE, pancurses::COLOR_BLACK),
        pancurses::init_pair(3, pancurses::COLOR_BLACK, pancurses::COLOR_WHITE),
        window.attron(pancurses::A_BOLD),
        window.attron(pancurses::A_NORMAL),
    );
    // enable green color
    let (maxy, maxx) = window.get_max_yx();
    loop {
        // => => => REDRAW HERE <= <= <= 
        window.erase();
        window.attrset(pancurses::A_NORMAL); // Reset to normal attributes
        window.border('│', '│', '─', '─', '┌', '┐', '└', '┘');
        let page_indicator = format!("Page {}/{}", page, (songs.songs.len() as f32 / songs.typical_page_size as f32).ceil() as usize);
        window.mvaddstr(0, maxx - (page_indicator.len() as i32 + 2), page_indicator.as_str());
        window.mvchgat(0, maxx - (page_indicator.len() as i32 + 2), page_indicator.len() as i32, pancurses::A_BOLD, 0);
        {
            let start_index = (page-1) * songs.typical_page_size;
            let end_index = std::cmp::min(start_index + songs.typical_page_size, songs.songs.len());
            for (i, song) in songs.songs[start_index..end_index].iter().enumerate() {
                let display_name = song.replace("music/", "").replace("music\\", "").replace(".mp3", "");
                window.mvaddstr(i as i32 + 1, 2, display_name.as_str());
                window.mvchgat(i as i32 + 1, 2, display_name.len() as i32, pancurses::A_BOLD, 0);
                if i == fun_index {
                    // highlight with color pair 3
                    window.mvchgat(i as i32 + 1, 2, display_name.len() as i32, pancurses::A_BOLD | pancurses::COLOR_PAIR(3), 3);
                }
                if song == &songs.current_name {
                    // highlight with a green * at the end
                    window.mvaddstr(i as i32 + 1, format!("{} *", display_name).len() as i32, " *");
                    window.mvchgat(i as i32 + 1, format!("{} *", display_name).len() as i32, 2, pancurses::A_BOLD, 1);

                }
            }  
        }
        window.mvaddstr(maxy-5, 0, "├".to_owned() + "─".repeat((maxx-2) as usize).as_str() + "┤");
        window.mvaddstr(maxy-4, 2, format!("{}", songs.current_name().replace("music/", "").replace("music\\", "").replace(".mp3", "")).as_str());
        window.mvchgat(maxy-4, 2, maxx-4, pancurses::A_NORMAL, 1);
        window.mvaddstr(maxy-3, 2, "Version  Loop       Crystal      Rpc      Vol ");
        window.mvaddstr(maxy-2, 2, format!("{}", version));
        window.mvchgat(maxy-2, 2, format!("{}", version).len() as i32, pancurses::A_BOLD, 0);
        window.mvaddstr(maxy-2, 11, format!("{} ", match isloop { true => "true", false => "false" }));
        window.mvchgat(maxy-2, 11, format!("{} ", match isloop { true => "true", false => "false" }).len() as i32, pancurses::A_BOLD, match isloop { true => 1, false => 2 });
        window.mvaddstr(maxy-2, maxx/2-4, " offline");
        window.mvchgat(maxy-2, maxx/2-4, " offline".len() as i32, pancurses::A_BOLD, 2);
        window.mvaddstr(
            maxy-2,
            maxx - ((format!("{} ", local_volume_counter)).len() as i32 + 2),
            format!("{} ", local_volume_counter)
        );
        window.mvchgat(maxy-2, maxx - ((format!("{} ", local_volume_counter)).len() as i32 + 2), (format!("{}  ", local_volume_counter)).len() as i32, pancurses::A_BOLD, 0);
        if reinit_rpc {
            window.mvaddstr(maxy-2, maxx - 15, "init");
            window.mvchgat(maxy-2, maxx - 15, "init".len() as i32, pancurses::A_BOLD, 2);
        } else {
            window.mvaddstr(maxy-2, maxx - 15, "done");
            window.mvchgat(maxy-2, maxx - 15, "done".len() as i32, pancurses::A_BOLD, 1);
        }
        window.refresh();

        // DRAW SONGS
        let key_opt = match comm_rx.try_recv() {
            Ok(_key) => Some(Input::KeyF13),
            Err(_) => window.getch(),
        };
        if let Some(key) = key_opt {
            // write to logfile
            match key {
                Input::KeyF13 => { // song ended
                    if !isloop {
                        songs.set_by_next();
                    }
                    tx.send(("play_track", songs.current_name())).unwrap();
                    reinit_rpc = true;
                },
                Input::Character('q') => break,
                Input::KeyDown | Input::Character('j') => {
                    if specialinteraction {
                        tx.send(("volume_down", String::new())).unwrap();
                        if local_volume_counter > 0.0 {
                            match local_volume_counter {  
                                1.0 => local_volume_counter = 0.9, 
                                0.9 => local_volume_counter = 0.8,
                                0.8 => local_volume_counter = 0.7,
                                0.7 => local_volume_counter = 0.6,
                                0.6 => local_volume_counter = 0.5,
                                0.5 => local_volume_counter = 0.4,
                                0.4 => local_volume_counter = 0.3,
                                0.3 => local_volume_counter = 0.2,
                                0.2 => local_volume_counter = 0.1,
                                0.1 => local_volume_counter = 0.0,
                                0.0 => (),
                                _ => (),
                            }
                        }
                    } else {
                        if fun_index+1 < songs.typical_page_size && (fun_index + ((page-1) * songs.typical_page_size)) < songs.songs.len()-1 {
                            fun_index += 1;
                        } else if (fun_index + ((page-1) * songs.typical_page_size)) < songs.songs.len()-1 {
                            page += 1;
                            fun_index = 0;
                        }
                    }
                },
                Input::KeyUp | Input::Character('u') => {
                    if specialinteraction {
                        tx.send(("volume_up", String::new())).unwrap();
                        if local_volume_counter < 1.0 {
                            match local_volume_counter {  
                                0.0 => local_volume_counter = 0.1, 
                                0.1 => local_volume_counter = 0.2,
                                0.2 => local_volume_counter = 0.3,
                                0.3 => local_volume_counter = 0.4,
                                0.4 => local_volume_counter = 0.5,
                                0.5 => local_volume_counter = 0.6,
                                0.6 => local_volume_counter = 0.7,
                                0.7 => local_volume_counter = 0.8,
                                0.8 => local_volume_counter = 0.9,
                                0.9 => local_volume_counter = 1.0,
                                1.0 => (),
                                _ => (),
                            }
                        }
                    } else {
                        if fun_index > 0 {
                            fun_index -= 1;
                        } else if page > 1 {
                            page -= 1;
                            fun_index = songs.typical_page_size -1;
                        }
                    }
                },
                Input::Character('p') => {
                    songs.set_by_pindex(fun_index, page);
                    tx.send(("play_track", songs.current_name())).unwrap();
                    reinit_rpc = true;
                },
                Input::Character('o') => {
                    if specialinteraction {
                        specialinteraction = false;
                    } else {
                        specialinteraction = true;
                    }
                },
                Input::Character('l') => {
                    isloop = !isloop;
                },
                Input::Character('s') => {
                    songs.stop();
                    tx.send(("pause", String::new())).unwrap();
                },
                Input::Character('b') => { todo!() },
                Input::Character('j') => { todo!() }, // SEARCH MODE TODO


                _ => (),
            }
            
        } else {
            thread::sleep(Duration::from_millis(100));
        }
        if reinit_rpc {
            rpctx.send((songs.current_name().to_string(), "v1.0")).unwrap();
            reinit_rpc = false;
        }

    }
    rpctx.send(("stop".to_string(), "stop")).unwrap();
    true
}

fn main() {
    let (tx, rx): (Sender<(&'static str, String)>, Receiver<(&'static str, String)>) = mpsc::channel();
    let (tx_proc, rx_proc): (Sender<Instant>, Receiver<Instant>) = mpsc::channel();
    let (comm_tx, comm_rx): (Sender<&'static str>, Receiver<&'static str>) = mpsc::channel();
    let (sigkill, issigkill): (Sender<bool>, Receiver<bool>) = mpsc::channel();
    thread::spawn(move || {
        match play_audio(rx, tx_proc) {
            Ok(_) => {
                ()
            },
            Err(e) => {
                eprintln!("Error in audio playback: {}", e);
            }
        }
    });
    tx.send(("volume_df", String::new())).unwrap();
    //tx.send(("play_track", "Psychogram.mp3".to_string())).unwrap(); // to test comm
    let mut found_val = (false, Instant::now());
    let ret_value: Result<Instant, TryRecvError> = Err(TryRecvError::Empty);
    let thrloop: thread::JoinHandle<()> = thread::spawn(move || loop {
        // i need a kill thing for this thread, because it doesn't have a natural break
        // because it is supposed to live as long as the program runs
        match issigkill.try_recv() {
            Ok(_) => {
                println!("Killing loop thread");
                break;
            },
            Err(_) => (),
        }
        match rx_proc.try_recv() {
            Ok(val) => {
                println!("{:?}", val);
                found_val = (true, val);
                if val <= Instant::now() {
                    found_val = (false, Instant::now());
                }
            },
            Err(_) => (),
        }
        if found_val.0 != true {
            match ret_value {
                Ok(val) => {
                    println!("{:?}", val);
                    found_val = (true, val);
                },
                Err(_) => (),
            }
        } else {
            if Instant::now() >= found_val.1 {
                // implement communication between management fn and this to let it know the song ended
                // thats the entire purpose of this thread
                comm_tx.send("turn").unwrap();
                found_val = (false, Instant::now());
            }
        }
        thread::sleep(Duration::from_millis(100));
    });

    if crystal_manager(tx, comm_rx) {
        sigkill.send(true).unwrap();
    }
    thrloop.join().unwrap();

}

fn play_audio(receiver: Receiver<(&'static str, String)>, transmitter: Sender<Instant>) -> Result<String, Box<dyn std::error::Error>> {
    let stream_handle: OutputStream = rodio::OutputStreamBuilder::open_default_stream()
        .expect("open default audio stream");
    let sink = rodio::Sink::connect_new(&stream_handle.mixer());
    loop {
        if let Ok((command, value)) = receiver.recv_timeout(Duration::from_millis(100)) {
            match command {
                "pause" => {
                    sink.pause();
                    transmitter.send(Instant::now())?; // send a time in the past to indicate paused state
                },
                "resume" => sink.play(),
                "stop" => {
                    sink.stop();
                    break;
                },
                "volume_df" => {
                    sink.set_volume(0.5);
                    println!("Volume: {}", sink.volume());
                },
                "volume_up" => {
                    match sink.volume() {  // these manual updates look dumb but volume +-0.1 doesn't work as intended (gives weird float values)
                        0.0 => sink.set_volume(0.1), // + ruining the volume control entirely, i speak from experience.
                        0.1 => sink.set_volume(0.2),
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
                    println!("Volume: {}", sink.volume());
                    
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
                    println!("Volume: {}", sink.volume());
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
                },
                _ => return Err("Unknown command".into()),
            }
        }
    }
    Ok("Stopped".to_string())
}