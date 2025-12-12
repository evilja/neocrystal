extern crate glob;
extern crate pancurses;
use crate::modules::audio::{AudioCommand, AudioReportAction};
use pancurses::{initscr, Input};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self};
use std::time::{Instant};
use super::general::GeneralState;

use super::{
    curses::*,
    presence::{rpc_handler, RpcCommand},
    songs::{absolute_index},
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
    let mut window = initscr();

    let mut general: GeneralState = GeneralState::new();

    let _rpc_thread = thread::spawn(move || {
        rpc_handler(rpcrx);
    });

    init_curses(&mut window);
    general.ui.draw_const(&mut window);
    loop {
        redraw(
            &mut general,
            &mut window,
        );

        // key_opt catches either duration communications from audio thread or user input
        // if nothing is there to catch, it will just skip after 10 milliseconds           there

        let key_opt = get_input_or_report!(window, comm_rx, general.songs, general.timer, 10);

        if let Some(mut key) = key_opt {
            if general.searchquery.mode != 0 {
                match key {
                    Input::KeyEnter | Input::Character('\n') => {
                        match general.searchquery.mode {
                            1 => {
                                general.songs.search(&general.searchquery.query);
                                general.index.index = 0;
                                general.index.page = 1;
                            }
                            2 => {
                                general.songs.set_artist(general.songs.match_c(), &general.searchquery.query);
                            }
                            3 => {
                                general.songs.set_playlist(general.songs.match_c(), &general.searchquery.query);
                            }
                            _ => {}
                        }
                        general.searchquery.default();
                        continue;
                    }
                    Input::KeyBackspace | Input::Character('\x7f') | Input::Character('\x08') => {
                        general.searchquery.query.pop();
                        continue;
                    }
                    Input::Character(i) => {
                        general.searchquery.query.push(i);
                        continue;
                    }
                    _ => {}
                }
            }
            if key == Input::KeyMouse {
                if let Ok(mevent) = pancurses::getmouse() {
                    if (mevent.bstate & 0x2) != 0 {
                        general.action = general.ui.click(mevent.x, mevent.y);
                    }
                    match general.action {
                        Action::Play(p, f) => {
                            general.index.page = p;
                            general.index.index = f;
                            key = Input::Character(PLAY)
                        }
                        Action::Shuffle => {
                            key = Input::Character(SHUFFLE);
                        }
                        Action::Repeat => {
                            key = Input::Character(LOOP);
                        }
                        Action::Rpc => {
                            general.rpc.renew();
                        }
                        Action::PgDown => {
                            let absolute =
                                absolute_index(0, general.index.page + 1, general.songs.typical_page_size)
                                    < general.songs.filtered_songs.len() - 1;
                            if absolute {
                                general.index.index = 0;
                                general.index.page += 1;
                            }
                        }
                        Action::PgUp => {
                            if general.index.page > 1 {
                                general.index.page -= 1;
                                general.index.index = general.songs.typical_page_size - 1;
                            } else {
                                general.index.index = 0;
                            }
                        }
                        Action::Nothing => (),
                    }
                }
            }
            match key {
                Input::KeyF13 => {
                    // song ended
                    if general.songs.stophandler {
                        continue;
                    } else if !general.state.isloop {
                        match general.songs.set_by_next() {
                            Ok(_) => (),
                            Err(_) => (),
                        }
                    }

                    tx.send(AudioCommand::Play(general.songs.current_song_path()))
                        .unwrap();
                    general.timer.maxlen = general.songs.get_duration();
                    general.timer.fcalc = general.timer.maxlen;
                    general.rpc.init();
                    general.sliding.reset_to(general.songs.current_name());
                }
                Input::KeyF14 => {
                    //duration sent
                    if general.rpc.timer <= Instant::now() && general.rpc.reinit {
                        general.handle_rpc(&rpctx);
                    }
                }
                Input::Character(QUIT) => {
                    tx.send(AudioCommand::Stop).unwrap();
                    break;
                }

                Input::KeyDown | Input::Character(DOWN) => {
                    move_selection(
                        Direction::Down,
                        &mut general,
                        &tx,
                    );
                }

                Input::KeyUp | Input::Character(UP) => {
                    move_selection(
                        Direction::Up,
                        &mut general,
                        &tx,
                    );
                }

                Input::Character(PLAY) => {
                    play_current_song(&mut general, &tx);
                    general.rpc.init();
                }

                Input::Character(SPECIAL) => {
                    general.state.spint = !general.state.spint;
                }

                Input::Character(LOOP) => {
                    general.state.isloop = !general.state.isloop;
                }

                Input::Character(STOP) => {
                    general.songs.stop();
                    tx.send(AudioCommand::Pause).unwrap();
                    rpctx.send(RpcCommand::Clear).unwrap();
                }

                Input::Character(BLACKLIST) => {
                    general.blacklist();
                }

                Input::Character(RESUME) => {
                    if general.songs.current_index == usize::MAX {
                        continue;
                    }
                    general.songs.stophandler = false;
                    tx.send(AudioCommand::Resume).unwrap();
                }
                Input::KeyRight | Input::Character(RIGHT) => {
                    tx.send(AudioCommand::SeekForward).unwrap();
                    general.rpc.renew();
                }
                Input::KeyLeft | Input::Character(LEFT) => {
                    tx.send(AudioCommand::SeekBackward).unwrap();
                    general.rpc.renew();
                }
                Input::Character(SHUFFLE) => {
                    general.songs.shuffle();
                }
                Input::Character(SEARCH) => {
                    general.searchquery.to_mode(1);
                }
                Input::Character(TOP) => {
                    general.index.page = 1;
                    general.index.index = 0;
                }
                Input::Character(CHANGE) => {
                    general.searchquery.to_mode(2);
                }
                Input::Character(SETPLAYLIST) => {
                    general.searchquery.to_mode(3);
                }
                Input::Character(SETNEXT) => {
                    general.songs.set_next(absolute_index(
                        general.index.index,
                        general.index.page,
                        general.songs.typical_page_size,
                    ));
                }
                Input::Character(DESEL) => {
                    general.state.desel = !general.state.desel;
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
    general: &mut GeneralState,
    tx: &Sender<AudioCommand>,
) {
    if general.songs.set_by_pindex(general.index.index, general.index.page) != Err(0) {
        if tx
            .send(AudioCommand::Play(general.songs.current_song_path()))
            .is_err()
        {
            return;
        }

        general.timer.maxlen = general.songs.get_duration();

        general.timer.fcalc = general.timer.maxlen;

        general.sliding.reset_to(general.songs.current_name());
    }
}

pub enum Direction {
    Up,
    Down,
}

pub fn move_selection(
    direction: Direction,
    general: &mut GeneralState,
    tx: &Sender<AudioCommand>,
) {
    if general.state.spint {
        match direction {
            Direction::Up => general.volume.step_up(),
            Direction::Down => general.volume.step_down(),
        }
        tx.send(AudioCommand::SetVolume(general.volume.as_f32()))
            .unwrap_or_else(|_| ());
    } else {
        match direction {
            Direction::Up => {
                if general.index.index > 0 {
                    general.index.index -= 1;
                } else if general.index.page > 1 {
                    general.index.page -= 1;
                    general.index.index = general.songs.typical_page_size - 1;
                }
            }
            Direction::Down => {
                let absolute = absolute_index(general.index.index, general.index.page, general.songs.typical_page_size)
                    < general.songs.filtered_songs.len() - 1;
                if general.index.index + 1 < general.songs.typical_page_size && absolute {
                    general.index.index += 1;
                } else if absolute {
                    general.index.page += 1;
                    general.index.index = 0;
                }
            }
        }
    }
}
