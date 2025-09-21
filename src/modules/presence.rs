use discord_presence::Client;
use std::sync::mpsc::Receiver;
use std::time::Duration;
use std::thread;

pub fn rpc_handler(comm_recv: Receiver<(String, &'static str)>) {
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
                loop {
                    match drpc.set_activity(|act| {
                    act.state(y)
                        .details(x.clone().replace("music/", "").replace("music\\", "").replace(".mp3", ""))
                        .assets(|ass| {
                            ass.small_image("github")
                                .small_text("github.com/evilja/neo-crystal-plus")
                                .large_image("default")
                                .large_text("Crystal+ by Myisha")
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