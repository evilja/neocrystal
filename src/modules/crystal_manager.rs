extern crate pancurses;
extern crate glob;
use std::thread;
use std::time::{Duration};
use std::sync::mpsc::{self, Receiver, Sender};
use std::path::Path;
use mp3_duration;
use pancurses::{initscr, Input};
use glob::glob;
use super::{songs::{Songs, absolute_index}, presence::rpc_handler, curses::*, utils::Volume};

pub fn crystal_manager(tx: Sender<(&'static str, String)>, comm_rx: Receiver<(&'static str, Duration)>) -> bool {
    const T: char = 'h';
    let (rpctx, rpcrx): (Sender<(String, u64)>, Receiver<(String, u64)>) = mpsc::channel();
    let mut page                    = 1;
    let mut fcalc: Duration         = Duration::from_secs(0);
    let mut fun_index               = 0;
    let mut window                  = initscr();
    let mut specialinteraction      = false;
    let mut local_volume_counter    = Volume {steps: 50, step_div: 5};
    let mut isloop                  = false;
    let mut maxlen: Duration        = Duration::from_secs(0);
    let mut reinit_rpc              = false;
    let mut is_search               = (false, String::from("false"));
    let mut songs                   = Songs::constructor(glob("music/*.mp3").unwrap().filter_map(Result::ok).map(|p| p.display().to_string()).collect::<Vec<String>>());
    let _rpc_thread                 = thread::spawn(move || {
                                        rpc_handler(rpcrx);
                                      });

    init_curses(&mut window);
    let (maxy, maxx)                = window.get_max_yx();
    loop {
        redraw(&mut window, maxx, maxy, &mut songs, page, local_volume_counter.steps, is_search.1.clone(), isloop, reinit_rpc, maxlen, fcalc, fun_index);

        let key_opt = match comm_rx.try_recv() {
            Ok(_key) => match _key.0 {
                "turn" => Some(Input::KeyF13),
                "duration" => {
                    fcalc = _key.1;
                    Some(Input::KeyF14)
                },
                _ => Some(Input::KeyF15),
            },
            Err(_) => window.getch(),
        };

        if let Some(key) = key_opt {
            if is_search.0 != false {
                match key {
                    Input::Character(T) => {
                        songs.search(is_search.1.clone());
                        is_search = (false, String::from("false"));
                        fun_index = 0;
                        page = 1;
                        continue;
                    }
                    Input::Character(i) => {
                        is_search.1.push(i);
                        continue;
                    },
                    _ => {}
                }
            }
            match key {
                Input::KeyF13 => { // song ended
                    if !isloop {
                        songs.set_by_next().unwrap();
                    }
                    tx.send(("play_track", songs.current_name())).unwrap();
                    reinit_rpc = true;
                    maxlen = mp3_duration::from_path(Path::new(songs.current_name.as_str())).unwrap();
                    rpctx.send((songs.current_name().to_string(), maxlen.as_secs_f32() as u64)).unwrap();
                },
                Input::KeyF14 => { //duration sent
                    if reinit_rpc {
                        reinit_rpc = false;
                    }
                }

                Input::Character('q') => break,

                Input::KeyDown | Input::Character('j') => {
                    if specialinteraction {
                        if local_volume_counter.as_f64() > 0.0 {
                            local_volume_counter.step_down();
                        }
                        tx.send(("set_volume", local_volume_counter.as_f32().to_string())).unwrap();
                    } else {
                        if fun_index+1 < songs.typical_page_size && absolute_index(fun_index, page, songs.typical_page_size) < songs.songs.len()-1 { // protection for page size
                            fun_index += 1;
                        } else if absolute_index(fun_index, page, songs.typical_page_size) < songs.songs.len()-1 {
                            page += 1;
                            fun_index = 0;
                        }
                    }
                },

                Input::KeyUp | Input::Character('u') => {
                    if specialinteraction {
                        if local_volume_counter.as_f64() < 1.0 {
                            local_volume_counter.step_up();
                        }
                        tx.send(("set_volume", local_volume_counter.as_f32().to_string())).unwrap();
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
                    if songs.set_by_pindex(fun_index, page) != Err(0) {
                        tx.send(("play_track", songs.current_name())).unwrap();
                        reinit_rpc = true;
                        maxlen = mp3_duration::from_path(Path::new(songs.current_name.as_str())).unwrap();
                        rpctx.send((songs.current_name().to_string(), maxlen.as_secs_f32() as u64)).unwrap();
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
                    songs.blacklist(absolute_index(fun_index, page, songs.typical_page_size));
                },

                Input::Character('r') => {
                    songs.stophandler = false;
                    tx.send(("resume", String::new())).unwrap();
                },
                Input::KeyRight => {
                    tx.send(("forward", String::new())).unwrap();

                }
                Input::KeyLeft => {
                    tx.send(("back", String::new())).unwrap();
                    
                },
                Input::Character('f') => { songs.shuffle(); },
                Input::Character(T) => { 
                    is_search.1.clear();
                    is_search.0 = true;

                }, // SEARCH MODE TODO
                Input::Character('g') => { page = 1; fun_index = 0; },

                _ => (),
            }
            
        } else {
            thread::sleep(Duration::from_millis(50));
        }
        

    }
    rpctx.send(("stop".to_string(), 0)).unwrap();
    true
}