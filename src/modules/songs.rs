use rand::prelude::*;
use super::utils::{artist_data, change_artist};
use mp3_duration;
use std::time::Duration;
use std::path::Path;

#[derive(Clone)]
pub struct Song {
    pub path: String,
    pub name: String,
    pub artist: String,
    pub searchable: String,
    pub original_index: usize,
    pub duration: Duration,
    pub current: bool,
}

pub struct Songs {
    pub all_songs: Vec<Song>,
    pub filtered_songs: Vec<Song>,
    pub current_index: usize,
    pub stophandler: bool,
    pub shuffle: bool,
    pub typical_page_size: usize,
    pub blacklist: Vec<usize>,
}

#[inline]
pub fn absolute_index(index: usize, page: usize, typical_page_size: usize) -> usize {
    index + ((page - 1) * typical_page_size)
}

impl Songs {
    pub fn constructor(paths: Vec<String>) -> Self {
        let mut all_songs = Vec::new();
        for (i, path) in paths.iter().enumerate() {
            let artist = artist_data(path);
            let name = path
                .replace("music/", "")
                .replace("music\\", "")
                .replace(".mp3", "");
            let searchable = format!("{} {}", name.to_lowercase(), artist.to_lowercase());
            let duration = mp3_duration::from_path(Path::new(path)) // optimization can be done here because loading the constructor can get slow with many songs
                .unwrap_or(Duration::from_secs(0));

            all_songs.push(Song {
                path: path.clone(),
                name,
                artist,
                searchable,
                original_index: i,
                duration,
                current: false,
            });
        }

        Self {
            filtered_songs: all_songs.clone(),
            all_songs,
            current_index: usize::MAX,
            stophandler: true,
            shuffle: false,
            typical_page_size: 14,
            blacklist: Vec::new(),
        }
    }
    pub fn original_song_path(&self, original_index: usize) -> String {
        if self.stophandler {
            return "Nothing".to_string();
        }
        self.all_songs.get(original_index).map(|s| s.path.clone()).unwrap_or_default()
    }
    pub fn current_song_path(&self) -> String {
        if self.stophandler {
            return "Nothing".to_string();
        }

        for song in &self.all_songs { // search is now useless ig because current_index works properly, i'll leave it for now
            if song.current {
                return song.path.clone();
            }
        }
        "Nothing".to_string()
    }
    pub fn get_original_index(&self, index_in_filtered: usize) -> usize { // im not sure these work xd
        if index_in_filtered >= self.filtered_songs.len() {
            return usize::MAX;
        }
        self.filtered_songs[index_in_filtered].original_index
    }
    pub fn get_filtered_index(&self, original_index: usize) -> usize { // im not sure these work xd
        if original_index == usize::MAX {
            return usize::MAX;
        }
        for (i, song) in self.filtered_songs.iter().enumerate() {
            if song.original_index == original_index {
                return i;
            }
        }
        usize::MAX
    }
    pub fn match_c(&self) -> usize { // wtf does that even do? idk but i wrote it and it works
        for i in 0..self.filtered_songs.len() {
            if self.filtered_songs[i].original_index == self.current_index {
                return i;
            }
        }
        usize::MAX
    }


    pub fn _get_artist(&self, index: usize) -> String {
        if self.stophandler {
            return "Nothing".to_string();
        }
        self.all_songs.get(index).map(|s| s.artist.clone()).unwrap_or("Nothing".to_string())
    }
    pub fn get_artist_search(&self) -> String {
        if self.stophandler {
            return "Nothing".to_string();
        }
        for song in &self.all_songs {
            if song.current {
                return song.artist.clone();
            }
        }
        "Nothing".to_string()
    }

    pub fn set_force(&mut self, original_index: usize) {
        if original_index >= self.all_songs.len() {
            return;
        }
        self.current_index = original_index;
        self.stophandler = false;
        self.renew_current_status(original_index);
    }

    pub fn set_artist(&mut self, index: usize, artist: String) {
        if self.stophandler || index >= self.filtered_songs.len() {
            return;
        }
        match change_artist(&self.filtered_songs[index].path, &artist) {
            Ok(_) => {
                self.all_songs[self.filtered_songs[index].original_index].artist = artist.clone();
                self.filtered_songs[index].artist = artist.clone();
            },
            Err(_) => {}
        }
    }

    pub fn search(&mut self, pattern: String) {
        if pattern == "false" || pattern.is_empty() {
            self.filtered_songs = self.all_songs.clone();
            return;
        }

        let pattern = pattern.to_lowercase();
        self.filtered_songs = self
            .all_songs
            .iter()
            .filter(|s| s.searchable.contains(&pattern))
            .cloned()
            .collect();
    }

    pub fn blacklist(&mut self, index_in_filtered: usize) {
        if index_in_filtered >= self.filtered_songs.len() {
            return;
        }
        let original_index = self.filtered_songs[index_in_filtered].original_index;
        if let Some(pos) = self.blacklist.iter().position(|&x| x == original_index) {
            self.blacklist.remove(pos);
        } else {
            self.blacklist.push(original_index);
        }
    }

    pub fn is_blacklist(&self, index_in_filtered: usize) -> bool {
        if index_in_filtered >= self.filtered_songs.len() {
            return false;
        }
        let original_index = self.filtered_songs[index_in_filtered].original_index;
        self.blacklist.contains(&original_index)
    }

    pub fn _all_songs(&self) -> Vec<String> {
        self.filtered_songs.iter().map(|s| s.path.clone()).collect()
    }

    pub fn _current_index(&self) -> usize {
        if self.stophandler {
            usize::MAX
        } else {
            self.current_index
        }
    }

    pub fn current_name(&self) -> String {
        if self.stophandler {
            return "Nothing".to_string();
        }

        for song in &self.all_songs {
            if song.current {
                return song.name.clone();
            }
        }
        "Nothing".to_string()
    }

    pub fn renew_current_status(&mut self, original_index: usize) {
        for song in &mut self.all_songs {
            song.current = false;
        }
        if let Some(song) = self.all_songs.get_mut(original_index) {
            song.current = true;
        }
        let filtered_index = self.get_filtered_index(original_index);
        for song in &mut self.filtered_songs {
            song.current = false;
        }
        if let Some(song) = self.filtered_songs.get_mut(filtered_index) {
            song.current = true;
        }
    }
    pub fn set_by_pindex(&mut self, index: usize, page: usize) -> Result<(), u8> {
        let absolute = absolute_index(index, page, self.typical_page_size);
        if absolute >= self.filtered_songs.len() {
            return Err(1);
        }

        let original_index = self.filtered_songs[absolute].original_index;
        if self.blacklist.contains(&original_index) {
            return Err(0);
        }
        self.renew_current_status(original_index);
        self.current_index = original_index;
        self.stophandler = false;
        Ok(())
    }

    pub fn set_by_next(&mut self) -> Result<usize, ()> {
        if self.filtered_songs.is_empty() || self.stophandler {
            return Err(()); // nothing to play
        }

        let last_index = self.filtered_songs.len() - 1;

        // 🔀 SHUFFLE MODE
        if self.shuffle {
            let mut rng = rand::rng();
            for _ in 0..=last_index {
                let candidate = (0..=last_index).choose(&mut rng).unwrap();
                let original_index = self.filtered_songs[candidate].original_index;
                if !self.blacklist.contains(&original_index) {
                    self.current_index = original_index;
                    self.stophandler = false;
                    self.renew_current_status(original_index);
                    return Ok(candidate);
                }
            }
            return Err(());
        }

        // ▶️ SEQUENTIAL MODE
        let mut try_index = self.get_filtered_index(self.current_index) + 1;
        while try_index <= last_index {
            let original_index = self.filtered_songs[try_index].original_index;
            if !self.blacklist.contains(&original_index) {
                self.current_index = original_index;
                self.stophandler = false;
                self.renew_current_status(original_index);
                return Ok(try_index);
            }
            try_index += 1;
        }

        try_index = 0;
        while try_index < self.get_filtered_index(self.current_index) {
            let original_index = self.filtered_songs[try_index].original_index;
            if !self.blacklist.contains(&original_index) {
                self.current_index = original_index;
                self.stophandler = false;
                self.renew_current_status(original_index);
                return Ok(try_index);
            }
            try_index += 1;
        }

        Err(())
    }

    pub fn stop(&mut self) {
        self.stophandler = true;
    }

    pub fn shuffle(&mut self) {
        self.shuffle = !self.shuffle;
    }
}
