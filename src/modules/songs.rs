use rand::prelude::*;
use super::utils::artist_data;
#[derive(Clone)]
pub struct Songs {
    pub songs: Vec<String>,
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
        Self {
            songs: songs.clone(),
            cache_artist: songs.iter().map(|s| artist_data(s)).collect(),
            current_song: 0,
            current_name: "Nothing".to_string(),
            typical_page_size: 14,
            blacklist: vec![],
            stophandler: true,
            shuffle: false,
        }
    }
    pub fn get_artist(&self, index: usize) -> String {
        if index >= self.cache_artist.len() {
            return "Unknown".to_string();
        }
        self.cache_artist[index].clone()
    }

    pub fn blacklist(&mut self, index: usize) {
        for i in 0..self.blacklist.len() {
            if self.blacklist[i] == index {
                self.blacklist.remove(i);
                return;
            }
        }
        self.blacklist.push(index);
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

        if self.blacklist.contains(&absolute) {
            return Err(0);
        }
        self.current_song = absolute;
        self.current_name = self.songs[absolute].clone();
        self.stophandler = false;
        Ok(())
    }
    pub fn set_by_next(&mut self) -> Result<usize, ()> {
        let last_possible_index = self.songs.len() - 1;
        if self.shuffle {
            for _ in 0..last_possible_index {
                match self.set_by_pindex(*(0..last_possible_index).collect::<Vec<usize>>().choose(&mut rand::rng()).unwrap(), 1) {
                    Ok(_) => return Ok(0),
                    Err(_) => (),
                }
            }
            return Err(());
        }
        for i in self.current_song+1..last_possible_index {
            if !self.blacklist.contains(&i) {
                self.current_song = i;
                self.current_name = self.songs[i].clone();
                self.stophandler = false;
                return Ok(i);
            }
        }
        for i in 0..self.current_song {
            if !self.blacklist.contains(&i) {
                self.current_song = i;
                self.current_name = self.songs[i].clone();
                self.stophandler = false;
                return Ok(i); 
            }
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
