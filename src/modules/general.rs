use std::time::{Duration, Instant};
use super::curses::Ownership;
use crate::modules::presence::{RpcCommunication, rpc_init_autobuild, rpc_pretend_autobuild, rpc_rnw_autobuild};
use crate::modules::songs::absolute_index;
use crate::modules::utils::ReinitMode;
use glob::glob;
use home::home_dir;


use super::utils::{
    Indexer,
    Timer,
    State,
    Volume,
    RpcState,
    SlidingText,
    SearchQuery
};
use super::tui_borrow::{UI};
use super::songs::Songs;

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
    pub songs: Songs
}

impl GeneralState { 
    pub fn blacklist(&mut self) {
        self.songs.blacklist(absolute_index(
            self.index.index,
            self.index.page,
            self.songs.typical_page_size,
        ));
    }

    pub fn handle_rpc(&mut self, comm: &RpcCommunication) {
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
            },
            volume: Volume {
                steps: 50,
                step_div: 2,
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
        .join("*.mp3")
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

#[cfg(feature = "mouse")]
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Action {
    Play(usize, usize),
    Shuffle,
    Repeat,
    Rpc,
    PgDown,
    PgUp,
    Nothing,
}

#[cfg(not(feature = "mouse"))]
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Action {
    Nothing
}