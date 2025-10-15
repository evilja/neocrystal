use rand::prelude::*;
use super::utils::artist_data;
#[derive(Clone)]
pub struct Songs {
    pub ins_s: Vec<String>,
    pub songs: Vec<String>,
    pub ins_a: Vec<String>,
    pub cache_artist: Vec<String>,
    pub current_song: usize,
    pub current_name: String,
    pub typical_page_size: usize,
    pub blacklist: Vec<usize>, 
    pub stophandler: bool,
    pub shuffle: bool,
    
}

#[inline]
pub fn absolute_index(index: usize, page: usize, typical_page_size: usize) -> usize {
    index + ((page - 1) * typical_page_size)
}

impl Songs {
    pub fn constructor(songs: Vec<String>) -> Self {
        let x: Vec<String> = songs.iter().map(|s| artist_data(s)).collect();
        Self {
            ins_s: songs.clone(),
            songs,
            ins_a: x.clone(),
            cache_artist: x,
            current_song: 0,
            current_name: "Nothing".to_string(),
            typical_page_size: 14,
            blacklist: vec![],
            stophandler: true,
            shuffle: false,
        }
    }
    pub fn get_artist(&self, index: usize) -> String {
        if self.stophandler || index == usize::MAX {
            return "Nothing".to_string();
        }
        return self.ins_a[self.ins_s.iter().position(|x| x == &self.current_name).unwrap()].clone();
    }
    pub fn search(&mut self, pattern: String) {
        if pattern == "false" || pattern.is_empty() {
            self.songs = self.ins_s.clone();
            self.cache_artist = self.ins_a.clone();
            return;
        }

        // search for pattern in ins_s and update songs and cache_artist
        self.songs.clear();
        self.cache_artist.clear();
        for (i, song) in self.ins_s.iter().enumerate() {
            if song.to_lowercase().contains(&pattern.to_lowercase()) || self.ins_a[i].to_lowercase().contains(&pattern.to_lowercase()) {
                self.songs.push(song.clone());
                self.cache_artist.push(self.ins_a[i].clone());
            }
        }
    }

    pub fn blacklist(&mut self, index_in_songs: usize) {
    // Convert index from songs[] (filtered) to ins_s[] (original)
        if index_in_songs >= self.songs.len() {
            return;
        }

        // Find the original index in ins_s
        if let Some(original_index) = self.ins_s.iter().position(|s| s == &self.songs[index_in_songs]) {
            if let Some(pos) = self.blacklist.iter().position(|&x| x == original_index) {
                // If already blacklisted, remove it
                self.blacklist.remove(pos);
            } else {
                // Otherwise add it
                self.blacklist.push(original_index);
            }
            
        }
    }
    pub fn is_blacklist(&self, index_in_songs: usize) -> bool {
        if index_in_songs >= self.songs.len() {
            return false;
        }

        if let Some(original_index) = self.ins_s.iter().position(|s| s == &self.songs[index_in_songs]) {
            self.blacklist.contains(&original_index)
        } else {
            false
        }
    }

    pub fn _all_songs(&self) -> Vec<String> {
        return self.songs.clone();
    }
    #[allow(dead_code)]
    pub fn current_index(&self) -> usize {
        if self.stophandler {
            return usize::MAX;
        }
        return self.current_song.clone();
    }
    pub fn current_name(&self) -> String {
        if self.stophandler {
            return "Nothing".to_string();
        }
        return self.current_name.clone();
    }
    pub fn set_by_pindex(&mut self, index: usize, page: usize) -> Result<(), u8> { 
        let absolute = absolute_index(index, page, self.typical_page_size);

        if absolute >= self.songs.len() {
            return Err(1);
        }

        // Map to ins_s index
        if let Some(original_index) = self.ins_s.iter().position(|s| s == &self.songs[absolute]) {
            if self.blacklist.contains(&original_index) {
                return Err(0);
            }
            self.current_song = absolute;
            self.current_name = self.songs[absolute].clone();
            self.stophandler = false;
            Ok(())
        } else {
            Err(1)
        }
    }

    pub fn set_by_next(&mut self) -> Result<usize, ()> {
        if self.songs.is_empty() {
            return Err(()); // nothing to play
        }

        let last_index = self.songs.len() - 1;

        // 🔀 SHUFFLE MODE
        if self.shuffle {
            let mut rng = rand::thread_rng();
            // try as many times as there are songs to find a non-blacklisted one
            for _ in 0..=last_index {
                let candidate = (0..=last_index).choose(&mut rng).unwrap();
                // Map songs index → ins_s index
                if let Some(original_index) = self.ins_s.iter().position(|s| s == &self.songs[candidate]) {
                    if !self.blacklist.contains(&original_index) {
                        self.current_song = candidate;
                        self.current_name = self.songs[candidate].clone();
                        self.stophandler = false;
                        return Ok(candidate);
                    }
                }
            }
            return Err(());
        }

        // ▶️ SEQUENTIAL MODE
        let mut try_index = self.current_song + 1;

        // First: go forward until end
        while try_index <= last_index {
            let original_index = self.ins_s.iter().position(|s| s == &self.songs[try_index]).unwrap();
            if !self.blacklist.contains(&original_index) {
                self.current_song = try_index;
                self.current_name = self.songs[try_index].clone();
                self.stophandler = false;
                return Ok(try_index);
            }
            try_index += 1;
        }

        // Then: wrap around from 0 to current_song - 1
        try_index = 0;
        while try_index < self.current_song {
            let original_index = self.ins_s.iter().position(|s| s == &self.songs[try_index]).unwrap();
            if !self.blacklist.contains(&original_index) {
                self.current_song = try_index;
                self.current_name = self.songs[try_index].clone();
                self.stophandler = false;
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
