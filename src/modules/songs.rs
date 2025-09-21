

#[derive(Clone)]
pub struct Songs {
    pub songs: Vec<String>,
    pub current_song: usize,
    pub current_name: String,
    pub typical_page_size: usize,
    pub blacklist: Vec<usize>, 
    pub stophandler: bool,
}

#[inline]
fn absolute_index(index: usize, page: usize, typical_page_size: usize) -> usize {
    index + ((page - 1) * typical_page_size)
}
            /// Documentation for Songs implementation
            /// 
            /// constructor -> Constructs Songs struct with a given String Vector. Vector should include paths.
            /// blacklist -> Auto adds/removes an index from blacklist.
            /// all_songs -> returns a self.songs clone.
            /// current_index -> returns current index when the song is not stopped by stop function.
            /// current_name -> returns current song name when the song is not stopped by stop function.
            /// set_by_pindex
            /// Sets the current song using a page and an index within that page.
            ///
            /// # Arguments
            /// * `index` - The index of the song within the page (0-based).
            /// * `page` - The page number (1-based). The absolute index is calculated as `index + (page - 1) * typical_page_size`.
            ///
            /// # Returns
            /// * `Ok(())` if the song is successfully set.
            /// * `Err(0)` if the song is blacklisted.
            /// set_by_next -> sets the current song to the next available song.
            /// stop -> blocks current_name and current_index getter functions.
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
}
