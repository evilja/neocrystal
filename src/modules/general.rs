use super::curses::Ownership;
use crate::modules::presence::{
    RpcCommunication, rpc_init_autobuild, rpc_pretend_autobuild, rpc_rnw_autobuild,
};
use crate::modules::songs::absolute_index;
use crate::modules::tui_ir::{ColorIntegerSize, Execute};
use crate::modules::utils::ReinitMode;
use glob::glob;
use home::home_dir;
use pancurses::{COLOR_PAIR, Window};
use std::time::{Duration, Instant};

use super::songs::Songs;
use super::tui_ir::UI;
use super::utils::{Indexer, RpcState, SearchQuery, SlidingText, State, Timer, Volume};

pub struct NcursesExec;

impl Execute<Window> for NcursesExec {
    fn cursor(x: usize, y: usize, w: &mut Window) {
        w.mv(y as i32, x as i32);
    }

    fn blob(ptr: *const u8, len: usize, color: ColorIntegerSize, w: &mut Window) {
        w.attron(COLOR_PAIR(color));
        unsafe {
            w.addstr(std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                ptr, len,
            )));
        }
        w.attroff(COLOR_PAIR(color));
    }

    fn flush(w: &mut Window) {
        w.noutrefresh();
        pancurses::doupdate();
    }
}

pub struct GeneralState {
    pub index: Indexer,
    pub timer: Timer,
    pub state: State,
    pub volume: Volume,
    pub ui: UI<Ownership>,
    pub action: Action,
    pub rpc: RpcState,
    pub sliding: SlidingText,
    pub searchquery: SearchQuery,
    pub songs: Songs,
}

impl GeneralState {
    pub fn blacklist(&mut self) {
        self.songs.blacklist(absolute_index(
            self.index.index,
            self.index.page,
            self.songs.typical_page_size,
        ));
    }

    pub fn handle_rpc(&mut self, comm: &RpcCommunication, instant: Instant) {
        match self.rpc.mode {
            ReinitMode::None => (),
            ReinitMode::Renew => {
                comm.send_message(rpc_rnw_autobuild(&self.timer));
            }
            ReinitMode::Pretend => {
                comm.send_message(rpc_pretend_autobuild(&self.timer));
            }
            ReinitMode::Init => {
                comm.send_message(rpc_init_autobuild(
                    &self.songs,
                    self.timer.maxlen.as_secs_f32() as u64,
                    instant,
                ));
            }
        }
        self.rpc.reset();
    }

    pub fn new() -> Self {
        Self {
            index: Indexer { page: 1, index: 0 },
            timer: Timer::new(),
            state: State {
                spint: false,
                isloop: false,
                desel: false,
                mouse_support: true,
                needs_update: true,
                needs_dbus: true,
            },
            volume: Volume {
                steps: 50,
                step_div: 1,
            },
            ui: UI::new(50, 20),
            action: Action::Nothing,
            rpc: RpcState {
                reinit: false,
                timer: Instant::now(),
                mode: ReinitMode::None,
            },
            sliding: SlidingText::new("Nothing", 23, Duration::from_millis(300)),
            searchquery: SearchQuery {
                mode: 0,
                query: String::from("false"),
            },
            songs: Songs::constructor(globwrap()),
        }
    }
}

fn home() -> String {
    home_dir()
        .expect("No home directory found")
        .join("Music")
        .join("*.*")
        .to_string_lossy()
        .to_string()
}
fn globwrap() -> Vec<String> {
    glob(&home())
        .unwrap()
        .filter_map(Result::ok)
        .map(|p| p.display().to_string())
        .collect::<Vec<String>>()
}

#[allow(dead_code)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Action {
    Play(usize, usize),
    Shuffle,
    Repeat,
    Rpc,
    PgDown,
    PgUp,
    Stop,
    Resume,
    DbusNext,
    DbusPrev,
    Nothing,
}
