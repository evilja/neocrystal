use std::time::{Duration, Instant};
use super::curses::Ownership;
use crate::modules::presence::RpcCommand;
use crate::modules::songs::absolute_index;
use crate::modules::utils::ReinitMode;
use std::sync::mpsc::Sender;
use crate::modules::presence::rpc_init_autobuild;
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

    pub fn handle_rpc(&mut self, rpctx: &Sender<RpcCommand>) {
        match self.rpc.mode {
            ReinitMode::None => (),
            ReinitMode::Renew => {
                let _ = rpctx.send(RpcCommand::Renew(
                    self.timer
                        .maxlen
                        .checked_sub(self.timer.fcalc)
                        .unwrap_or_default()
                        .as_secs(), // elapsed time as u64
                ));
            }
            ReinitMode::Pretend => {
                let _ = rpctx.send(RpcCommand::Pretend(
                    self.timer
                        .maxlen
                        .checked_sub(self.timer.fcalc)
                        .unwrap_or_default()
                        .as_secs(), // elapsed time as u64
                ));
            }
            ReinitMode::Init => {
                let _ = rpctx.send(rpc_init_autobuild(
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
                mouse_support: false,
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