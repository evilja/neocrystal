use discord_presence::{Client, models::ActivityType};
use std::sync::mpsc::Receiver;
use std::time::Duration;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use super::utils::artist_data;


pub fn rpc_handler(comm_recv: Receiver<(String, u64)>) {
    let mut drpc = Client::new(1003981361079668829);
    drpc.on_ready(|_ctx| {
        ()
    })
    .persist();

    drpc.on_error(|ctx| {
        ()
    })
    .persist();
    drpc.start();
    let mut st_ts: (u64, u64) = (0, 0);
    let mut title: String = "".to_string();
    let mut detai: String = "".to_string();
    let mut ed_ts: (u64, u64) = (0, 0);
    loop {
        match comm_recv.recv() {
            Ok((x, y)) => {
                if x == "%stop" {
                    break;
                } else if x == "%clear" {
                    let _ = drpc.clear_activity();
                    continue;
                } else if x == "%renew" { 
                    st_ts.1 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - y;
                    ed_ts.1 = ed_ts.0 - st_ts.0 + st_ts.1;
                } else {
                    st_ts.0 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - 3;
                    title = artist_data(&x);
                    detai = x.replace("music/", "").replace("music\\", "").replace(".mp3", "");
                    ed_ts.0 = st_ts.0 + y;
                    let _ = drpc.clear_activity();
                    st_ts.1 = st_ts.0;
                    ed_ts.1 = ed_ts.0;
                }
                for _ in 0..=5 {
                    match drpc.set_activity(|act| {
                    act
                        .activity_type(ActivityType::Listening)
                        .state(&title)
                        .details(&detai)
                        .assets(|ass| {
                            ass
                                .small_image("github")
                                .small_text("github.com/evilja/neo-crystal-plus")
                                .large_image("default")
                                .large_text("Crystal+ by Myisha")
                        })
                        .timestamps(|ts| {
                            ts
                                .start(st_ts.1)
                                .end(ed_ts.1)
                        })
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
