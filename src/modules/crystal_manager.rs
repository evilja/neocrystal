extern crate glob;
extern crate pancurses;
use crate::modules::audio::{AudioCommand, AudioReportAction};
use crate::modules::presence::rpc_init_autobuild;
use glob::glob;
use home::home_dir;
use pancurses::{initscr, Input};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self};
use std::time::{Duration, Instant};

use super::{
    curses::*,
    presence::{rpc_handler, RpcCommand},
    songs::{absolute_index, Songs},
    utils::SlidingText,
    utils::{Indexer, ReinitMode, RpcState, SearchQuery, State, Timer, Volume},
};

const UP: char = 'u';
const DOWN: char = 'j';
const LEFT: char = 'n';
const RIGHT: char = 'm';
const SHUFFLE: char = 'f';
const PLAY: char = 'p';
const BLACKLIST: char = 'b';
const STOP: char = 's';
const RESUME: char = 'r';
const LOOP: char = 'l';
const SPECIAL: char = 'o';
const QUIT: char = 'q';
const SEARCH: char = 'h';
const TOP: char = 'g';
const CHANGE: char = 'c';
const SETNEXT: char = 'e';
const DESEL: char = 'd';
const SETPLAYLIST: char = 'v';

/// macro: get_input_or_report
/// try to get input from pancurses:
/// success -> return the key
/// fail -> try to get information from comm_rx (audio thread)
///      -> success -> set local timer fcalc to the value &&
///                 check if fcalc is smaller than 100 milliseconds && info belongs to the current song
///                 true -> return f13 meaning song ended. 100 milliseconds because get_pos is inconsistent
///                 false -> return f14 meaning a duration is sent, triggering rpc things and such.
///      -> fail -> do nothing
macro_rules! get_input_or_report {
    ($window:expr, $comm_rx:expr, $songs:expr, $loctimer:expr, $timeout_ms:expr) => {{
        $window.getch().or_else(|| {
            match $comm_rx.recv_timeout(std::time::Duration::from_millis($timeout_ms)) {
                Ok(msg) => match msg {
                    crate::AudioReportAction::Duration(name, time) => {
                        if name == $songs.current_song_path() {
                            $loctimer.fcalc = time;
                            if $loctimer.fcalc <= std::time::Duration::from_millis(100) {
                                Some(Input::KeyF13)
                            } else {
                                Some(Input::KeyF14)
                            }
                        } else {
                            Some(Input::KeyF15)
                        }
                    }
                    _ => Some(Input::KeyF15),
                },
                Err(_) => None,
            }
        })
    }};
}

pub fn crystal_manager(tx: Sender<AudioCommand>, comm_rx: Receiver<AudioReportAction>) -> bool {
    let (rpctx, rpcrx): (Sender<RpcCommand>, Receiver<RpcCommand>) = mpsc::channel();
    let mut locind = Indexer { page: 1, index: 0 };
    let mut loctimer = Timer::new();
    let mut window = initscr();
    let mut state = State {
        spint: false,
        isloop: false,
        desel: false,
    };
    let mut local_volume_counter = Volume {
        steps: 50,
        step_div: 2,
    };
    let mut ui = UI::new();
    let mut action = Action::Nothing;
    let mut rpc_state = RpcState {
        reinit: false,
        timer: Instant::now(),
        mode: ReinitMode::None,
    };
    let mut local_sliding = SlidingText::new("Nothing", 23, Duration::from_millis(300));
    let mut is_search = SearchQuery {
        mode: 0,
        query: String::from("false"),
    };
    let homedir = home_dir()
        .expect("No home directory found")
        .join("Music")
        .join("*.mp3")
        .to_string_lossy()
        .to_string();
    let mut songs = Songs::constructor(
        glob(&homedir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|p| p.display().to_string())
            .collect::<Vec<String>>(),
    );
    let _rpc_thread = thread::spawn(move || {
        rpc_handler(rpcrx);
    });

    init_curses(&mut window);
    ui.draw_const(&mut window);
    loop {
        redraw(
            &mut ui,
            &mut window,
            &songs,
            locind.page,
            local_volume_counter.steps,
            &is_search.query,
            state.isloop,
            rpc_state.reinit,
            loctimer.maxlen,
            loctimer.fcalc,
            locind.index,
            state.desel,
            local_sliding.visible_text(),
        );

        // key_opt catches either duration communications from audio thread or user input
        // if nothing is there to catch, it will just skip after 10 milliseconds           there

        let key_opt = get_input_or_report!(window, comm_rx, songs, loctimer, 10);

        if let Some(mut key) = key_opt {
            if is_search.mode != 0 {
                match key {
                    Input::KeyEnter | Input::Character('\n') => {
                        match is_search.mode {
                            1 => {
                                songs.search(&is_search.query);
                                locind.index = 0;
                                locind.page = 1;
                            }
                            2 => {
                                songs.set_artist(songs.match_c(), &is_search.query);
                            }
                            3 => {
                                songs.set_playlist(songs.match_c(), &is_search.query);
                            }
                            _ => {}
                        }
                        is_search.default();
                        continue;
                    }
                    Input::KeyBackspace | Input::Character('\x7f') | Input::Character('\x08') => {
                        is_search.query.pop();
                        continue;
                    }
                    Input::Character(i) => {
                        is_search.query.push(i);
                        continue;
                    }
                    _ => {}
                }
            }
            if key == Input::KeyMouse {
                if let Ok(mevent) = pancurses::getmouse() {
                    if (mevent.bstate & 0x2) != 0 {
                        action = ui.click(mevent.x, mevent.y);
                    }
                    match action {
                        Action::Play(p, f) => {
                            locind.page = p;
                            locind.index = f;
                            key = Input::Character(PLAY)
                        }
                        Action::Shuffle => {
                            key = Input::Character(SHUFFLE);
                        }
                        Action::Repeat => {
                            key = Input::Character(LOOP);
                        }
                        Action::Rpc => {
                            rpc_state.setup(ReinitMode::Renew);
                        }
                        Action::PgDown => {
                            let absolute =
                                absolute_index(0, locind.page + 1, songs.typical_page_size)
                                    < songs.filtered_songs.len() - 1;
                            if absolute {
                                locind.index = 0;
                                locind.page += 1;
                            }
                        }
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
                Input::KeyF13 => {
                    // song ended
                    if songs.stophandler {
                        continue;
                    } else if !state.isloop {
                        match songs.set_by_next() {
                            Ok(_) => (),
                            Err(_) => (),
                        }
                    }

                    tx.send(AudioCommand::Play(songs.current_song_path()))
                        .unwrap();
                    loctimer.maxlen = songs.get_duration();
                    loctimer.fcalc = loctimer.maxlen;
                    rpc_state.setup(ReinitMode::Init);
                    local_sliding.reset_to(songs.current_name());
                    continue;
                }
                Input::KeyF14 => {
                    //duration sent
                    if rpc_state.timer <= Instant::now() && rpc_state.reinit {
                        match rpc_state.mode {
                            ReinitMode::None => continue,
                            ReinitMode::Renew => {
                                let _ = rpctx.send(RpcCommand::Renew(
                                    loctimer
                                        .maxlen
                                        .checked_sub(loctimer.fcalc)
                                        .unwrap_or_default()
                                        .as_secs(), // elapsed time as u64
                                ));
                            }
                            ReinitMode::Init => {
                                let _ = rpctx.send(rpc_init_autobuild(
                                    &songs,
                                    loctimer.maxlen.as_secs_f32() as u64,
                                ));
                            }
                        }
                        rpc_state.reset();
                    }
                    continue;
                }
                Input::Character(QUIT) => {
                    tx.send(AudioCommand::Stop).unwrap();
                    break;
                }

                Input::KeyDown | Input::Character(DOWN) => {
                    move_selection(
                        Direction::Down,
                        &mut locind,
                        &state,
                        &songs,
                        &mut local_volume_counter,
                        &tx,
                    );
                    continue;
                }

                Input::KeyUp | Input::Character(UP) => {
                    move_selection(
                        Direction::Up,
                        &mut locind,
                        &state,
                        &songs,
                        &mut local_volume_counter,
                        &tx,
                    );
                    continue;
                }

                Input::Character(PLAY) => {
                    play_current_song(&locind, &mut songs, &tx, &mut loctimer, &mut local_sliding);
                    rpc_state.setup(ReinitMode::Init);
                    continue;
                }

                Input::Character(SPECIAL) => {
                    state.spint = !state.spint;
                    continue;
                }

                Input::Character(LOOP) => {
                    state.isloop = !state.isloop;
                    continue;
                }

                Input::Character(STOP) => {
                    songs.stop();
                    tx.send(AudioCommand::Pause).unwrap();
                    rpctx.send(RpcCommand::Clear).unwrap();
                    continue;
                }

                Input::Character(BLACKLIST) => {
                    songs.blacklist(absolute_index(
                        locind.index,
                        locind.page,
                        songs.typical_page_size,
                    ));
                    continue;
                }

                Input::Character(RESUME) => {
                    if songs.current_index == usize::MAX {
                        continue;
                    }
                    songs.stophandler = false;
                    tx.send(AudioCommand::Resume).unwrap();
                    continue;
                }
                Input::KeyRight | Input::Character(RIGHT) => {
                    tx.send(AudioCommand::SeekForward).unwrap();
                    rpc_state.setup(ReinitMode::Renew);
                    continue;
                }
                Input::KeyLeft | Input::Character(LEFT) => {
                    tx.send(AudioCommand::SeekBackward).unwrap();
                    rpc_state.setup(ReinitMode::Renew);
                    continue;
                }
                Input::Character(SHUFFLE) => {
                    songs.shuffle();
                }
                Input::Character(SEARCH) => {
                    is_search.to_mode(1);
                    continue;
                }
                Input::Character(TOP) => {
                    locind.page = 1;
                    locind.index = 0;
                    continue;
                }
                Input::Character(CHANGE) => {
                    is_search.to_mode(2);
                    continue;
                }
                Input::Character(SETPLAYLIST) => {
                    is_search.to_mode(3);
                    continue;
                }
                Input::Character(SETNEXT) => {
                    songs.set_next(absolute_index(
                        locind.index,
                        locind.page,
                        songs.typical_page_size,
                    ));
                    continue;
                }
                Input::Character(DESEL) => {
                    state.desel = !state.desel;
                    continue;
                }

                _ => (),
            }
        }
    }
    match rpctx.send(RpcCommand::Stop) {
        _ => (),
    }
    exit_curses(&mut window);
    true
}

pub fn play_current_song(
    locind: &Indexer,
    songs: &mut Songs,
    tx: &Sender<AudioCommand>,
    loctimer: &mut Timer,
    local_sliding: &mut SlidingText,
) {
    if songs.set_by_pindex(locind.index, locind.page) != Err(0) {
        if tx
            .send(AudioCommand::Play(songs.current_song_path()))
            .is_err()
        {
            return;
        }

        loctimer.maxlen = songs.get_duration();

        loctimer.fcalc = loctimer.maxlen;

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
    tx: &Sender<AudioCommand>,
) {
    if state.spint {
        match direction {
            Direction::Up => local_volume_counter.step_up(),
            Direction::Down => local_volume_counter.step_down(),
        }
        tx.send(AudioCommand::SetVolume(local_volume_counter.as_f32()))
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
                let absolute = absolute_index(locind.index, locind.page, songs.typical_page_size)
                    < songs.filtered_songs.len() - 1;
                if locind.index + 1 < songs.typical_page_size && absolute {
                    locind.index += 1;
                } else if absolute {
                    locind.page += 1;
                    locind.index = 0;
                }
            }
        }
    }
}
