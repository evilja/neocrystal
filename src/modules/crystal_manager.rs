extern crate pancurses;
extern crate glob;
use std::thread::{self};
use std::time::{Duration, Instant};
use std::sync::mpsc::{self, Receiver, Sender};
use pancurses::{initscr, Input};
use glob::glob;

use super::{songs::{Songs, absolute_index}, 
            presence::rpc_handler, 
            curses::*, 
            utils::{Volume, SearchQuery, State, RpcState, Indexer, Timer, ReinitMode}, 
            utils::SlidingText};

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
const DESEL:        char= 'd';
const SETPLAYLIST:  char= 'v';


pub struct PlaybackError {
    pub pre: Instant,
    pub rep: u16,
    pub mrp: u16,
}
impl PlaybackError {
    pub fn constructor(mrp: u16) -> Self {
        Self {
            pre: Instant::now(),
            rep: 0,
            mrp,
        }
    }
    pub fn test(&mut self) -> bool {
        if self.pre > Instant::now() - Duration::from_secs(10) {
            if self.rep == self.mrp { self.rep = 0; }
            self.rep += 1;
        }
        if self.rep == self.mrp {
            return true
        }
        return false
        
    }

}

pub fn crystal_manager(tx: Sender<(&'static str, String)>, comm_rx: Receiver<(&'static str, Duration)>) -> bool {
    let (rpctx, rpcrx): (Sender<(String, u64)>, Receiver<(String, u64)>) = mpsc::channel();
    let mut locind                  = Indexer { page: 1, index: 0 };
    let mut loctimer                = Timer { fcalc: Duration::from_secs(0), maxlen: Duration::from_secs(0) };
    let mut window                  = initscr();
    let mut state                   = State { spint: false, isloop: false, desel: false };
    let mut local_volume_counter    = Volume {steps: 50, step_div: 2};
    let mut ui                      = UI::new();
    let mut action                  = Action::Nothing;
    let mut kill_timer              = PlaybackError::constructor(6);
    let mut rpc_state               = RpcState {reinit: false, timer: Instant::now(), mode: ReinitMode::None};
    let mut local_sliding           = SlidingText::new("Nothing", 27, Duration::from_millis(300));
    let mut is_search               = SearchQuery { mode: 0, query: String::from("false") };
    let mut songs                   = Songs::constructor(glob("music/*.mp3").unwrap().filter_map(Result::ok).map(|p| p.display().to_string()).collect::<Vec<String>>());
    let _rpc_thread                 = thread::spawn(move || {
                                        rpc_handler(rpcrx);
                                      });

    init_curses(&mut window);
    ui.draw_const(&mut window);
    loop {
        redraw(&mut ui, &mut window, &songs, locind.page,
                local_volume_counter.steps, &is_search.query,
                state.isloop, rpc_state.reinit, loctimer.maxlen, loctimer.fcalc, 
                locind.index, state.desel, local_sliding.visible_text()
            );
        let key_opt = window.getch().or_else(|| {
            match comm_rx.recv_timeout(Duration::from_millis(10)) {
                Ok(_key) => match _key.0 {
                    "turn" => Some(Input::KeyF13),
                    "duration" => {
                        loctimer.fcalc = _key.1;
                        Some(Input::KeyF14)
                    },
                    _ => Some(Input::KeyF15),
                },
                Err(_) => None,
            }
        });
        if let Some(mut key) = key_opt {
            if is_search.mode != 0 {
                match key {
                    
                    Input::KeyEnter | Input::Character('\n') => {
                        match is_search.mode {
                            1 => {
                                songs.search(&is_search.query);
                                locind.index = 0;
                                locind.page = 1;
                            },
                            2 => {songs.set_artist(songs.match_c(), &is_search.query);},
                            3 => {songs.set_playlist(songs.match_c(), &is_search.query);}
                            _ => {}
                        }
                        is_search.default();
                        continue;
                    },
                    Input::KeyBackspace | Input::Character('\x7f') | Input::Character('\x08') => {
                        is_search.query.pop();
                        continue;
                    },
                    Input::Character(i) => {
                        is_search.query.push(i);
                        continue;
                    },
                    _ => {}
                }
            }
            if key == Input::KeyMouse {
                if let Ok(mevent) = pancurses::getmouse() {
                    if (mevent.bstate & 0x2) != 0 {
                        action = ui.click(mevent.x, mevent.y);
                    }
                    match action {
                        Action::Play(p, f) => { locind.page = p; locind.index = f; key = Input::Character(PLAY)},
                        Action::Shuffle => { key = Input::Character(SHUFFLE); },
                        Action::Repeat => { key = Input::Character(LOOP); },
                        Action::Rpc => { rpc_state.setup(ReinitMode::Renew); },
                        Action::PgDown => {
                            let absolute = absolute_index(0, locind.page+1, songs.typical_page_size) < songs.filtered_songs.len() - 1;
                            if absolute {
                                locind.index = 0;
                                locind.page += 1;
                            }

                        },
                        Action::PgUp => {
                            if locind.page > 1 {
                                locind.page -= 1;
                                locind.index = songs.typical_page_size - 1;
                            } else {
                                locind.index = 0;
                            }
                        }
                        Action::Nothing => (),
                    }
                }
            }
            match key {
                Input::KeyF13 => { // song ended
                    if songs.stophandler {
                        continue;
                    } else if !state.isloop {
                        match songs.set_by_next() {
                            Ok(_) => (),
                            Err(_) => {
                                continue;
                            }
                        }
                    }
                    match kill_timer.test() {
                        true => {
                              play_current_song(&locind, &mut songs, &tx, &mut loctimer, &mut local_sliding);
                        },
                        false => (),
                    }

                    tx.send(("play_track", songs.current_song_path())).unwrap();
                    loctimer.maxlen = songs.get_duration();
                    rpc_state.setup(ReinitMode::Init);
                    local_sliding.reset_to(songs.current_name());
                    continue;
                },
                Input::KeyF14 => { //duration sent
                    if rpc_state.timer <= Instant::now() && rpc_state.reinit {
                        match rpc_state.mode {
                            ReinitMode::None => continue,
                            ReinitMode::Renew => { let _ = rpctx.send(("%renew".to_string(), loctimer.maxlen.checked_sub(loctimer.fcalc).unwrap_or_default().as_secs())); },
                            ReinitMode::Init => { let _ = rpctx.send((songs.current_song_path().to_string(), loctimer.maxlen.as_secs_f32() as u64)); },
                        }
                        rpc_state.reset();
                    }
                    continue;
                },
                Input::Character(QUIT) => break,

                Input::KeyDown | Input::Character(DOWN) => {
                    move_selection(Direction::Down, &mut locind, &state, &songs, &mut local_volume_counter, &tx);
                    continue;
                },

                Input::KeyUp | Input::Character(UP) => {
                    move_selection(Direction::Up, &mut locind, &state, &songs, &mut local_volume_counter, &tx);
                    continue;
                },

                Input::Character(PLAY) => {
                    play_current_song(&locind, &mut songs, &tx, &mut loctimer, &mut local_sliding);
                    rpc_state.setup(ReinitMode::Init);
                    continue;
                },

                Input::Character(SPECIAL) => {
                    state.spint = !state.spint;
                    continue;
                },

                Input::Character(LOOP) => {
                    state.isloop = !state.isloop;
                    continue;
                },

                Input::Character(STOP) => {
                    songs.stop();
                    tx.send(("pause", String::new())).unwrap();
                    rpctx.send(("%clear".to_string(), 0)).unwrap();
                    continue;
                },

                Input::Character(BLACKLIST) => {
                    songs.blacklist(absolute_index(locind.index, locind.page, songs.typical_page_size));
                    continue;
                },

                Input::Character(RESUME) => {
                    if songs.current_index == usize::MAX {
                        continue;
                    }
                    songs.stophandler = false;
                    tx.send(("resume", String::new())).unwrap();
                    continue;
                },
                Input::KeyRight | Input::Character(RIGHT) => {
                    tx.send(("forward", String::new())).unwrap();
                    rpc_state.setup(ReinitMode::Renew);
                    continue;

                },
                Input::KeyLeft | Input::Character(LEFT) => {
                    tx.send(("back", String::new())).unwrap();
                    rpc_state.setup(ReinitMode::Renew);
                    continue; 
                },
                Input::Character(SHUFFLE) => { songs.shuffle(); },
                Input::Character(SEARCH) => {
                    is_search.to_mode(1);
                    continue;
                },
                Input::Character(TOP) => { locind.page = 1; locind.index = 0; continue;},
                Input::Character(CHANGE) => {
                    is_search.to_mode(2);
                    continue;
                },
                Input::Character(SETPLAYLIST) => {
                    is_search.to_mode(3);
                    continue;
                },
                Input::Character(SETNEXT) => {
                    songs.set_next(absolute_index(locind.index, locind.page, songs.typical_page_size));
                    continue;
                },
                Input::Character(DESEL) => {
                    state.desel = !state.desel;
                    continue;
                },

                _ => (),
            }
            
        }
        

    }
    match rpctx.send(("%stop".to_string(), 0)) { _ => () }
    true
}

pub fn play_current_song(
    locind: &Indexer,
    songs: &mut Songs,
    tx: &Sender<(&'static str, String)>,
    loctimer: &mut Timer,
    local_sliding: &mut SlidingText,
) {
    if songs.set_by_pindex(locind.index, locind.page) != Err(0) {
        if tx.send(("play_track", songs.current_song_path())).is_err() {
            return;
        }

        loctimer.maxlen = songs.get_duration();

        loctimer.fcalc = Duration::from_secs(0);

        local_sliding.reset_to(songs.current_name());
    }
}

pub enum Direction {
    Up,
    Down,
}

pub fn move_selection(
    direction: Direction,
    locind: &mut Indexer,
    state: &State,
    songs: &Songs,
    local_volume_counter: &mut Volume,
    tx: &Sender<(&'static str, String)>
) {
    if state.spint {
        match direction {
            Direction::Up => local_volume_counter.step_up(),
            Direction::Down => local_volume_counter.step_down(),
        }
        tx.send(("set_volume", local_volume_counter.as_f32().to_string()))
            .unwrap_or_else(|_| ());
    } else {
        match direction {
            Direction::Up => {
                if locind.index > 0 {
                    locind.index -= 1;
                } else if locind.page > 1 {
                    locind.page -= 1;
                    locind.index = songs.typical_page_size - 1;
                }
            }
            Direction::Down => {
                let absolute = absolute_index(locind.index, locind.page, songs.typical_page_size) < songs.filtered_songs.len() - 1;
                if locind.index + 1 < songs.typical_page_size
                    && absolute
                {
                    locind.index += 1;
                } else if absolute {
                    locind.page += 1;
                    locind.index = 0;
                }
            }
        }
    }
}

