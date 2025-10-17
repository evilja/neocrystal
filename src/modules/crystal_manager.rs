extern crate pancurses;
extern crate glob;
use std::thread;
use std::time::{Duration};
use std::sync::mpsc::{self, Receiver, Sender};
use pancurses::{initscr, Input};
use glob::glob;

use super::{songs::{Songs, absolute_index}, presence::rpc_handler, curses::*, utils::Volume};
const UP:           char= 'u';
const DOWN:         char= 'j';
const LEFT:         char= 'n';
const RIGHT:        char= 'm';
const SHUFFLE:      char= 'f';
const PLAY:         char= 'p';
const BLACKLIST:    char= 'b';
const STOP:         char= 's';
const RESUME:       char= 'r';
const LOOP:         char= 'l';
const SPECIAL:      char= 'o';
const QUIT:         char= 'q';
const SEARCH:       char= 'h';
const TOP:          char= 'g';
const CHANGE:       char= 'c';
const SETNEXT:      char= 'e';

pub fn crystal_manager(tx: Sender<(&'static str, String)>, comm_rx: Receiver<(&'static str, Duration)>) -> bool {
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
    let mut setnext                 = usize::MAX;
    let mut is_search               = (0, String::from("false"));
    let mut suspend_redraw          = false;
    let mut songs                   = Songs::constructor(glob("music/*.mp3").unwrap().filter_map(Result::ok).map(|p| p.display().to_string()).collect::<Vec<String>>());
    let _rpc_thread                 = thread::spawn(move || {
                                        rpc_handler(rpcrx);
                                      });

    init_curses(&mut window);
    let (maxy, maxx)                = window.get_max_yx();
    loop {
        redraw(&mut window, maxx, maxy, &mut songs, page,
                local_volume_counter.steps, is_search.1.clone(),
                isloop, reinit_rpc, maxlen, fcalc, fun_index,
                setnext
            );


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
            if is_search.0 != 0 {
                match key {
                    
                    Input::KeyEnter | Input::Character('\n') => {
                        match is_search.0 {
                            1 => {
                                songs.search(is_search.1.clone());
                                fun_index = 0;
                                page = 1;
                            },
                            2 => {songs.set_artist(songs.match_c(), is_search.1.clone());},
                            _ => {}
                        }
                        is_search = (0, String::from("false"));
                        continue;
                    },
                    Input::KeyBackspace | Input::Character('\x7f') | Input::Character('\x08') => {
                        is_search.1.pop();
                        continue;
                    },
                    Input::Character(i) => {
                        is_search.1.push(i);
                        continue;
                    },
                    _ => {}
                }
            }
            match key {
                Input::KeyF13 => { // song ended
                    println!("Song ended rpc received");
                    let mut sp: String = "N/A".to_string();
                    if setnext != usize::MAX {
                        songs.set_force(setnext);
                        sp = songs.original_song_path(setnext);
                        setnext = usize::MAX;
                    } else if !isloop {
                        songs.set_by_next().unwrap();
                    }
                    match sp.as_str() {
                        "N/A" => sp = songs.current_song_path(),
                        _ => (),
                    }

                    tx.send(("play_track", sp)).unwrap();
                    reinit_rpc = true;
                    maxlen = songs.all_songs.get(songs.current_index).map(|s| s.duration).unwrap_or(Duration::from_secs(0));
                    let _ = rpctx.send((songs.current_song_path().to_string(), maxlen.as_secs_f32() as u64));
                    continue;
                },
                Input::KeyF14 => { //duration sent
                    if reinit_rpc {
                        reinit_rpc = false;
                    }
                    continue;
                }

                Input::Character(QUIT) => break,

                Input::KeyDown | Input::Character(DOWN) => {
                    if specialinteraction {
                        if local_volume_counter.as_f64() > 0.0 {
                            local_volume_counter.step_down();
                        }
                        tx.send(("set_volume", local_volume_counter.as_f32().to_string())).unwrap();
                    } else {
                        if fun_index+1 < songs.typical_page_size && absolute_index(fun_index, page, songs.typical_page_size) < songs.filtered_songs.len()-1 { // protection for page size
                            fun_index += 1;
                        } else if absolute_index(fun_index, page, songs.typical_page_size) < songs.filtered_songs.len()-1 {
                            page += 1;
                            fun_index = 0;
                        }
                    }
                    continue;
                },

                Input::KeyUp | Input::Character(UP) => {
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
                    continue;
                },

                Input::Character(PLAY) => {
                    if songs.set_by_pindex(fun_index, page) != Err(0) {
                        tx.send(("play_track", songs.current_song_path())).unwrap();
                        reinit_rpc = true;
                        maxlen = songs.all_songs.get(songs.current_index).map(|s| s.duration).unwrap_or(Duration::from_secs(0));
                        for _i in 0..=1 {
                            match rpctx.send((songs.current_song_path().to_string(), maxlen.as_secs_f32() as u64)) {
                                Ok(()) => break,
                                Err(_) => thread::sleep(Duration::from_millis(100)),
                            }
                            fcalc = Duration::from_secs(0);
                        }
                    }
                    continue;
                },

                Input::Character(SPECIAL) => {
                    if specialinteraction {
                        specialinteraction = false;
                    } else {
                        specialinteraction = true;
                    }
                    continue;
                },

                Input::Character(LOOP) => {
                    isloop = !isloop;
                    continue;
                },

                Input::Character(STOP) => {
                    songs.stop();
                    tx.send(("pause", String::new())).unwrap();
                    continue;
                },

                Input::Character(BLACKLIST) => {
                    songs.blacklist(absolute_index(fun_index, page, songs.typical_page_size));
                    continue;
                },

                Input::Character(RESUME) => {
                    songs.stophandler = false;
                    tx.send(("resume", String::new())).unwrap();
                    continue;
                },
                Input::KeyRight | Input::Character(RIGHT) => {
                    tx.send(("forward", String::new())).unwrap();
                    suspend_redraw = true;
                    continue;

                }
                Input::KeyLeft | Input::Character(LEFT) => {
                    tx.send(("back", String::new())).unwrap();
                    suspend_redraw = true;
                    continue;
                    
                },
                Input::Character(SHUFFLE) => { songs.shuffle(); },
                Input::Character(SEARCH) => {
                    is_search.1.clear();
                    is_search.0 = 1;
                    continue;
                }, // SEARCH MODE TODO
                Input::Character(TOP) => { page = 1; fun_index = 0; continue;},
                Input::Character(CHANGE) => {
                    is_search.1.clear();
                    is_search.0 = 2;
                    continue;

                },
                Input::Character(SETNEXT) => {
                    setnext = songs.get_original_index(absolute_index(fun_index, page, songs.typical_page_size));
                    continue;
                },

                _ => (),
            }
            
        }
        

    }
    match rpctx.send(("stop".to_string(), 0)) { _ => () }
    true
}