
extern crate pancurses;
extern crate glob;
use std::thread;
use std::sync::mpsc::{self, Receiver, Sender};
mod modules;
use crate::modules::audio::AudioReportAction;
use crate::modules::curses::init_locale;
use crate::modules::{audio::{play_audio, AudioCommand}, crystal_manager::crystal_manager};
fn main() { // establish communications and threads, then give the job to crystal_manager fn
    init_locale();
    let (tx, rx): (Sender<AudioCommand>, Receiver<AudioCommand>)                = mpsc::channel();
    let (tx_proc, rx_proc): (Sender<AudioReportAction>, Receiver<AudioReportAction>)                                    = mpsc::channel();
    thread::spawn(move || {
        match play_audio(rx, tx_proc) {
            Ok(_) => {
                ()
            },
            Err(_) => {
                ()
            }
        }
    });

    tx.send(AudioCommand::SetVolume(0.5)).unwrap();

    crystal_manager(tx, rx_proc);

}

