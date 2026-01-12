extern crate glob;
extern crate pancurses;
use super::general::{Action, GeneralState};
use crate::modules::audio::{AudioCommand, AudioReportAction};
#[cfg(not(target_os = "windows"))]
use crate::modules::dbus::spawn_mpris;
#[cfg(not(target_os = "windows"))]
use crate::modules::mouse::{self};
use crate::modules::presence;
use pancurses::{Input, initscr};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self};
use std::time::Instant;

use super::{
    curses::*,
    presence::{RpcCommand, rpc_handler},
    songs::absolute_index,
};

pub const UP: char = 'u';
pub const DOWN: char = 'j';
pub const LEFT: char = 'n';
pub const RIGHT: char = 'm';
pub const SHUFFLE: char = 'f';
pub const PLAY: char = 'p';
pub const BLACKLIST: char = 'b';
pub const STOP: char = 's';
pub const RESUME: char = 'r';
pub const LOOP: char = 'l';
pub const SPECIAL: char = 'o';
pub const QUIT: char = 'q';
pub const SEARCH: char = 'h';
pub const FULL: char = 'g';
pub const CHANGE: char = 'c';
pub const SETNEXT: char = 'e';
pub const DESEL: char = 'd';
pub const SETPLAYLIST: char = 'v';
pub const MOUSE_SUPPORT: char = 't';

/// macro: get_input_or_report
/// try to get input from pancurses:
/// success -> return the key
/// fail -> try to get information from comm_rx (audio thread)
///      -> success -> set local timer fcalc to the value &&
///                 check if fcalc is smaller than 100 milliseconds && info belongs to the current song
///                 true -> return f13 meaning song ended. 100 milliseconds because get_pos is inconsistent
///                 false -> return f14 meaning a duration is sent, triggering rpc things and such.
///      -> fail -> check d-bus commands
macro_rules! get_input_or_report {
    ($window:expr, $comm_rx:expr, $general:expr, $loctimer:expr, $timeout_ms:expr) => {{
        // 1) Try real input first
        let mut key = $window.getch().or_else(|| {
            match $comm_rx.recv_timeout(std::time::Duration::from_millis($timeout_ms)) {
                Ok(msg) => match msg {
                    crate::AudioReportAction::Duration(name, time) => {
                        if name == $general.songs.current_song_path() {
                            $loctimer.fcalc = time;
                            if $loctimer.fcalc <= std::time::Duration::from_millis(100) {
                                Some(pancurses::Input::KeyF13)
                            } else {
                                Some(pancurses::Input::KeyF14)
                            }
                        } else {
                            Some(pancurses::Input::KeyF15)
                        }
                    }
                    _ => Some(pancurses::Input::KeyF15),
                },
                Err(_) => None,
            }
        });

        // 2) If no real input, but D-Bus action exists, synthesize equivalent key
        if key.is_none() && $general.action != Action::Nothing {
            if let Some(nk) = crate::modules::mouse::action_to_key($general.action, &mut $general) {
                key = Some(nk);
            }
        }

        key
    }};
}

pub fn crystal_manager(tx: Sender<AudioCommand>, comm_rx: Receiver<AudioReportAction>) -> bool {
    let mut window = initscr();
    let (dbus_action_tx, dbus_action_rx): (Sender<Action>, Receiver<Action>) = mpsc::channel();
    let mut general: GeneralState = GeneralState::new();

    #[cfg(not(target_os = "windows"))]
    let mpris = spawn_mpris(dbus_action_tx.clone());

    let mut page = PageData::new();

    let rpc_comm = {
        let (rpc_comm, receiver) = presence::RpcCommunication::new();
        if let Some(rx) = receiver {
            let _rpc_thread = thread::spawn(move || {
                rpc_handler(rx);
            });
        }
        rpc_comm
    };

    init_curses(&mut window);
    autoalloc(&mut general);
    draw_all(&mut general, &mut page);
    loop {
        if general.state.needs_update {
            update(&mut general, &mut window);
            general.state.needs_update = false;
        }
        if general.state.needs_dbus {
            #[cfg(not(target_os = "windows"))]
            {
                let mut s = mpris.state.lock().unwrap();

                if general.songs.current_index == usize::MAX {
                    s.playback_status = 2;
                    s.title = "Nothing".into();
                    s.artist = vec!["Nothing".into()];
                    s.length_us = 0;
                } else if general.songs.stophandler {
                    s.playback_status = 1;
                } else {
                    s.playback_status = 0;
                    s.title = general.songs.current_name();
                    s.artist = vec![general.songs.current_artist()];
                    s.length_us = general.songs.get_duration().as_micros() as i64;
                }
            }
            #[cfg(not(target_os = "windows"))]
            mpris.emit();
            general.state.needs_dbus = false;
        }
        while let Ok(action) = dbus_action_rx.try_recv() {
            general.action = action;
        }
        // key_opt catches either duration communications from audio thread or user input
        // if nothing is there to catch, it will just skip after 10 milliseconds           there
        let key_opt = get_input_or_report!(window, comm_rx, general, general.timer, 10);

        if let Some(mut key) = key_opt {
            general.state.needs_update = true;
            if general.searchquery.mode != 0 {
                match key {
                    Input::KeyEnter | Input::Character('\n') => {
                        match general.searchquery.mode {
                            1 => {
                                general.songs.search(&general.searchquery.query);
                                general.index.index = 0;
                                general.index.page = 1;
                                page.draw_changed_moved_page(&mut general);
                                page.draw_indicators(&mut general);
                            }
                            2 => {
                                general.songs.set_artist(
                                    general.songs.match_c(),
                                    &general.searchquery.query,
                                );
                                draw_artist(&mut general);
                                general.state.needs_dbus = true;
                            }
                            3 => {
                                general.songs.set_playlist(
                                    general.songs.match_c(),
                                    &general.searchquery.query,
                                );
                                draw_playlist(&mut general);
                            }
                            _ => {}
                        }
                        general.searchquery.default();
                        draw_header(&mut general);
                        continue;
                    }
                    Input::KeyBackspace | Input::Character('\x7f') | Input::Character('\x08') => {
                        general.searchquery.query.pop();
                        draw_header(&mut general);
                        continue;
                    }
                    Input::Character(i) => {
                        general.searchquery.query.push(i);
                        draw_header(&mut general);
                        continue;
                    }
                    _ => {}
                }
            }
            if key == Input::KeyMouse {
                if !(general.state.mouse_support) {
                    continue;
                }
                if let Ok(mevent) = pancurses::getmouse() {
                    if let Some(action) = mouse::handle_mouse(mevent, &general) {
                        general.action = action;
                    }
                }
            }
            if general.action != Action::Nothing {
                if let Some(nk) = mouse::action_to_key(general.action, &mut general) {
                    key = nk;
                }
            }
            match key {
                Input::KeyNext => {
                    // song ended but ignore loop. this is used from D-Bus or keyboard but mainly dbus
                    if general.songs.stophandler {
                        continue;
                    } else {
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
                    draw_artist(&mut general);
                    draw_playlist(&mut general);
                    draw_sliding(&mut general);
                    draw_time_max(&mut general);
                    draw_time_cur(&mut general);
                    page.draw_indicators(&mut general);
                    draw_rpc_indc(&mut general);
                    general.state.needs_dbus = true;
                }
                Input::KeyPrevious => {
                    if general.songs.stophandler {
                        continue;
                    } else {
                        match general.songs.prev() {
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
                    draw_artist(&mut general);
                    draw_playlist(&mut general);
                    draw_sliding(&mut general);
                    draw_time_max(&mut general);
                    draw_time_cur(&mut general);
                    page.draw_indicators(&mut general);
                    draw_rpc_indc(&mut general);
                    general.state.needs_dbus = true;
                }
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
                    draw_artist(&mut general);
                    draw_playlist(&mut general);
                    draw_sliding(&mut general);
                    draw_time_max(&mut general);
                    draw_time_cur(&mut general);
                    page.draw_indicators(&mut general);
                    draw_rpc_indc(&mut general);
                    general.state.needs_dbus = true;
                }
                Input::KeyF14 => {
                    //duration sent
                    if general.rpc.timer <= Instant::now() && general.rpc.reinit {
                        general.handle_rpc(&rpc_comm, general.rpc.timer);
                        draw_rpc_indc(&mut general);
                    }
                    draw_progress(&mut general);
                    draw_time_cur(&mut general);
                    if general.sliding.is_changing() {
                        draw_sliding(&mut general);
                    }
                }
                Input::Character(QUIT) => {
                    tx.send(AudioCommand::Stop).unwrap();
                    break;
                }

                Input::KeyDown | Input::Character(DOWN) => {
                    move_selection(Direction::Down, &mut general, &tx, &mut page);
                }

                Input::KeyUp | Input::Character(UP) => {
                    move_selection(Direction::Up, &mut general, &tx, &mut page);
                }

                Input::Character(PLAY) => {
                    if !play_current_song(&mut general, &tx) {
                        continue;
                    };
                    general.rpc.init();
                    draw_artist(&mut general);
                    draw_playlist(&mut general);
                    draw_sliding(&mut general);
                    draw_time_max(&mut general);
                    draw_time_cur(&mut general);
                    page.draw_indicators(&mut general);
                    draw_rpc_indc(&mut general);
                    draw_progress(&mut general);
                    match general.action {
                        #[cfg(feature = "mouse")]
                        Action::Play(_, _) => {
                            page.draw_unchanged_moved_page(&mut general);
                        }
                        _ => (),
                    }
                    general.state.needs_dbus = true;
                }

                Input::Character(SPECIAL) => {
                    general.state.spint = !general.state.spint;
                }

                Input::Character(LOOP) => {
                    general.state.isloop = !general.state.isloop;
                    draw_loop_indc(&mut general);
                    page.draw_indicators(&mut general);
                }

                Input::Character(STOP) => {
                    general.songs.stop();
                    tx.send(AudioCommand::Pause).unwrap();
                    rpc_comm.send_message(RpcCommand::Clear);
                    page.draw_indicators(&mut general);
                    general.state.needs_dbus = true;
                }

                Input::Character(BLACKLIST) => {
                    general.blacklist();
                    page.draw_indicators(&mut general);
                }

                Input::Character(RESUME) => {
                    if general.songs.current_index == usize::MAX {
                        continue;
                    }
                    general.songs.resume();
                    tx.send(AudioCommand::Resume).unwrap();
                    general.rpc.pretend();
                    page.draw_indicators(&mut general);
                    draw_rpc_indc(&mut general);
                    general.state.needs_dbus = true;
                }
                Input::KeyRight | Input::Character(RIGHT) => {
                    tx.send(AudioCommand::SeekForward).unwrap();
                    general.rpc.renew();
                    draw_rpc_indc(&mut general);
                }
                Input::KeyLeft | Input::Character(LEFT) => {
                    tx.send(AudioCommand::SeekBackward).unwrap();
                    general.rpc.renew();
                    draw_rpc_indc(&mut general);
                }
                Input::Character(SHUFFLE) => {
                    general.songs.shuffle();
                    page.draw_indicators(&mut general);
                    draw_shuffle_indc(&mut general);
                }
                Input::Character(SEARCH) => {
                    general.searchquery.to_mode(1);
                    draw_search(&mut general);
                }
                Input::Character(FULL) => {
                    window.clear();
                    draw_all(&mut general, &mut page);
                }
                Input::Character(CHANGE) => {
                    general.searchquery.to_mode(2);
                    draw_search(&mut general);
                }
                Input::Character(SETPLAYLIST) => {
                    general.searchquery.to_mode(3);
                    draw_search(&mut general);
                }
                Input::Character(SETNEXT) => {
                    general.songs.set_next(absolute_index(
                        general.index.index,
                        general.index.page,
                        general.songs.typical_page_size,
                    ));
                    page.draw_indicators(&mut general);
                }
                Input::Character(DESEL) => {
                    general.state.desel = !general.state.desel;
                    page.draw_changed_moved_page(&mut general);
                }
                Input::KeyPPage => {
                    change_page(Direction::Up, &mut general, &mut page);
                    page.draw_indicators(&mut general);
                    page.draw_changed_moved_page(&mut general);
                    draw_page(&mut general);
                }
                Input::KeyNPage => {
                    change_page(Direction::Down, &mut general, &mut page);
                    page.draw_indicators(&mut general);
                    page.draw_changed_moved_page(&mut general);
                    draw_page(&mut general);
                }
                Input::Character(MOUSE_SUPPORT) => {
                    general.state.mouse_support = !general.state.mouse_support;
                }

                _ => (),
            }
        }
        general.action = Action::Nothing;
    }
    match rpc_comm.send_message(RpcCommand::Stop) {
        _ => (),
    }
    exit_curses(&mut window);
    true
}

pub fn play_current_song(general: &mut GeneralState, tx: &Sender<AudioCommand>) -> bool {
    if general
        .songs
        .set_by_pindex(general.index.index, general.index.page)
        != Err(0)
    {
        if tx
            .send(AudioCommand::Play(general.songs.current_song_path()))
            .is_err()
        {
            return false;
        }

        general.timer.maxlen = general.songs.get_duration();

        general.timer.fcalc = general.timer.maxlen;

        general.sliding.reset_to(general.songs.current_name());
        return true;
    } else {
        return false;
    }
}

pub fn change_page(dir: Direction, general: &mut GeneralState, page: &mut PageData) {
    let psize = general.songs.typical_page_size.max(1);
    let total = general.songs.filtered_songs.len();

    if total == 0 {
        return;
    }

    let max_page = (total + psize - 1) / psize;

    match dir {
        Direction::Up => {
            if general.index.page > 1 {
                general.index.page -= 1;
                general.index.index = 0;
                draw_page(general);
                page.draw_changed_moved_page(general);
                page.draw_indicators(general);
            }
        }

        Direction::Down => {
            if general.index.page < max_page {
                general.index.page += 1;
                general.index.index = 0;
                draw_page(general);
                page.draw_changed_moved_page(general);
                page.draw_indicators(general);
            }
        }
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
    page: &mut PageData,
) {
    if general.state.spint {
        match direction {
            Direction::Up => general.volume.step_up(),
            Direction::Down => general.volume.step_down(),
        }
        tx.send(AudioCommand::SetVolume(general.volume.as_f32()))
            .unwrap_or_else(|_| ());
        draw_vol_indc(general);
    } else if !general.songs.filtered_songs.is_empty() {
        match direction {
            Direction::Up => {
                if general.index.index > 0 {
                    general.index.index -= 1;
                    page.draw_unchanged_moved_page(general);
                } else if general.index.page > 1 {
                    general.index.page -= 1;
                    general.index.index = general.songs.typical_page_size - 1;
                    draw_page(general);
                    page.draw_indicators(general);
                    page.draw_changed_moved_page(general);
                }
            }
            Direction::Down => {
                let absolute = absolute_index(
                    general.index.index,
                    general.index.page,
                    general.songs.typical_page_size,
                ) < general.songs.filtered_songs.len() - 1;
                if general.index.index + 1 < general.songs.typical_page_size && absolute {
                    general.index.index += 1;
                    page.draw_unchanged_moved_page(general);
                } else if absolute {
                    general.index.page += 1;
                    general.index.index = 0;
                    draw_page(general);
                    page.draw_indicators(general);
                    page.draw_changed_moved_page(general);
                }
            }
        }
    }
}
