
extern crate pancurses;
extern crate glob;
use std::thread;
use std::time::{Duration, Instant};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::path::Path;
use mp3_duration;
use pancurses::{initscr, Input};
use glob::glob;
mod modules;
use crate::modules::songs::Songs;
use crate::modules::presence::rpc_handler;
use crate::modules::curses::*;
use crate::modules::audio::play_audio;

fn crystal_manager(tx: Sender<(&'static str, String)>, comm_rx: Receiver<(&'static str, Duration)>) -> bool {
    let (rpctx, rpcrx): (Sender<(String, &'static str)>, Receiver<(String, &'static str)>) 
                                    = mpsc::channel();
    let version                     = "v1.1r".to_string();
    let mut page                    = 1;
    let mut fcalc: Duration         = Duration::from_secs(0);
    let mut fun_index               = 0;
    let mut window                  = initscr();
    let mut specialinteraction      = false;
    let mut local_volume_counter    = 0.5;
    let mut isloop                  = false;
    let mut maxlen: Duration        = Duration::from_secs(0);
    let mut reinit_rpc              = true;
    let mut songs                   = Songs::constructor(glob("music/*.mp3").unwrap().filter_map(Result::ok).map(|p| p.display().to_string()).collect::<Vec<String>>());
    let _rpc_thread                 = thread::spawn(move || {
                                        rpc_handler(rpcrx);
                                      });
    init_curses(&mut window);
    let (maxy, maxx)                = window.get_max_yx();
    loop {
        redraw(&mut window, maxx, maxy, &mut songs, page, local_volume_counter, version.clone(), isloop, reinit_rpc, maxlen, fcalc, fun_index);

        let key_opt = match comm_rx.try_recv() {
            Ok(_key) => match _key.0 {
                "turn" => Some(Input::KeyF13),
                "duration" => {
                    fcalc = _key.1;
                    Some(Input::KeyF14)
                },
                _ => Some(Input::KeyF14),
            },
            Err(_) => window.getch(),
        };

        if let Some(key) = key_opt {
            match key {
                Input::KeyF13 => { // song ended
                    if !isloop {
                        songs.set_by_next();
                    }
                    tx.send(("play_track", songs.current_name())).unwrap();
                    reinit_rpc = true;
                    maxlen = mp3_duration::from_path(Path::new(songs.current_name.as_str())).unwrap();
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
                        if fun_index+1 < songs.typical_page_size && (fun_index + ((page-1) * songs.typical_page_size)) < songs.songs.len()-1 { // protection for page size
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
                    if songs.set_by_pindex(fun_index, page, false) != 9879871 { // magic number 9879871 means not found
                        tx.send(("play_track", songs.current_name())).unwrap();
                        reinit_rpc = true;
                        maxlen = mp3_duration::from_path(Path::new(songs.current_name.as_str())).unwrap();
                        fcalc = Duration::from_secs(0);
                    }
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

                Input::Character('b') => { 
                    songs.blacklist(fun_index + ((page-1) * songs.typical_page_size));
                },

                Input::Character('r') => {
                    songs.stophandler = false;
                    tx.send(("resume", String::new())).unwrap();
                },

                Input::Character('h') => { todo!() }, // SEARCH MODE TODO


                _ => (),
            }
            
        } else {
            thread::sleep(Duration::from_millis(50));
        }

        if reinit_rpc {
            rpctx.send((songs.current_name().to_string(), "v1.1r")).unwrap();
            reinit_rpc = false;
        }

    }
    rpctx.send(("stop".to_string(), "stop")).unwrap();
    true
}

fn main() { // establish communications and threads, then give the job to crystal_manager fn
    let (tx, rx): (Sender<(&'static str, String)>, Receiver<(&'static str, String)>)                = mpsc::channel();
    let (tx_proc, rx_proc): (Sender<Instant>, Receiver<Instant>)                                    = mpsc::channel();
    let (comm_tx, comm_rx): (Sender<(&'static str, Duration)>, Receiver<(&'static str, Duration)>)  = mpsc::channel();
    let (sigkill, issigkill): (Sender<bool>, Receiver<bool>)                                        = mpsc::channel();
    let mut found_val                                                                               = (false, Instant::now());
    let ret_value: Result<Instant, TryRecvError>                                                    = Err(TryRecvError::Empty);

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
    let thrloop: thread::JoinHandle<()> = thread::spawn(move || loop {
        match issigkill.try_recv() {
            Ok(_) => {
                println!("Killing loop thread");
                break;
            },
            Err(_) => (),
        }
        match rx_proc.try_recv() {
            Ok(val) => {
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
                    found_val = (true, val);
                },
                Err(_) => (),
            }
        } else {
            if Instant::now() >= found_val.1 {
                comm_tx.send(("turn", Instant::now() - Instant::now())).unwrap();
                found_val = (false, Instant::now());
            }
            comm_tx.send(("duration", found_val.1 - Instant::now())).unwrap();
        }
        
        thread::sleep(Duration::from_millis(100));
    });

    if crystal_manager(tx, comm_rx) {
        sigkill.send(true).unwrap();
    }
    thrloop.join().unwrap();

}

