use crate::modules::songs::Songs;
use crate::modules::utils::Timer;
use std::sync::mpsc::Receiver;
#[cfg(feature = "rpc")]
use std::sync::mpsc::{Sender, self};
#[cfg(feature = "rpc")]
use std::thread;
#[cfg(feature = "rpc")]
use std::time::Duration;
use std::time::Instant;
#[cfg(feature = "rpc")]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "rpc")]
use discord_presence::{models::ActivityType, Client};


pub struct RpcCommunication {
    #[cfg(feature = "rpc")]
    sender: Sender<RpcCommand>
}

impl RpcCommunication {
    pub fn new() -> (Self, Option<Receiver<RpcCommand>>) {
        #[cfg(feature = "rpc")]
        {
            let (sender, receiver): (Sender<RpcCommand>, Receiver<RpcCommand>) = mpsc::channel();
            (Self {
                sender: sender
            }, Some(receiver))
        }
        #[cfg(not(feature = "rpc"))]
        {
            (Self {
            }, None)
        }
    }
    pub fn send_message(&self, _command: RpcCommand) {
        #[cfg(feature = "rpc")]
        self.sender.send(_command).unwrap();
    }
}
// #[cfg(feature = "rpc")]
// #[cfg(not(feature = "rpc"))]
#[cfg(feature = "rpc")]
pub enum RpcCommand {
    Init(String, String, u64, Instant),
    Renew(u64),
    Stop,
    Clear,
    Pretend(u64),
}
#[cfg(not(feature = "rpc"))]
pub enum RpcCommand {
    Init,
    Renew,
    Stop,
    Clear,
    Pretend,
}
pub fn rpc_init_autobuild(_songs: &Songs, _stamp: u64, _instant: Instant) -> RpcCommand {
    #[cfg(feature = "rpc")]
    return RpcCommand::Init(
        _songs.current_name(),
        _songs.current_artist(),
        _stamp,
        _instant
    );
    #[cfg(not(feature = "rpc"))]
    return RpcCommand::Init;
}

pub fn rpc_rnw_autobuild(_timer: &Timer) -> RpcCommand {
    #[cfg(feature = "rpc")]
    return RpcCommand::Renew(
        _timer
            .maxlen
            .checked_sub(_timer.fcalc)
            .unwrap_or_default()
            .as_secs(), // elapsed time as u64
    );

    #[cfg(not(feature = "rpc"))]
    return RpcCommand::Renew;
}
pub fn rpc_pretend_autobuild(_timer: &Timer) -> RpcCommand {
    #[cfg(feature = "rpc")]
    return RpcCommand::Pretend(
        _timer
            .maxlen
            .checked_sub(_timer.fcalc)
            .unwrap_or_default()
            .as_secs(), // elapsed time as u64
    );

    #[cfg(not(feature = "rpc"))]
    return RpcCommand::Pretend;
}

#[cfg(feature = "rpc")]
#[derive(PartialEq)]
enum Init {
    Yes,
    No,
    Pretend,
}
#[cfg(feature = "rpc")]
struct Epoch {
    epoch: Duration,
    instant: Instant,
}
#[cfg(feature = "rpc")]
impl Epoch {
    pub fn to_epoch(&self, instant: Instant) -> Duration {
        self.epoch + instant.duration_since(self.instant)
    }
}

#[cfg(not(feature = "rpc"))]
pub fn rpc_handler(_: Receiver<RpcCommand>) {}

#[cfg(feature = "rpc")]
pub fn rpc_handler(comm_recv: Receiver<RpcCommand>) {
    let mut drpc = Client::new(1003981361079668829);
    drpc.on_ready(|_ctx| ()).persist();

    drpc.on_error(|_ctx| ()).persist();
    drpc.start();
    let mut st_ts: (u64, u64) = (0, 0);
    let mut title: String = "".to_string();
    let mut detai: String = "".to_string();
    let mut ed_ts: (u64, u64) = (0, 0);
    let mut init: Init = Init::No;
    let timer = Epoch {
        epoch: SystemTime::now().duration_since(UNIX_EPOCH).unwrap(),
        instant: Instant::now(),
    };

    loop {
        match comm_recv.recv() {
            Ok(rc) => {
                match rc {
                    RpcCommand::Stop => break,

                    RpcCommand::Clear => {
                        let _ = drpc.clear_activity();
                        init = Init::Pretend;
                        continue;
                    }

                    RpcCommand::Renew(time) => {
                        if init != Init::Yes {
                            continue;
                        }
                        st_ts.1 = timer.to_epoch(Instant::now())
                            .as_secs()
                            - time;
                        ed_ts.1 = ed_ts.0 - st_ts.0 + st_ts.1;
                    }
                    RpcCommand::Pretend(time) => {
                        if init != Init::Pretend {
                            continue;
                        }
                        st_ts.1 = timer.to_epoch(Instant::now())
                            .as_secs()
                            - time;
                        ed_ts.1 = ed_ts.0 - st_ts.0 + st_ts.1;
                        init = Init::Yes;
                    }

                    RpcCommand::Init(name, artist, time, instant) => {
                        st_ts.0 = timer.to_epoch(instant)
                            .as_secs();
                        ed_ts.0 = st_ts.0 + time;
                        title = artist;
                        detai = name;
                        st_ts.1 = st_ts.0;
                        ed_ts.1 = ed_ts.0;
                        init = Init::Yes;
                    }
                };
                let _ = drpc.set_activity(|act| {
                        act.activity_type(ActivityType::Listening)
                            .state(&title)
                            .details(&detai)
                            .assets(|ass| {
                                ass.large_image("001")
                                    .small_image("github")
                                    .small_text("github.com/evilja/neocrystal")
                            })
                            .timestamps(|ts| ts.start(st_ts.1).end(ed_ts.1))
                            
                    });
            }
            Err(_) => thread::sleep(Duration::from_secs(1)),
        }
    }
}
