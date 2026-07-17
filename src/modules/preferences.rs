use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub const BASE_SCORE: u8 = 100;
pub const MAX_SCORE: u8 = 255;
const MISSES_BEFORE_DECAY: u64 = 5;
const DATABASE_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PreferenceRecord {
    pub score: u8,
    pub missed_sessions: u64,
    #[serde(skip)]
    pub played_this_session: bool,
}

impl Default for PreferenceRecord {
    fn default() -> Self {
        Self {
            score: BASE_SCORE,
            missed_sessions: 0,
            played_this_session: false,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct PreferenceDocument {
    version: u32,
    songs: BTreeMap<String, PreferenceRecord>,
}

pub struct Preferences {
    path: PathBuf,
    records: BTreeMap<String, PreferenceRecord>,
}

impl Preferences {
    #[allow(dead_code)]
    pub fn new(library_paths: &[String]) -> Self {
        Self::with_path(library_paths, default_database_path())
    }

    pub fn with_path<P: Into<PathBuf>>(library_paths: &[String], path: P) -> Self {
        let path = path.into();
        let records = Self::load(&path);
        let mut preferences = Self { path, records };

        let mut changed = false;
        for song_path in library_paths {
            if !preferences.records.contains_key(song_path) {
                preferences
                    .records
                    .insert(song_path.clone(), PreferenceRecord::default());
                changed = true;
            }
        }
        if changed {
            preferences.save();
        }
        preferences
    }

    fn load(path: &Path) -> BTreeMap<String, PreferenceRecord> {
        let Ok(contents) = fs::read_to_string(path) else {
            return BTreeMap::new();
        };
        let Ok(document) = serde_json::from_str::<serde_json::Value>(&contents) else {
            return BTreeMap::new();
        };
        if document.get("version").and_then(serde_json::Value::as_u64)
            != Some(DATABASE_VERSION as u64)
        {
            return BTreeMap::new();
        }

        let Some(songs) = document.get("songs").and_then(serde_json::Value::as_object) else {
            return BTreeMap::new();
        };
        songs
            .iter()
            .map(|(path, value)| {
                (
                    path.clone(),
                    serde_json::from_value(value.clone()).unwrap_or_default(),
                )
            })
            .collect()
    }

    fn save(&self) {
        let parent = self
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        if fs::create_dir_all(parent).is_err() {
            return;
        }

        let document = PreferenceDocument {
            version: DATABASE_VERSION,
            songs: self.records.clone(),
        };
        let Ok(contents) = serde_json::to_vec_pretty(&document) else {
            return;
        };

        let temporary = self.path.with_file_name(format!(
            ".{}.tmp-{}",
            self.path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("song-scores.json"),
            std::process::id()
        ));
        if fs::write(&temporary, contents).is_ok() {
            if fs::rename(&temporary, &self.path).is_err() {
                // Windows does not replace an existing destination with
                // rename.  The temporary file is complete before this
                // fallback removes the old copy.
                #[cfg(windows)]
                if fs::remove_file(&self.path).is_ok() {
                    let _ = fs::rename(&temporary, &self.path);
                }
            }
            let _ = fs::remove_file(temporary);
        }
    }

    fn record_mut(&mut self, song_path: &str) -> &mut PreferenceRecord {
        self.records.entry(song_path.to_string()).or_default()
    }

    pub fn score(&self, song_path: &str) -> u8 {
        self.records
            .get(song_path)
            .map(|record| record.score)
            .unwrap_or(BASE_SCORE)
    }

    #[allow(dead_code)]
    pub fn record(&self, song_path: &str) -> PreferenceRecord {
        self.records.get(song_path).cloned().unwrap_or_default()
    }

    #[allow(dead_code)]
    pub fn database_path(&self) -> &Path {
        &self.path
    }

    pub fn mark_playback_started(&mut self, song_path: &str) {
        let record = self.record_mut(song_path);
        record.played_this_session = true;
        record.missed_sessions = 0;
        self.save();
    }

    pub fn record_direct_selection(&mut self, song_path: &str) {
        let record = self.record_mut(song_path);
        if record.score < MAX_SCORE {
            record.score += 1;
        }
        self.save();
    }

    pub fn record_skip(&mut self, song_path: &str) {
        let record = self.record_mut(song_path);
        record.score = record.score.saturating_sub(1);
        self.save();
    }

    pub fn record_completion(&mut self, song_path: &str) {
        let record = self.record_mut(song_path);
        if record.score < BASE_SCORE {
            record.score += 1;
        }
        self.save();
    }

    pub fn finalize_session(&mut self, library_paths: &[String]) {
        let mut changed = false;
        for song_path in library_paths {
            let record = self.record_mut(song_path);
            if record.played_this_session {
                record.played_this_session = false;
                changed = true;
                continue;
            }

            record.missed_sessions = record.missed_sessions.saturating_add(1);
            record.played_this_session = false;
            if record.score > BASE_SCORE {
                let decaying_misses = record.missed_sessions.saturating_sub(MISSES_BEFORE_DECAY);
                let loss = decaying_misses.min(u8::MAX as u64) as u8;
                record.score = record.score.saturating_sub(loss).max(BASE_SCORE);
            }
            changed = true;
        }

        if changed {
            self.save();
        }
    }

    #[cfg(test)]
    fn set_record(&mut self, song_path: &str, score: u8, missed_sessions: u64) {
        self.records.insert(
            song_path.to_string(),
            PreferenceRecord {
                score,
                missed_sessions,
                played_this_session: false,
            },
        );
        self.save();
    }
}

pub fn default_database_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let base = env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .or_else(|| home::home_dir())
            .unwrap_or_else(|| PathBuf::from("."));
        return base.join("neocrystal").join("song-scores.json");
    }

    #[cfg(target_os = "macos")]
    {
        let base = env::var_os("HOME")
            .map(PathBuf::from)
            .or_else(|| home::home_dir())
            .unwrap_or_else(|| PathBuf::from("."));
        return base
            .join("Library")
            .join("Application Support")
            .join("neocrystal")
            .join("song-scores.json");
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let base = env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
            .or_else(|| home::home_dir().map(|home| home.join(".local/share")))
            .unwrap_or_else(|| PathBuf::from("."));
        return base.join("neocrystal").join("song-scores.json");
    }

    #[allow(unreachable_code)]
    {
        home::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("neocrystal")
            .join("song-scores.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!(
            "neocrystal-{name}-{}-{stamp}.json",
            std::process::id()
        ))
    }

    fn paths() -> Vec<String> {
        vec!["/music/a.mp3".to_string(), "/music/b.mp3".to_string()]
    }

    #[test]
    fn defaults_and_json_round_trip() {
        let file = test_path("round-trip");
        let library = paths();
        let mut preferences = Preferences::with_path(&library, &file);
        assert_eq!(preferences.score(&library[0]), BASE_SCORE);
        preferences.record_direct_selection(&library[0]);

        let loaded = Preferences::with_path(&library, &file);
        assert_eq!(loaded.score(&library[0]), 101);
        assert_eq!(loaded.score("/music/new.mp3"), BASE_SCORE);
        let _ = fs::remove_file(file);
    }

    #[test]
    fn score_boundaries_and_completion_rules() {
        let file = test_path("boundaries");
        let library = paths();
        let mut preferences = Preferences::with_path(&library, &file);
        preferences.set_record(&library[0], MAX_SCORE, 0);
        preferences.record_direct_selection(&library[0]);
        assert_eq!(preferences.score(&library[0]), MAX_SCORE);

        preferences.set_record(&library[0], 0, 0);
        preferences.record_skip(&library[0]);
        assert_eq!(preferences.score(&library[0]), 0);
        preferences.record_completion(&library[0]);
        assert_eq!(preferences.score(&library[0]), 1);
        preferences.set_record(&library[0], BASE_SCORE, 0);
        preferences.record_completion(&library[0]);
        assert_eq!(preferences.score(&library[0]), BASE_SCORE);
        let _ = fs::remove_file(file);
    }

    #[test]
    fn played_and_consecutive_misses_decay_after_five_misses() {
        let file = test_path("sessions");
        let library = paths();
        let mut preferences = Preferences::with_path(&library, &file);
        preferences.set_record(&library[0], 110, 0);

        for expected_misses in 1..=MISSES_BEFORE_DECAY {
            preferences.finalize_session(&library);
            assert_eq!(preferences.record(&library[0]).score, 110);
            assert_eq!(
                preferences.record(&library[0]).missed_sessions,
                expected_misses
            );
        }

        preferences.finalize_session(&library);
        assert_eq!(preferences.record(&library[0]).score, 109);
        assert_eq!(preferences.record(&library[0]).missed_sessions, 6);
        preferences.finalize_session(&library);
        assert_eq!(preferences.record(&library[0]).score, 107);
        assert_eq!(preferences.record(&library[0]).missed_sessions, 7);

        preferences.mark_playback_started(&library[0]);
        assert_eq!(preferences.record(&library[0]).missed_sessions, 0);
        preferences.finalize_session(&library);
        assert_eq!(preferences.record(&library[0]).score, 107);
        assert_eq!(preferences.record(&library[0]).missed_sessions, 0);
        preferences.finalize_session(&library);
        assert_eq!(preferences.record(&library[0]).score, 107);
        assert_eq!(preferences.record(&library[0]).missed_sessions, 1);

        preferences.set_record(&library[1], 99, 4);
        preferences.finalize_session(&library);
        assert_eq!(preferences.record(&library[1]).score, 99);
        assert_eq!(preferences.record(&library[1]).missed_sessions, 5);
        let _ = fs::remove_file(file);
    }

    #[test]
    fn malformed_data_is_nonfatal_and_unknown_records_survive() {
        let file = test_path("malformed");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "not json").unwrap();
        let library = paths();
        let preferences = Preferences::with_path(&library, &file);
        assert_eq!(preferences.score(&library[0]), BASE_SCORE);

        let unknown = "/music/temporarily-removed.mp3";
        let mut preferences = Preferences::with_path(&library, &file);
        preferences.set_record(unknown, 140, 3);
        let loaded = Preferences::with_path(&library, &file);
        assert_eq!(loaded.score(unknown), 140);
        let _ = fs::remove_file(file);
    }
}
