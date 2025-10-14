use discord_presence::{Client, models::ActivityType};
use std::sync::mpsc::Receiver;
use std::time::Duration;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use super::utils::artist_data;

pub fn rpc_handler(comm_recv: Receiver<(String, u64)>) {
    let mut drpc = Client::new(1003981361079668829);
    drpc.on_ready(|_ctx| {
        println!("READY!");
    })
    .persist();

    drpc.on_error(|ctx| {
        eprintln!("An error occured, {:?}", ctx.event);
    })
    .persist();
    drpc.start();
    loop {
        match comm_recv.recv() {
            Ok((x, y)) => {
                if x == "stop" {
                    break;
                }
                drpc.clear_activity().unwrap();
                let st_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                let title = &artist_data(&x);
                let ed_ts = st_ts + y;
                println!("st_ts is {}, ed_ts is {}", st_ts, ed_ts);
                
                loop {
                    match drpc.set_activity(|act| {
                    act
                        .activity_type(ActivityType::Listening)
                        .state(title)
                        .details(x.clone().replace("music/", "").replace("music\\", "").replace(".mp3", ""))
                        .assets(|ass| {
                            ass.small_image("github")
                                .small_text("github.com/evilja/neo-crystal-plus")
                                .large_image("default")
                                .large_text("Crystal+ by Myisha")
                        })
                        .timestamps(|ts| {
                            ts.start(st_ts)
                            .end(ed_ts)
                        })
                    }) {
                        Ok(_) => break,
                        Err(_) => thread::sleep(Duration::from_secs(1)),
                    }
                }
            }
            Err(_) => thread::sleep(Duration::from_secs(1)),
        }
    }
}