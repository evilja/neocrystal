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
}

pub struct Songs {
    pub all_songs: Vec<Song>,
    pub filtered_songs: Vec<Song>,
    pub current_index: usize,
    pub stophandler: bool,
    pub shuffle: bool,
    pub typical_page_size: usize,
    pub blacklist: Vec<usize>,
    pub setnext: usize,
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
            setnext: usize::MAX,
        }
    }
    pub fn _original_song_path(&self, original_index: usize) -> String {
        if self.stophandler {
            return "Nothing".to_string();
        }
        self.all_songs.get(original_index).map(|s| s.path.clone()).unwrap_or_default()
    }
    pub fn current_song_path(&self) -> String {
        if self.stophandler {
            return "Nothing".to_string();
        }

        return self.all_songs.get(self.current_index).map(|s| s.path.clone()).unwrap_or("Nothing".to_string());
        
    }
    pub fn set_next(&mut self, original_index: usize) {
        self.setnext = original_index;
    }
    pub fn get_next(&self) -> usize {
        self.setnext
    }
    pub fn _get_original_index(&self, index_in_filtered: usize) -> usize { // im not sure these work xd
        if index_in_filtered >= self.filtered_songs.len() {
            return usize::MAX;
        }
        self.filtered_songs[index_in_filtered].original_index
    }
    pub fn get_filtered_index(&self, original_index: usize) -> Result<usize, ()> { // im not sure these work xd
        if original_index == usize::MAX {
            return Err(());
        }
        for (i, song) in self.filtered_songs.iter().enumerate() {
            if song.original_index == original_index {
                return Ok(i);
            }
        }
        Err(())
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
        self.all_songs.get(index).map(|s| s.artist.clone()).unwrap_or("Unknown".to_string())
    }
    pub fn get_artist_search(&self) -> String {
        return self.all_songs.get(self.current_index).map(|s| s.artist.clone()).unwrap_or("Unknown".to_string());
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
            self.setnext = self._algorithm_setnext().unwrap_or(usize::MAX);
            return;
        }

        let pattern = pattern.to_lowercase();
        self.filtered_songs = self
            .all_songs
            .iter()
            .filter(|s| s.searchable.contains(&pattern))
            .cloned()
            .collect();
        self.setnext = self._algorithm_setnext().unwrap_or(usize::MAX);
    }

    pub fn blacklist(&mut self, index_in_filtered: usize) {
        if index_in_filtered >= self.filtered_songs.len(){
            return;
        }
        let original_index = self.filtered_songs[index_in_filtered].original_index;
        if original_index == self.current_index {
            return;
        }
        if let Some(pos) = self.blacklist.iter().position(|&x| x == original_index) {
            self.blacklist.remove(pos);
            if !self.shuffle && self.setnext > original_index && self.setnext != usize::MAX {
                self.setnext = self._algorithm_setnext().unwrap_or(usize::MAX);
            }
        } else {
            self.blacklist.push(original_index);
            if original_index == self.setnext {
                self.setnext = self._algorithm_setnext().unwrap_or(usize::MAX);
            }
        }
    }

    pub fn is_blacklist(&self, original_index: usize) -> bool {
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
        return self.all_songs.get(self.current_index).map(|s| s.name.clone()).unwrap_or("Nothing".to_string());
    }

    pub fn renew_current_status(&mut self, original_index: usize) {
        self.current_index = original_index;
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
        self.stophandler = false;
        self.setnext = self._algorithm_setnext().unwrap_or(usize::MAX);
        Ok(())
    }
    pub fn get_duration(&self) -> Duration {
        if self.stophandler {
            return Duration::from_secs(0);
        }
        self.all_songs.get(self.current_index).map(|s| s.duration).unwrap_or(Duration::from_secs(0))
    }


    pub fn _algorithm_setnext(&mut self) -> Result<usize, ()> {
        if self.filtered_songs.is_empty() || self.stophandler {
            return Err(()); // nothing to play
        }

        // If only 1 entry, return it (unless blacklisted)
        if self.filtered_songs.len() == 1 {
            let original_index = self.filtered_songs[0].original_index;
            if self.blacklist.contains(&original_index) {
                return Err(());
            } else {
                return Ok(original_index);
            }
        }

        // 🔀 SHUFFLE MODE
        if self.shuffle {
            let mut rng = rand::rng();
            // collect candidates once
            let candidate_list = self
                .filtered_songs
                .iter()
                .map(|s| s.original_index)
                .filter(|orig| !self.blacklist.contains(orig) && *orig != self.current_index)
                .collect::<Vec<_>>();

            if candidate_list.is_empty() {
                return Err(());
            }

            // pick randomly from candidate_list
            let idx = rng.random_range(0..candidate_list.len());
            return Ok(candidate_list[idx]);
        }

        // ▶️ SEQUENTIAL MODE (safe)
        if let Ok(start) = self.get_filtered_index(self.current_index) {
            // try after current
            for i in (start + 1)..self.filtered_songs.len() {
                let original_index = self.filtered_songs[i].original_index;
                if !self.blacklist.contains(&original_index) {
                    return Ok(original_index);
                }
            }
            // wrap-around: check from 0 up to start
            for i in 0..=start {
                let original_index = self.filtered_songs[i].original_index;
                if !self.blacklist.contains(&original_index) {
                    return Ok(original_index);
                }
            }
        } else {
            // current not found (or current unset) -> just search from start
            for i in 0..self.filtered_songs.len() {
                let original_index = self.filtered_songs[i].original_index;
                if !self.blacklist.contains(&original_index) && original_index != self.current_index {
                    return Ok(original_index);
                }
            }
        }

        Err(())
    }


    pub fn set_by_next(&mut self) -> Result<usize, ()> {
        if self.setnext == usize::MAX {
            Err(())
        } else {
            self.renew_current_status(self.setnext);
            self.setnext = self._algorithm_setnext().unwrap_or(usize::MAX);
            Ok(self.current_index)
        }
    }

    pub fn stop(&mut self) {
        self.stophandler = true;
        self.setnext = usize::MAX;
    }

    pub fn shuffle(&mut self) {
        self.shuffle = !self.shuffle;
        self.setnext = self._algorithm_setnext().unwrap_or(usize::MAX);
    }
}
