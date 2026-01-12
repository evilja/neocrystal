// NOTICE:
// This module is for Linux D-Bus support
// for things like adding the player to
// a status bar.
// Also this module is a vibeware and HIGHLY DANGEROUS (i didnt read most of it).
// That's because I don't know how to set up a bus client.
// Doesn't mean it sucks though, it works. I'll change its parts when needed.
// Tested on: Hyprland/waybar on Gentoo 23.0 ~amd64 dbus-1.16.2 with USE flags -* X elogind

#![cfg(not(target_os = "windows"))]

use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc::Sender};
use std::thread;

use zbus::blocking::Connection;
use zbus::interface;
use zvariant::Value;

use crate::modules::general::Action;

const BUS_NAME: &str = "org.mpris.MediaPlayer2.neocrystal";
const OBJ_PATH: &str = "/org/mpris/MediaPlayer2";

pub struct MprisHandle {
    pub state: Arc<Mutex<MprisState>>,
    emit_tx: Sender<()>,
}
impl MprisHandle {
    pub fn emit(&self) {
        let _ = self.emit_tx.send(());
    }
}

#[derive(Clone)]
pub struct MprisState {
    pub playback_status: u8, // 0 Playing, 1 Paused, 2 Stopped
    pub title: String,
    pub artist: Vec<String>,
    pub length_us: i64,
}

impl Default for MprisState {
    fn default() -> Self {
        Self {
            playback_status: 2,
            title: "Nothing".into(),
            artist: vec!["Nothing".into()],
            length_us: 0,
        }
    }
}

/// Root interface
struct MprisRoot;

#[interface(name = "org.mpris.MediaPlayer2")]
impl MprisRoot {
    #[zbus(property)]
    fn can_quit(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn can_raise(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn has_track_list(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn identity(&self) -> &str {
        "NeoCrystal"
    }

    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Vec<String> {
        vec!["file".into()]
    }

    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<String> {
        vec!["audio/mp3".into()]
    }
}

/// Player interface
struct MprisPlayer {
    tx: Sender<Action>,
    state: Arc<Mutex<MprisState>>,
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl MprisPlayer {
    fn play(&self) {
        let _ = self.tx.send(Action::Play(1, 0));
    }

    fn pause(&self) {
        let _ = self.tx.send(Action::Stop);
    }

    fn play_pause(&self) {
        let s = self.state.lock().unwrap().playback_status;
        let _ = match s {
            0 => self.tx.send(Action::Stop),
            1 => self.tx.send(Action::Resume),
            _ => self.tx.send(Action::Play(1, 0)),
        };
    }

    fn next(&self) {
        let _ = self.tx.send(Action::DbusNext);
    }

    fn previous(&self) {
        let _ = self.tx.send(Action::DbusPrev);
    }

    #[zbus(property)]
    fn playback_status(&self) -> String {
        match self.state.lock().unwrap().playback_status {
            0 => "Playing",
            1 => "Paused",
            _ => "Stopped",
        }
        .into()
    }

    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, Value<'_>> {
        let s = self.state.lock().unwrap();
        let mut m = HashMap::new();
        m.insert("xesam:title".into(), Value::new(s.title.clone()));
        m.insert("xesam:artist".into(), Value::new(s.artist.clone()));
        m.insert("mpris:length".into(), Value::new(s.length_us));
        m
    }

    #[zbus(property)]
    fn can_play(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_pause(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        true
    }
}

pub fn spawn_mpris(action_tx: Sender<Action>) -> MprisHandle {
    let state = Arc::new(Mutex::new(MprisState::default()));
    let state_clone = state.clone();

    let (emit_tx, emit_rx) = std::sync::mpsc::channel::<()>();

    thread::spawn(move || {
        let conn = Connection::session().expect("D-Bus session failed");
        conn.request_name(BUS_NAME).unwrap();

        let server = conn.object_server();
        server.at(OBJ_PATH, MprisRoot).unwrap();
        server
            .at(
                OBJ_PATH,
                MprisPlayer {
                    tx: action_tx,
                    state: state_clone.clone(),
                },
            )
            .unwrap();

        loop {
            emit_rx.recv().unwrap();

            let s = state_clone.lock().unwrap();

            let mut changed = HashMap::<&str, Value>::new();

            changed.insert(
                "PlaybackStatus",
                Value::new(match s.playback_status {
                    0 => "Playing",
                    1 => "Paused",
                    _ => "Stopped",
                }),
            );

            let mut metadata = HashMap::<&str, Value>::new();
            metadata.insert("xesam:title", Value::new(s.title.clone()));
            metadata.insert("xesam:artist", Value::new(s.artist.clone()));
            metadata.insert("mpris:length", Value::new(s.length_us));

            changed.insert("Metadata", Value::new(metadata));

            let _ = conn.emit_signal(
                None::<&str>,
                OBJ_PATH,
                "org.freedesktop.DBus.Properties",
                "PropertiesChanged",
                &("org.mpris.MediaPlayer2.Player", changed, Vec::<&str>::new()),
            );
        }
    });

    MprisHandle { state, emit_tx }
}
