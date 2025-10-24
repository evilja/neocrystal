use rand::prelude::*;
use super::utils::{artist_data, change_artist};
use mp3_duration;
use std::time::Duration;
use std::path::Path;
use std::thread::spawn;


#[derive(Clone)]
pub struct Song {
    pub path: String,
    pub name: String,
    pub artist: String,
    pub searchable: String,
    pub duration: Duration,
    pub forced: bool,
}

pub struct Songs {
    pub all_songs: Vec<Song>,
    pub filtered_songs: Vec<usize>,
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
        let mut all_songs = Vec::with_capacity(paths.len());
        let mut durations = vec![Duration::from_secs(0); paths.len()];
        let mut handles = Vec::new();

        for (i, path) in paths.iter().enumerate() {
            let path_clone = path.clone();
            let handle = spawn(move || {
                mp3_duration::from_path(Path::new(&path_clone))
                    .unwrap_or(Duration::from_secs(0))
            });
            handles.push((i, handle));
        }

        for (i, handle) in handles {
            if let Ok(duration) = handle.join() {
                durations[i] = duration;
            }
        }

        for (i, path) in paths.iter().enumerate() {
            let artist = artist_data(path);
            let name = path
                .replace("music/", "")
                .replace("music\\", "")
                .replace(".mp3", "");
            let searchable = format!("{} {}", name.to_lowercase(), artist.to_lowercase());

            all_songs.push(Song {
                path: path.clone(),
                name,
                artist,
                searchable,
                duration: durations[i],
                forced: false,
            });
        }

        let filtered_songs = (0..all_songs.len()).collect::<Vec<_>>();

        Self {
            all_songs,
            filtered_songs,
            current_index: usize::MAX,
            stophandler: true,
            shuffle: false,
            typical_page_size: 14,
            blacklist: Vec::new(),
            setnext: usize::MAX,
        }
    }

    pub fn current_song_path(&self) -> String {
        if self.stophandler {
            return "Nothing".to_string();
        }

        self.all_songs
            .get(self.current_index)
            .map(|s| s.path.clone())
            .unwrap_or("Nothing".to_string())
    }

    pub fn set_next(&mut self, original_index: usize) {
        self.setnext = original_index;
        self.all_songs[original_index].forced = true;
    }

    pub fn get_next(&self) -> usize {
        self.setnext
    }

    pub fn get_filtered_index(&self, original_index: usize) -> Result<usize, ()> {
        self.filtered_songs
            .iter()
            .position(|&i| i == original_index)
            .ok_or(())
    }

    pub fn match_c(&self) -> usize {
        self.filtered_songs
            .iter()
            .position(|&i| i == self.current_index)
            .unwrap_or(usize::MAX)
    }

    pub fn get_artist_search(&self) -> String {
        self.all_songs
            .get(self.current_index)
            .map(|s| s.artist.clone())
            .unwrap_or("Unknown".to_string())
    }

    pub fn set_artist(&mut self, index_in_filtered: usize, artist: &String) {
        if self.stophandler || index_in_filtered >= self.filtered_songs.len() {
            return;
        }
        let idx = self.filtered_songs[index_in_filtered];
        if change_artist(&self.all_songs[idx].path, artist).is_ok() {
            self.all_songs[idx].artist = artist.clone();
        }
    }

    pub fn search(&mut self, pattern: &String) {
        if pattern == "false" || pattern.is_empty() {
            self.filtered_songs = (0..self.all_songs.len()).collect();
            self.setnext = self.algorithm_setnext().unwrap_or(usize::MAX);
            return;
        }

        let pattern = pattern.to_lowercase();
        self.filtered_songs = self
            .all_songs
            .iter()
            .enumerate()
            .filter(|(_, s)| s.searchable.contains(&pattern))
            .map(|(i, _)| i)
            .collect();

        self.setnext = self.algorithm_setnext().unwrap_or(usize::MAX);
    }

    pub fn blacklist(&mut self, index_in_filtered: usize) {
        if index_in_filtered >= self.filtered_songs.len() {
            return;
        }

        let original_index = self.filtered_songs[index_in_filtered];

        if original_index == self.current_index {
            return;
        }

        if let Some(pos) = self.blacklist.iter().position(|&x| x == original_index) {
            self.blacklist.remove(pos);
            if !self.shuffle && self.setnext > original_index && self.setnext != usize::MAX {
                self.setnext = self.algorithm_setnext().unwrap_or(usize::MAX);
            }
        } else {
            self.blacklist.push(original_index);
            if original_index == self.setnext {
                if self.setnext != usize::MAX { self.all_songs[self.setnext].forced = false; }
                self.setnext = self.algorithm_setnext().unwrap_or(usize::MAX);
            }
        }
    }

    pub fn is_blacklist(&self, original_index: usize) -> bool {
        self.blacklist.contains(&original_index)
    }

    pub fn current_name(&self) -> String {
        self.all_songs
            .get(self.current_index)
            .map(|s| s.name.clone())
            .unwrap_or("Nothing".to_string())
    }

    fn renew_current_status(&mut self, original_index: usize) {
        self.current_index = original_index;
        self.all_songs[original_index].forced = false;
    }

    pub fn set_by_pindex(&mut self, index: usize, page: usize) -> Result<(), u8> {
        let absolute = absolute_index(index, page, self.typical_page_size);
        if absolute >= self.filtered_songs.len() {
            return Err(1);
        }

        let original_index = self.filtered_songs[absolute];
        if self.blacklist.contains(&original_index) {
            return Err(0);
        }
        self.renew_current_status(original_index);
        self.stophandler = false;
        self.setnext = self.algorithm_setnext().unwrap_or(usize::MAX);
        Ok(())
    }

    pub fn get_duration(&self) -> Duration {
        if self.stophandler {
            return Duration::from_secs(0);
        }
        self.all_songs
            .get(self.current_index)
            .map(|s| s.duration)
            .unwrap_or(Duration::from_secs(0))
    }

    fn algorithm_setnext(&mut self) -> Result<usize, ()> {
        if self.filtered_songs.is_empty() || self.stophandler {
            return Err(());
        }
        if self.setnext != usize::MAX && self.all_songs[self.setnext].forced { return Ok(self.setnext) }
        if self.filtered_songs.len() == 1 {
            let original_index = self.filtered_songs[0];
            if self.blacklist.contains(&original_index) {
                return Err(());
            } else {
                return Ok(original_index);
            }
        }

        if self.shuffle {
            let mut rng = rand::rng();
            let candidate_list = self
                .filtered_songs
                .iter()
                .filter(|&&i| !self.blacklist.contains(&i) && i != self.current_index)
                .copied()
                .collect::<Vec<_>>();

            if candidate_list.is_empty() {
                return Err(());
            }

            let idx = rng.random_range(0..candidate_list.len());
            return Ok(candidate_list[idx]);
        }

        // sequential
        if let Ok(start) = self.get_filtered_index(self.current_index) {
            for &i in self.filtered_songs.iter().skip(start + 1) {
                if !self.blacklist.contains(&i) {
                    return Ok(i);
                }
            }
            for &i in self.filtered_songs.iter().take(start + 1) {
                if !self.blacklist.contains(&i) {
                    return Ok(i);
                }
            }
        } else {
            for &i in &self.filtered_songs {
                if !self.blacklist.contains(&i) && i != self.current_index {
                    return Ok(i);
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
            self.setnext = self.algorithm_setnext().unwrap_or(usize::MAX);
            Ok(self.current_index)
        }
    }

    pub fn stop(&mut self) {
        self.stophandler = true;
        //self.setnext = usize::MAX;
    }

    pub fn shuffle(&mut self) {
        self.shuffle = !self.shuffle;
        self.setnext = self.algorithm_setnext().unwrap_or(usize::MAX);
    }
}
