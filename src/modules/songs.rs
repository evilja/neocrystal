

#[derive(Clone)]
pub struct Songs {
    pub songs: Vec<String>,
    pub current_song: usize,
    pub current_name: String,
    pub typical_page_size: usize,
    pub blacklist: Vec<usize>, 
    pub stophandler: bool,
}

impl Songs {
    pub fn constructor(songs: Vec<String>) -> Self {
        Self {
            songs: songs.clone(),
            current_song: 0,
            current_name: "Nothing".to_string(),
            typical_page_size: 14,
            blacklist: vec![],
            stophandler: true,
        }
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
            return 0;
        }
        return self.current_song.clone();
    }
    pub fn current_name(&self) -> String {
        if self.stophandler {
            return "Nothing".to_string();
        }
        return self.current_name.clone();
    }
    pub fn set_by_pindex(&mut self, index: usize, page: usize, setbynext: bool) -> usize {
        if index + ((page - 1) * self.typical_page_size) == self.songs.len() { // if given index is the last song, search entire songs vector for any available
            for i in 0..self.songs.len() {
                if !self.blacklist.contains(&i) {
                    self.current_song = i;
                    self.current_name = self.songs[i].clone();
                    self.stophandler = false;
                    return i as usize;
                } else if !setbynext {
                    self.stophandler = false;
                    return self.current_song;
                }

            }
        }
        if !setbynext && self.blacklist.contains(&(index + (page - 1) * self.typical_page_size)) {
            return 9879871 as usize;
        } else if setbynext {
            for i in 0..self.songs.len() {
                match self.set_by_pindex(index+i, page, false) {
                    9879871 => (),
                    _ => {
                        self.stophandler = false;
                        return self.current_song;
                    }
                }
            }
            self.stophandler = true;
            return 9879871 as usize;
        }
        self.current_song = index + ((page-1) * self.typical_page_size);
        self.current_name = self.songs[self.current_song].clone();
        self.stophandler = false;
        index + ((page-1) * self.typical_page_size)
        
    }
    pub fn set_by_next(&mut self) -> usize {
        self.set_by_pindex(self.current_song+1, 1, true)
    }
    pub fn stop(&mut self) {
        self.stophandler = true;
    }
}
