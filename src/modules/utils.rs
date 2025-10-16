use id3::TagLike;

pub struct Volume {
    pub steps: u8,
    pub step_div: u8,
}
impl Volume {
    pub fn step_up(&mut self) {
        if self.steps < 100 {
            self.steps += self.step_div;
        }
    }
    pub fn step_down(&mut self) {
        if self.steps != 0 {
            self.steps -= self.step_div;
        }
    }
    pub fn as_f64(&self) -> f64 {
        self.steps as f64 / 100 as f64
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
