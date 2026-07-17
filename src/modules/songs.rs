use crate::modules::preferences::{BASE_SCORE, Preferences};
use crate::modules::shuffle::plan;
use crate::modules::utils::{addto_album, album_data};

use super::audio::audio_duration;
use super::utils::{artist_data, change_artist};
use rand::rng;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
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
}

pub struct Songs {
    pub all_songs: Vec<Song>,
    /// Stable, alphabetically sorted membership for the UI.
    pub filtered_songs: Vec<usize>,
    pub current_index: usize,
    pub stophandler: bool,
    pub shuffle: bool,
    pub typical_page_size: usize,
    pub blacklist: Vec<usize>,
    /// Cached automatic/forced next value used by the UI indicator.
    pub setnext: usize,
    pub preferences: Preferences,
    shuffle_pending: Vec<usize>,
    shuffle_consumed: HashSet<usize>,
    forced_next: Option<usize>,
    first_loop_bias_available: bool,
}

#[inline]
pub fn absolute_index(index: usize, page: usize, typical_page_size: usize) -> usize {
    index + ((page - 1) * typical_page_size)
}

impl Songs {
    pub fn constructor(paths: Vec<String>) -> Self {
        Self::constructor_with_preferences_path(paths, super::preferences::default_database_path())
    }

    pub fn constructor_with_preferences_path<P: Into<PathBuf>>(
        paths: Vec<String>,
        preference_path: P,
    ) -> Self {
        let mut all_songs = Vec::with_capacity(paths.len());
        let mut durations = vec![Duration::default(); paths.len()];
        let mut handles = Vec::new();

        for (i, path) in paths.iter().enumerate() {
            if !Path::new(path).exists() {
                continue;
            }
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
            let name = Path::new(path)
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
            });
        }

        all_songs.sort_by(|a, b| (&a.artist, &a.name).cmp(&(&b.artist, &b.name)));
        let library_paths: Vec<String> = all_songs.iter().map(|song| song.path.clone()).collect();
        let preferences = Preferences::with_path(&library_paths, preference_path);
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
            preferences,
            shuffle_pending: Vec::new(),
            shuffle_consumed: HashSet::new(),
            forced_next: None,
            first_loop_bias_available: true,
        }
    }

    fn reset_next_track(&mut self) {
        self.setnext = if let Some(forced) = self.valid_forced_next() {
            forced
        } else {
            self.forced_next = None;
            self.next_candidate().unwrap_or(usize::MAX)
        };
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

    fn original_index_at_filtered(&self, index_in_filtered: usize) -> Option<usize> {
        self.filtered_songs.get(index_in_filtered).copied()
    }

    fn current_filtered_index(&self) -> Option<usize> {
        self.filtered_songs
            .iter()
            .position(|&index| index == self.current_index)
    }

    fn reset_filter(&mut self) {
        self.filtered_songs = (0..self.all_songs.len()).collect();
    }

    fn is_blacklisted(&self, original_index: usize) -> bool {
        self.blacklist.contains(&original_index)
    }

    pub fn get_ordered(&self) -> Vec<usize> {
        self.filtered_songs.clone()
    }

    #[allow(dead_code)]
    pub fn get_unordered(&self) -> &Vec<usize> {
        &self.filtered_songs
    }

    pub fn shuffle(&mut self) {
        self.shuffle = !self.shuffle;
        if self.shuffle {
            self.shuffle_pending.clear();
            self.shuffle_consumed.clear();
            self.generate_shuffle_cycle(true);
            if let Some(forced) = self.valid_forced_next() {
                self.remove_from_pending(forced);
            }
        } else {
            self.shuffle_pending.clear();
            self.shuffle_consumed.clear();
        }
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
            if self.setnext == usize::MAX && self.current_index != usize::MAX {
                return 1;
            }
            0
        } else {
            2
        }
    }

    pub fn current_song_path(&self) -> String {
        self.current_song()
            .map(|s| s.path.clone())
            .unwrap_or_else(|| NOTHING.to_string())
    }

    /// Schedule one explicit next song.  `index_in_filtered` is a UI position,
    /// not an original index into `all_songs`.
    pub fn set_next(&mut self, index_in_filtered: usize) {
        let Some(original_index) = self.original_index_at_filtered(index_in_filtered) else {
            return;
        };
        if self.is_blacklisted(original_index) {
            return;
        }

        self.forced_next = Some(original_index);
        self.remove_from_pending(original_index);
        self.reset_next_track();
    }

    pub fn get_next(&self) -> usize {
        self.setnext
    }

    pub fn match_c(&self) -> usize {
        self.current_filtered_index().unwrap_or(usize::MAX)
    }

    pub fn set_artist(&mut self, index_in_filtered: usize, artist: &str) {
        if self.stophandler {
            return;
        }

        let Some(idx) = self.original_index_at_filtered(index_in_filtered) else {
            return;
        };

        if change_artist(&self.all_songs[idx].path, artist).is_ok() {
            self.all_songs[idx].artist = artist.to_owned();
            Self::rebuild_searchable(&mut self.all_songs[idx]);
        }
    }

    pub fn set_playlist(&mut self, index_in_filtered: usize, playlist: &str) {
        if self.stophandler {
            return;
        }

        let Some(idx) = self.original_index_at_filtered(index_in_filtered) else {
            return;
        };

        if addto_album(&self.all_songs[idx].path, playlist).is_ok() {
            self.all_songs[idx].playlist = playlist.to_owned();
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
                .filter(|(_, song)| song.searchable.contains(&pattern))
                .map(|(index, _)| index)
                .collect();
        }

        self.rebuild_shuffle_schedule();
        self.reset_next_track();
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
        } else {
            self.blacklist.push(original_index);
            self.remove_from_pending(original_index);
            if self.forced_next == Some(original_index) {
                self.forced_next = None;
            }
        }
        self.rebuild_shuffle_schedule();
        self.reset_next_track();
    }

    pub fn is_blacklist(&self, original_index: usize) -> bool {
        self.is_blacklisted(original_index)
    }

    pub fn current_name(&self) -> String {
        self.current_song()
            .map(|s| s.name.clone())
            .unwrap_or_else(|| NOTHING.to_string())
    }

    pub fn set_by_pindex(&mut self, index: usize, page: usize) -> Result<(), u8> {
        let absolute = absolute_index(index, page, self.typical_page_size);
        let Some(original_index) = self.original_index_at_filtered(absolute) else {
            return Err(1);
        };
        if self.is_blacklisted(original_index) {
            return Err(0);
        }

        self.direct_selection(original_index)
            .map(|_| ())
            .map_err(|_| 1)
    }

    pub fn get_duration(&self) -> Duration {
        self.current_song().map(|s| s.duration).unwrap_or_default()
    }

    fn eligible_entries(&self) -> Vec<(usize, u8)> {
        self.filtered_songs
            .iter()
            .copied()
            .filter(|&index| !self.is_blacklisted(index))
            .map(|index| (index, self.preferences.score(&self.all_songs[index].path)))
            .collect()
    }

    fn valid_forced_next(&self) -> Option<usize> {
        self.forced_next
            .filter(|&index| self.filtered_songs.contains(&index) && !self.is_blacklisted(index))
    }

    fn remove_from_pending(&mut self, original_index: usize) {
        self.shuffle_pending
            .retain(|&index| index != original_index);
        self.shuffle_consumed.insert(original_index);
    }

    fn generate_shuffle_cycle(&mut self, consume_current: bool) {
        let entries = self.eligible_entries();
        if entries.is_empty() {
            self.shuffle_pending.clear();
            return;
        }

        let current = if !self.stophandler {
            Some(self.current_index)
        } else {
            None
        };
        let mut random = rng();
        let order = plan(
            &entries,
            self.first_loop_bias_available,
            current,
            &mut random,
        );
        self.first_loop_bias_available = false;
        self.shuffle_pending = order;
        self.shuffle_consumed.clear();

        if consume_current && !self.stophandler {
            self.remove_from_pending(self.current_index);
        }
    }

    fn rebuild_shuffle_schedule(&mut self) {
        if !self.shuffle {
            self.shuffle_pending.clear();
            self.shuffle_consumed.clear();
            return;
        }

        let entries: Vec<_> = self
            .eligible_entries()
            .into_iter()
            .filter(|(index, _)| !self.shuffle_consumed.contains(index))
            .collect();
        if entries.is_empty() {
            self.shuffle_pending.clear();
            return;
        }

        let mut random = rng();
        self.shuffle_pending = plan(
            &entries,
            false,
            (!self.stophandler).then_some(self.current_index),
            &mut random,
        );
    }

    fn next_candidate(&self) -> Option<usize> {
        if let Some(forced) = self.valid_forced_next() {
            return Some(forced);
        }

        if self.shuffle {
            return self.shuffle_pending.first().copied();
        }
        self.sequential_candidate(1)
    }

    fn sequential_candidate(&self, direction: isize) -> Option<usize> {
        let eligible: Vec<usize> = self
            .filtered_songs
            .iter()
            .copied()
            .filter(|&index| !self.is_blacklisted(index))
            .collect();
        if eligible.is_empty() {
            return None;
        }

        let Some(current_position) = self.current_filtered_index() else {
            return eligible.first().copied();
        };
        let len = self.filtered_songs.len();
        if len == 0 {
            return None;
        }

        for offset in 1..=len {
            let offset = offset as isize * direction;
            let position = (current_position as isize + offset).rem_euclid(len as isize) as usize;
            let candidate = self.filtered_songs[position];
            if !self.is_blacklisted(candidate) {
                return Some(candidate);
            }
        }
        None
    }

    fn take_next_target(&mut self) -> Option<usize> {
        if let Some(forced) = self.valid_forced_next() {
            self.forced_next = None;
            self.remove_from_pending(forced);
            return Some(forced);
        }

        if self.shuffle {
            if self.shuffle_pending.is_empty() {
                self.generate_shuffle_cycle(false);
            }
            let target = self.shuffle_pending.first().copied()?;
            self.shuffle_pending.remove(0);
            self.shuffle_consumed.insert(target);
            return Some(target);
        }

        self.sequential_candidate(1)
    }

    fn take_previous_target(&mut self) -> Option<usize> {
        let target = self.sequential_candidate(-1)?;
        if self.shuffle {
            self.remove_from_pending(target);
        }
        Some(target)
    }

    fn start_song(&mut self, original_index: usize, consume_shuffle_entry: bool) {
        self.current_index = original_index;
        self.stophandler = false;
        if consume_shuffle_entry {
            self.remove_from_pending(original_index);
        }
        self.preferences
            .mark_playback_started(&self.all_songs[original_index].path);
        self.reset_next_track();
    }

    /// Play a song selected explicitly by the user.
    pub fn direct_selection(&mut self, original_index: usize) -> Result<usize, ()> {
        if !self.filtered_songs.contains(&original_index)
            || self.is_blacklisted(original_index)
            || original_index >= self.all_songs.len()
        {
            return Err(());
        }

        let old = (!self.stophandler).then_some(self.current_index);
        self.forced_next = None;
        self.remove_from_pending(original_index);
        if let Some(old) = old.filter(|&old| old != original_index) {
            let old_path = self.all_songs[old].path.clone();
            self.preferences.record_skip(&old_path);
        }
        let path = self.all_songs[original_index].path.clone();
        self.preferences.record_direct_selection(&path);
        self.start_song(original_index, true);
        Ok(original_index)
    }

    /// Record a natural completion, then advance through the automatic order.
    pub fn completed_and_next(&mut self) -> Result<usize, ()> {
        if self.stophandler || self.current_index >= self.all_songs.len() {
            return Err(());
        }
        let target = self.take_next_target().ok_or(())?;
        let old_path = self.all_songs[self.current_index].path.clone();
        self.preferences.record_completion(&old_path);
        self.start_song(target, true);
        Ok(target)
    }

    /// Advance because the user pressed Next.
    pub fn skipped_and_next(&mut self) -> Result<usize, ()> {
        if self.stophandler || self.current_index >= self.all_songs.len() {
            return Err(());
        }
        let target = self.take_next_target().ok_or(())?;
        let old = self.current_index;
        if old != target {
            let old_path = self.all_songs[old].path.clone();
            self.preferences.record_skip(&old_path);
        }
        self.start_song(target, true);
        Ok(target)
    }

    /// Move to the previous sorted/filtered song as an explicit skip.
    pub fn skipped_and_previous(&mut self) -> Result<usize, ()> {
        if self.stophandler || self.current_index >= self.all_songs.len() {
            return Err(());
        }
        let target = self.take_previous_target().ok_or(())?;
        let old = self.current_index;
        if old != target {
            let old_path = self.all_songs[old].path.clone();
            self.preferences.record_skip(&old_path);
        }
        self.start_song(target, true);
        Ok(target)
    }

    /// Repeat the current song without consuming another shuffle entry.
    pub fn completed_with_repeat(&mut self) -> Result<usize, ()> {
        if self.stophandler || self.current_index >= self.all_songs.len() {
            return Err(());
        }
        let current = self.current_index;
        let path = self.all_songs[current].path.clone();
        self.preferences.record_completion(&path);
        self.start_song(current, false);
        Ok(current)
    }

    // Compatibility wrappers for callers outside the event loop.
    #[allow(dead_code)]
    pub fn set_by_next(&mut self) -> Result<usize, ()> {
        self.skipped_and_next()
    }

    #[allow(dead_code)]
    pub fn prev(&mut self) -> Result<usize, ()> {
        self.skipped_and_previous()
    }

    pub fn resume(&mut self) {
        self.stophandler = false;
        self.reset_next_track();
    }

    pub fn stop(&mut self) {
        self.stophandler = true;
        self.setnext = usize::MAX;
    }

    #[allow(dead_code)]
    pub fn score(&self, original_index: usize) -> u8 {
        self.all_songs
            .get(original_index)
            .map(|song| self.preferences.score(&song.path))
            .unwrap_or(BASE_SCORE)
    }

    pub fn finish_session(&mut self) {
        let paths: Vec<String> = self
            .all_songs
            .iter()
            .map(|song| song.path.clone())
            .collect();
        self.preferences.finalize_session(&paths);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture(name: &str, count: usize) -> Songs {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!("neocrystal-songs-{name}-{stamp}.json"));
        let songs = Songs::constructor_with_preferences_path(
            (0..count)
                .map(|index| format!("/music/{index:03}.mp3"))
                .collect(),
            &path,
        );
        let _ = fs::remove_file(path);
        songs
    }

    #[test]
    fn direct_selection_rewards_and_reselecting_does_not_skip() {
        let mut songs = fixture("selection", 3);
        songs.direct_selection(0).unwrap();
        assert_eq!(songs.score(0), 101);
        songs.direct_selection(0).unwrap();
        assert_eq!(songs.score(0), 102);
        songs.direct_selection(1).unwrap();
        assert_eq!(songs.score(0), 101);
        assert_eq!(songs.score(1), 101);
    }

    #[test]
    fn sequential_transitions_and_filtering_are_safe() {
        let mut songs = fixture("sequential", 3);
        songs.direct_selection(1).unwrap();
        assert_eq!(songs.skipped_and_next().unwrap(), 2);
        assert_eq!(songs.skipped_and_previous().unwrap(), 1);
        songs.search(&"does-not-exist".to_string());
        assert!(songs.skipped_and_next().is_err());
        songs.search(&"false".to_string());
        songs.blacklist(2);
        assert!(!songs.is_blacklist(1));
    }

    #[test]
    fn shuffle_consumes_a_cycle_without_duplicates() {
        let mut songs = fixture("shuffle", 8);
        songs.shuffle();
        songs.direct_selection(0).unwrap();
        let mut played = Vec::new();
        for _ in 0..7 {
            let next = songs.skipped_and_next().unwrap();
            played.push(next);
        }
        played.push(songs.current_index);
        played.push(0);
        played.sort_unstable();
        played.dedup();
        assert_eq!(played.len(), 8);
        assert!(!songs.shuffle_pending.contains(&0));
    }

    #[test]
    fn forced_next_uses_filtered_position_and_is_consumed() {
        let mut songs = fixture("forced", 4);
        songs.direct_selection(0).unwrap();
        songs.set_next(2);
        assert_eq!(songs.get_next(), 2);
        songs.shuffle();
        songs.set_next(1);
        assert_eq!(songs.skipped_and_next().unwrap(), 1);
        assert!(!songs.shuffle_pending.contains(&1));
    }
}
