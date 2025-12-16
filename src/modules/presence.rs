use crate::modules::songs::Songs;
use discord_presence::{models::ActivityType, Client};
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

pub enum RpcCommand {
    Init(String, String, String, u64),
    Renew(u64),
    Stop,
    Clear,
    Pretend(u64),
}

pub fn rpc_init_autobuild(songs: &Songs, stamp: u64) -> RpcCommand {
    RpcCommand::Init(
        songs.current_name(),
        songs.current_artist(),
        songs.current_playlist(),
        stamp,
    )
}

#[derive(PartialEq)]
enum Init {
    Yes,
    No,
    Pretend,
}

pub fn rpc_handler(comm_recv: Receiver<RpcCommand>) {
    let mut drpc = Client::new(1003981361079668829);
    drpc.on_ready(|_ctx| ()).persist();

    drpc.on_error(|_ctx| ()).persist();
    drpc.start();
    let mut st_ts: (u64, u64) = (0, 0);
    let mut title: String = "".to_string();
    let mut detai: String = "".to_string();
    let mut plist: String = "Crystal+ by Myisha".to_string();
    let mut ed_ts: (u64, u64) = (0, 0);
    let mut init: Init = Init::No;
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
                        st_ts.1 = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                            - time;
                        ed_ts.1 = ed_ts.0 - st_ts.0 + st_ts.1;
                    }
                    RpcCommand::Pretend(time) => {
                        if init != Init::Pretend {
                            continue;
                        }
                        st_ts.1 = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                            - time;
                        ed_ts.1 = ed_ts.0 - st_ts.0 + st_ts.1;
                        init = Init::Yes;
                    }

                    RpcCommand::Init(name, artist, playlist, time) => {
                        st_ts.0 = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                            - 3;
                        ed_ts.0 = st_ts.0 + time;
                        title = artist;
                        detai = name;
                        plist = playlist;
                        st_ts.1 = st_ts.0;
                        ed_ts.1 = ed_ts.0;
                        init = Init::Yes;
                    }
                };
                for _ in 0..=5 {
                    match drpc.set_activity(|act| {
                        act.activity_type(ActivityType::Listening)
                            .state(&title)
                            .details(&detai)
                            .assets(|ass| {
                                ass.large_image("001")
                                    .large_text(&plist)
                                    .small_image("github")
                                    .small_text("github.com/evilja/neo-crystal-plus")
                            })
                            .timestamps(|ts| ts.start(st_ts.1).end(ed_ts.1))
                    }) {
                        Ok(_) => break,
                        Err(_) => thread::sleep(Duration::from_secs(3)),
                    }
                }
            }
            Err(_) => thread::sleep(Duration::from_secs(1)),
        }
    }
}
