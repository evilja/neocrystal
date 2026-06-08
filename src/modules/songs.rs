use crate::modules::utils::{addto_album, album_data};
use rand::seq::SliceRandom;

use super::audio::audio_duration;
use super::utils::{artist_data, change_artist};
use std::path::Path;
use std::thread::spawn;
use std::time::Duration;

const NOTHING: &str = "Nothing";

#[derive(Clone)]
pub struct Song {
    pub path: String,
    pub name: String,
    pub artist: String,
    pub playlist: String,
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
        let mut durations = vec![Duration::default(); paths.len()];
        let mut handles = Vec::new();

        for (i, path) in paths.iter().enumerate() {
            let path_clone = path.clone();
            let handle = spawn(move || audio_duration(&path_clone));
            handles.push((i, handle));
        }

        for (i, handle) in handles {
            if let Ok(duration) = handle.join() {
                durations[i] = duration;
            }
        }

        for (i, path) in paths.iter().enumerate() {
            let artist = artist_data(path);
            let playlist = album_data(path);
            let name = Path::new(&path)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let searchable = format!(
                "{} {} {}",
                name.to_lowercase(),
                artist.to_lowercase(),
                playlist.to_lowercase()
            );

            all_songs.push(Song {
                path: path.clone(),
                name,
                artist,
                playlist,
                searchable,
                duration: durations[i],
                forced: false,
            });
        }

        all_songs.sort_by(|a, b| (&a.artist, &a.name).cmp(&(&b.artist, &b.name)));

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

    fn reset_next_track(&mut self) {
        self.setnext = self.algorithm_setnext().unwrap_or(usize::MAX);
    }

    fn rebuild_searchable(song: &mut Song) {
        song.searchable = format!(
            "{}{}{}",
            song.name.to_lowercase(),
            song.artist.to_lowercase(),
            song.playlist.to_lowercase()
        );
    }

    fn current_song(&self) -> Option<&Song> {
        if self.stophandler {
            return None;
        }

        self.all_songs.get(self.current_index)
    }

    fn ordered_filtered(&self) -> Vec<usize> {
        let mut ordered = self.filtered_songs.clone();
        ordered.sort();
        ordered
    }

    fn original_index_at_filtered(&self, index_in_filtered: usize) -> Option<usize> {
        self.ordered_filtered().get(index_in_filtered).copied()
    }

    fn current_filtered_index(&self) -> Option<usize> {
        self.ordered_filtered()
            .iter()
            .position(|&i| i == self.current_index)
    }

    fn reset_filter(&mut self) {
        self.filtered_songs = (0..self.all_songs.len()).collect();
    }

    fn is_blacklisted(&self, original_index: usize) -> bool {
        self.blacklist.contains(&original_index)
    }

    pub fn get_ordered(&self) -> Vec<usize> {
        self.ordered_filtered()
    }

    pub fn get_unordered(&self) -> &Vec<usize> {
        &self.filtered_songs
    }

    fn urandom(&mut self) {
        if self.shuffle {
            self.filtered_songs.shuffle(&mut rand::rng());
        } else {
            self.filtered_songs.sort();
        }
    }

    pub fn shuffle(&mut self) {
        self.shuffle = !self.shuffle;
        self.urandom();
        self.reset_next_track();
    }

    pub fn current_artist(&self) -> String {
        self.current_song()
            .map(|s| s.artist.clone())
            .unwrap_or_else(|| NOTHING.to_string())
    }

    pub fn current_playlist(&self) -> String {
        self.current_song()
            .map(|song| {
                if song.playlist.is_empty() {
                    " ".to_string()
                } else {
                    song.playlist.clone()
                }
            })
            .unwrap_or_else(|| " ".to_string())
    }

    pub fn _status(&self) -> u8 {
        if self.stophandler {
            // current index is initialized but setnext is 0 from stop() method
            if self.setnext == usize::MAX && !self.current_index == usize::MAX {
                return 1;
            }
            return 0;
        } else {
            return 2;
        }
    }

    pub fn current_song_path(&self) -> String {
        self.current_song()
            .map(|s| s.path.clone())
            .unwrap_or_else(|| NOTHING.to_string())
    }

    pub fn set_next(&mut self, original_index: usize) {
        if let Some(&idx) = self.filtered_songs.get(original_index) {
            self.setnext = idx;
            self.all_songs[self.setnext].forced = true;
        }
    }

    pub fn get_next(&self) -> usize {
        self.setnext
    }

    pub fn match_c(&self) -> usize {
        self.current_filtered_index().unwrap_or(usize::MAX)
    }

    pub fn set_artist(&mut self, index_in_filtered: usize, artist: &String) {
        if self.stophandler {
            return;
        }

        let Some(idx) = self.original_index_at_filtered(index_in_filtered) else {
            return;
        };

        if change_artist(&self.all_songs[idx].path, artist).is_ok() {
            self.all_songs[idx].artist = artist.clone();
            Self::rebuild_searchable(&mut self.all_songs[idx]);
        }
    }

    pub fn set_playlist(&mut self, index_in_filtered: usize, playlist: &String) {
        if self.stophandler {
            return;
        }

        let Some(idx) = self.original_index_at_filtered(index_in_filtered) else {
            return;
        };

        if addto_album(&self.all_songs[idx].path, playlist).is_ok() {
            self.all_songs[idx].playlist = playlist.clone();
            Self::rebuild_searchable(&mut self.all_songs[idx]);
        }
    }

    pub fn search(&mut self, pattern: &String) {
        if pattern == "false" || pattern.is_empty() {
            self.reset_filter();
        } else {
            let pattern = pattern.to_lowercase();
            self.filtered_songs = self
                .all_songs
                .iter()
                .enumerate()
                .filter(|(_, s)| s.searchable.contains(&pattern))
                .map(|(i, _)| i)
                .collect();
        }

        self.reset_next_track();
        self.urandom();
    }

    pub fn blacklist(&mut self, index_in_filtered: usize) {
        let Some(original_index) = self.original_index_at_filtered(index_in_filtered) else {
            return;
        };

        if original_index == self.current_index {
            return;
        }

        if let Some(pos) = self.blacklist.iter().position(|&x| x == original_index) {
            self.blacklist.remove(pos);
            if !self.shuffle && self.setnext > original_index && self.setnext != usize::MAX {
                self.reset_next_track();
            }
        } else {
            self.blacklist.push(original_index);
            if original_index == self.setnext {
                if self.setnext != usize::MAX {
                    self.all_songs[self.setnext].forced = false;
                }
                self.reset_next_track();
            }
        }
    }

    pub fn is_blacklist(&self, original_index: usize) -> bool {
        self.is_blacklisted(original_index)
    }

    pub fn current_name(&self) -> String {
        self.current_song()
            .map(|s| s.name.clone())
            .unwrap_or_else(|| NOTHING.to_string())
    }

    fn renew_current_status(&mut self, original_index: usize) {
        self.current_index = original_index;
        self.all_songs[original_index].forced = false;
    }

    pub fn set_by_pindex(&mut self, index: usize, page: usize) -> Result<(), u8> {
        let absolute = absolute_index(index, page, self.typical_page_size);

        let Some(original_index) = self.original_index_at_filtered(absolute) else {
            return Err(1);
        };

        if self.is_blacklisted(original_index) {
            return Err(0);
        }

        self.renew_current_status(original_index);
        self.stophandler = false;
        self.reset_next_track();
        Ok(())
    }

    pub fn get_duration(&self) -> Duration {
        self.current_song().map(|s| s.duration).unwrap_or_default()
    }

    fn algorithm_setnext(&mut self) -> Result<usize, ()> {
        if self.filtered_songs.is_empty() || self.stophandler {
            return Err(());
        }
        if self.setnext != usize::MAX && self.all_songs[self.setnext].forced {
            return Ok(self.setnext);
        }
        if self.filtered_songs.len() == 1 {
            let original_index = self.get_unordered()[0];
            if self.is_blacklisted(original_index) {
                return Err(());
            } else {
                return Ok(original_index);
            }
        }
        // sequential
        if let Some(start) = self.current_filtered_index() {
            for &i in &self.get_unordered()[start + 1..] {
                if !self.is_blacklisted(i) {
                    return Ok(i);
                }
            }
            self.urandom();
            for &i in self.get_unordered().iter().take(start) {
                if !self.is_blacklisted(i) {
                    return Ok(i);
                }
            }
        } else {
            self.urandom();
            // this shit should NEVER run but is there just in case
            for &i in &self.filtered_songs {
                if !self.is_blacklisted(i) && i != self.current_index {
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
            self.reset_next_track();
            Ok(self.current_index)
        }
    }

    pub fn prev(&mut self) -> Result<usize, ()> {
        if self.stophandler {
            return Err(());
        }

        let Some(start) = self.current_filtered_index() else {
            return Err(());
        };

        let unordered = self.get_unordered();
        let candidate = (0..start)
            .rev()
            .map(|i| unordered[i])
            .chain((start + 1..unordered.len()).rev().map(|i| unordered[i]))
            .find(|&i| !self.is_blacklisted(i));
        if let Some(i) = candidate {
            self.renew_current_status(i);
            self.reset_next_track();
            Ok(self.current_index)
        } else {
            Err(())
        }
    }

    pub fn resume(&mut self) {
        self.stophandler = false;
        self.reset_next_track();
    }
    pub fn stop(&mut self) {
        self.stophandler = true;
        self.setnext = usize::MAX;
    }
}
