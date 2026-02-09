use audiotags::{Album, Tag};

use unicode_width::UnicodeWidthStr;
use unicode_segmentation::UnicodeSegmentation;
use std::time::{Duration, Instant};

pub struct Volume {
    pub steps: u8,
    pub step_div: u8,
}

impl Volume {
    pub fn step_up(&mut self) {
        self.steps += self.step_div;
        if self.steps > 100 {
            self.steps = 100;
        }
    }
    pub fn step_down(&mut self) {
        if self.step_div > self.steps {
            self.steps = 0;
        } else {
            self.steps -= self.step_div;
        }
    }
    pub fn as_f32(&self) -> f32 {
        self.steps as f32 / 100 as f32
    }
}

pub fn artist_data(filepath: &str) -> String {
    let tag =  Tag::new().read_from_path(filepath);
    match tag {
        Ok(t) => {
            let artist = t.artists().unwrap_or(vec!["Unknown"]).join(", ");
            artist
        }
        Err(_) => "Unknown".to_string(),
    }
}
pub fn album_data(filepath: &str) -> String {
    let tag = Tag::new().read_from_path(filepath);
    match tag {
        Ok(t) => {
            let artist = t.album_title();
            match artist {
                Some(a) => a.to_string(),
                None => "".to_string(),
            }
        }
        Err(_) => "".to_string(),
    }
}
pub fn addto_album(filepath: &str, new_playlist: &str) -> Result<(), String> {
    let mut tag =
        Tag::new().read_from_path(filepath).map_err(|e| format!("Failed to read ID3 tag: {}", e))?;
    tag.set_album(Album::with_title(new_playlist));
    tag.write_to_path(filepath)
        .map_err(|e| format!("Failed to write ID3 tag: {}", e))?;
    Ok(())
}
pub fn change_artist(filepath: &str, new_artist: &str) -> Result<(), String> {
    let mut tag =
        Tag::new().read_from_path(filepath).map_err(|e| format!("Failed to read ID3 tag: {}", e))?;
    tag.set_artist(new_artist);
    tag.write_to_path(filepath)
        .map_err(|e| format!("Failed to write ID3 tag: {}", e))?;
    Ok(())
}

pub struct SlidingText {
    graphemes: Vec<String>,
    grapheme_width: usize,
    width: usize,
    offset: usize,
    last_tick: Instant,
    speed: Duration,
}

impl SlidingText {
    pub fn new(text: impl Into<String>, width: usize, speed: Duration) -> Self {
        let text = text.into() + "   "; // keep your padding
        let graphemes = text.graphemes(true).map(|g| g.to_string()).collect();

        Self {
            graphemes,
            grapheme_width: 0,
            width,
            offset: 0,
            last_tick: Instant::now(),
            speed,
        }
    }

    pub fn is_changing(&mut self) -> bool {
        !(self.grapheme_width <= self.width)
    }

    pub fn reset_to(&mut self, new_text: impl Into<String>) {
        let text = new_text.into() + "   ";
        self.graphemes = text.graphemes(true).map(|g| g.to_string()).collect();
        self.grapheme_width = self.graphemes.iter().map(|g| g.width()).sum();
        self.offset = 0;
        self.last_tick = Instant::now();
    }

    fn _update(&mut self) {
        if self.last_tick.elapsed() >= self.speed {
            self.offset = (self.offset + 1) % self.graphemes.len();
            self.last_tick = Instant::now();
        }
    }

    pub fn visible_text(&mut self) -> String {
        self._update();

        if self.graphemes.is_empty() {
            return String::new();
        }

        if self.grapheme_width <= self.width {
            // Trim exactly 3 trailing graphemes
            let trimmed = &self.graphemes[..self.graphemes.len().saturating_sub(3)];
            return trimmed.concat();
        }

        // Normal sliding behavior
        let mut visible = String::new();
        let mut idx = self.offset;

        while visible.width() < self.width {
            visible.push_str(&self.graphemes[idx]);
            idx = (idx + 1) % self.graphemes.len();
        }

        visible
    }
}

pub struct SearchQuery {
    pub mode: u8,
    pub query: String,
}
impl SearchQuery {
    pub fn default(&mut self) {
        self.mode = 0;
        self.query = String::from("false");
    }
    pub fn to_mode(&mut self, mode: u8) {
        self.mode = mode;
        self.query = String::new();
    }
}
#[derive(PartialEq)]
pub enum ReinitMode {
    Renew,
    Init,
    None,
    Pretend,
}
pub struct Timer {
    pub fcalc: Duration,
    pub maxlen: Duration,
}
impl Timer {
    pub fn new() -> Self {
        Self {
            fcalc: Duration::ZERO,
            maxlen: Duration::ZERO,
        }
    }
}
pub struct State {
    pub spint: bool,
    pub isloop: bool,
    pub desel: bool,
    pub mouse_support: bool,
    pub needs_update: bool,
    pub needs_dbus: bool,
}
pub struct RpcState {
    pub reinit: bool,
    pub timer: Instant,
    pub mode: ReinitMode,
}
pub struct Indexer {
    pub page: usize,
    pub index: usize,
}

impl RpcState {
    fn _init(&mut self) {
        self.reinit = true;
        self.timer = Instant::now() + Duration::from_secs(1);   
    }

    pub fn renew(&mut self) {
        self._init();
        self.mode = ReinitMode::Renew;
    }
    pub fn pretend(&mut self) {
        self._init();
        self.mode = ReinitMode::Pretend;
    }
    pub fn init(&mut self) {
        self._init();
        self.mode = ReinitMode::Init;
    }
    pub fn reset(&mut self) {
        self.reinit = false;
        self.timer = Instant::now();
        self.mode = ReinitMode::None;
    }
}
