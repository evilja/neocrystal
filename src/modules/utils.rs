use id3::TagLike;

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
    let tag = id3::Tag::read_from_path(filepath);
    match tag {
        Ok(t) => {
            let artist = t.artists().unwrap_or(vec!["Unknown"]).join(", ");

            artist
        },
        Err(_) => "Unknown".to_string(),
    }
}

pub fn change_artist(filepath: &str, new_artist: &str) -> Result<(), String> {
    let mut tag = id3::Tag::read_from_path(filepath).map_err(|e| format!("Failed to read ID3 tag: {}", e))?;
    tag.set_artist(new_artist);
    tag.write_to_path(filepath, id3::Version::Id3v24).map_err(|e| format!("Failed to write ID3 tag: {}", e))?;
    Ok(())
}

use std::time::{Duration, Instant};

pub struct SlidingText {
    text: String,
    width: usize,
    offset: usize,
    last_tick: Instant,
    speed: Duration, // how often to slide by 1 char
}

impl SlidingText {
    pub fn new(text: impl Into<String>, width: usize, speed: Duration) -> Self {
        let text = text.into();
        Self {
            text: format!("{}   ", text), // add spaces at the end for smooth loop
            width,
            offset: 0,
            last_tick: Instant::now(),
            speed,
        }
    }

    pub fn reset_to(&mut self, new_text: impl Into<String>) {
        self.text = format!("{}   ", new_text.into());
        self.offset = 0;
        self.last_tick = Instant::now();
    }
    /// Advance the offset if enough time passed
    pub fn _update(&mut self) {
        if self.last_tick.elapsed() >= self.speed {
            self.offset = (self.offset + 1) % self.text.len();
            self.last_tick = Instant::now();
        }
    }

    /// Get the currently visible slice (with wrapping)
    pub fn visible_text(&mut self) -> String {
        self._update();
        let len = self.text.len();
        if self.text == "Nothing   ".to_string() {
            return "Nothing".to_string();
        }
        if len-3 <= self.width {
            return self.text.clone();
        }
        if self.offset + self.width <= len {
            self.text[self.offset..self.offset + self.width].to_string()
        } else {
            let end_part = &self.text[self.offset..];
            let start_part = &self.text[..(self.width - end_part.len())];
            format!("{}{}", end_part, start_part)
        }
    }
}
