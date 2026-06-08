use pancurses::{mousemask, Window};

use crate::modules::{general::NcursesExec, utils::ReinitMode};
use std::time::Duration;
use unicode_width::UnicodeWidthStr;

use super::general::GeneralState;

const WINDOW_WIDTH: i32 = 50;
const WINDOW_HEIGHT: i32 = 20;
const TITLE_COLOR: u32 = 9;
const ACTIVE_COLOR: u32 = 1;
const INACTIVE_COLOR: u32 = 2;
const HIGHLIGHT_COLOR: u32 = 3;
const PENDING_COLOR: u32 = 4;

#[inline]
pub fn calc(maxlen: Duration, curr: Duration) -> usize {
    ((maxlen.as_secs_f64() - curr.as_secs_f64()) / (maxlen.as_secs_f64() / 15_f64))
        .clamp(0.0, 15.0)
        .round() as usize
}

#[inline]
pub fn to_mm_ss(duration: Duration) -> String {
    format!(
        "{:02}:{:02}",
        duration.as_secs() / 60,
        duration.as_secs() % 60
    )
}
#[derive(Copy, Clone, PartialEq)]
pub enum Ownership {
    Songs,
    Subtitle,
    SongInd,
    Playlist,
    Sliding,
    ShuRep,
    Time1,
    Time2,
    ShuInd,
    LoopInd,
    Artist,
    RpcVol,
    RpcInd,
    VolInd,
    Search,
    Page,
    Progress,
}

fn centered_x(general: &GeneralState, owner: Ownership, text: &str) -> usize {
    let width = general.ui.get_range(&owner).unwrap_or(0);
    width.saturating_sub(text.width()) / 2
}

fn write_bool_indicator(
    general: &mut GeneralState,
    owner: Ownership,
    enabled: bool,
    yes: &str,
    no: &str,
) {
    let (text, color) = if enabled {
        (yes, ACTIVE_COLOR)
    } else {
        (no, INACTIVE_COLOR)
    };

    general.ui.write(&owner, 0, 0, text, color);
}

fn current_page_bounds(general: &GeneralState) -> (usize, usize, usize) {
    let page_size = general.songs.typical_page_size.max(1);
    let page = general.index.page.max(1);
    let start = (page - 1) * page_size;
    let end = (start + page_size).min(general.songs.filtered_songs.len());
    (page_size, start, end)
}

pub fn draw_subtitle(general: &mut GeneralState, text: Option<&str>) {
    let content = text.unwrap_or("");
    general.ui.write(
        &Ownership::Subtitle,
        centered_x(general, Ownership::Subtitle, content),
        0,
        content,
        TITLE_COLOR,
    );
}

pub fn autoalloc(general: &mut GeneralState) {
    general.ui.alloc(&Ownership::Songs, (2, 46), (1, 14));
    general.ui.alloc(&Ownership::SongInd, (1, 1), (1, 14));
    general.ui.alloc(&Ownership::Playlist, (2, 12), (16, 1));
    general.ui.alloc(&Ownership::Sliding, (14, 23), (16, 1));
    general.ui.alloc(&Ownership::ShuRep, (2, 8), (17, 1));
    general.ui.alloc(&Ownership::Time1, (12, 5), (17, 1));
    general.ui.alloc(&Ownership::Time2, (34, 5), (17, 1));
    general.ui.alloc(&Ownership::ShuInd, (2, 3), (18, 1));
    general.ui.alloc(&Ownership::LoopInd, (6, 3), (18, 1));
    general.ui.alloc(&Ownership::Artist, (12, 27), (18, 1));
    general.ui.alloc(&Ownership::RpcVol, (41, 7), (17, 1));
    general.ui.alloc(&Ownership::RpcInd, (41, 3), (18, 1));
    general.ui.alloc(&Ownership::VolInd, (45, 3), (18, 1));
    general.ui.c_alloc(
        &Ownership::Subtitle,
        (2, 46),
        (15, 1),
        Some("─".to_string()),
    );
    general
        .ui
        .c_alloc(&Ownership::Search, (2, 32), (0, 1), Some("─".to_string()));
    general
        .ui
        .c_alloc(&Ownership::Page, (35, 13), (0, 1), Some("─".to_string()));
    general.ui.c_alloc(
        &Ownership::Progress,
        (18, 15),
        (17, 1),
        Some("─".to_string()),
    );
}

pub fn draw_frame(general: &mut GeneralState) {
    general.ui.inject_si(0, 0, "┌", 0);
    general.ui.inject_si(49, 0, "┐", 0);
    general.ui.inject_si(0, 19, "└", 0);
    general.ui.inject_si(49, 19, "┘", 0);
    general.ui.inject_simx(1, 0, "──────", 0, 8);
    general.ui.inject_simx(1, 15, "──────", 0, 8);
    general.ui.inject_simx(1, 19, "──────", 0, 8);
    general.ui.inject_simy(0, 1, "│", 0, 18);
    general.ui.inject_simy(49, 1, "│", 0, 18);
    general.ui.inject_si(0, 15, "├", 0);
    general.ui.inject_si(49, 15, "┤", 0);
}

pub fn draw_page(general: &mut GeneralState) {
    let total = general.songs.filtered_songs.len();
    let psize = general.songs.typical_page_size.max(1);
    let max_page = (total + psize - 1) / psize;
    let cur_page = general.index.page.max(1).min(max_page.max(1));
    general.ui.write(
        &Ownership::Page,
        0,
        0,
        &format!("< Page: {}/{} >", cur_page, max_page.max(1)),
        0,
    );
}

pub fn draw_search(general: &mut GeneralState) {
    let text = format!("Search: {}", general.searchquery.query);
    general.ui.write(
        &Ownership::Search,
        0,
        0,
        if general.searchquery.mode == 0 {
            "Search or edit"
        } else {
            &text
        },
        TITLE_COLOR,
    );
}

pub fn draw_header(general: &mut GeneralState) {
    draw_search(general);
    draw_page(general);
}

pub struct PageData {
    current: Option<usize>,
    next: Option<usize>,
    blacklist: Vec<usize>,
    select: usize,
}

impl PageData {
    pub fn new() -> Self {
        Self {
            current: None,
            next: None,
            blacklist: Vec::with_capacity(3),
            select: 0,
        }
    }
    fn page_song_index(general: &GeneralState, row: usize) -> Option<usize> {
        let (page_size, start, end) = current_page_bounds(general);
        if row >= page_size {
            return None;
        }

        let absolute = start + row;
        if absolute >= end {
            return None;
        }

        Some(general.songs.get_ordered()[absolute])
    }

    pub fn draw_unchanged_moved_page(&mut self, general: &mut GeneralState) {
        if self.select == general.index.index {
            return;
        }

        if !general.state.desel {
            if let Some(index) = Self::page_song_index(general, self.select) {
                let name = general.songs.all_songs[index].name.clone();
                general
                    .ui
                    .write(&Ownership::Songs, 0, self.select, &name, 0);
            }
            if let Some(index) = Self::page_song_index(general, general.index.index) {
                let name = general.songs.all_songs[index].name.clone();
                general.ui.write(
                    &Ownership::Songs,
                    0,
                    general.index.index,
                    &name,
                    HIGHLIGHT_COLOR,
                );
            }
        }

        self.select = general.index.index;
    }

    pub fn draw_changed_moved_page(&mut self, general: &mut GeneralState) {
        let (page_size, start, end) = current_page_bounds(general);

        let mut row = 0;
        for abs in start..end {
            let original = general.songs.get_ordered()[abs];
            let song = &general.songs.all_songs[original];
            let color = if general.index.index == row && !general.state.desel {
                self.select = row;
                HIGHLIGHT_COLOR
            } else {
                0
            };

            general
                .ui
                .write(&Ownership::Songs, 0, row, &song.name, color);
            row += 1;
        }

        while row < page_size {
            general.ui.empty_instruction(&Ownership::Songs, row);
            row += 1;
        }
    }
    pub fn draw_indicators(&mut self, general: &mut GeneralState) {
        if let Some(r) = self.current {
            general.ui.empty_instruction(&Ownership::SongInd, r);
        }
        if let Some(r) = self.next {
            general.ui.empty_instruction(&Ownership::SongInd, r);
        }
        for &r in &self.blacklist {
            general.ui.empty_instruction(&Ownership::SongInd, r);
        }
        self.current = None;
        self.next = None;
        self.blacklist.clear();

        let (page_size, start, end) = current_page_bounds(general);

        let current = general.songs.match_c();
        let next = general.songs.get_next();

        let mut row = 0;
        for abs in start..end {
            let original = general.songs.get_ordered()[abs];

            if abs == current {
                self.current = Some(row);
                general
                    .ui
                    .write(&Ownership::SongInd, 0, row, ">", ACTIVE_COLOR);
            } else if original == next && !general.state.isloop {
                self.next = Some(row);
                general
                    .ui
                    .write(&Ownership::SongInd, 0, row, "*", PENDING_COLOR);
            } else if general.songs.is_blacklist(original) {
                self.blacklist.push(row);
                general
                    .ui
                    .write(&Ownership::SongInd, 0, row, "x", INACTIVE_COLOR);
            }

            row += 1;
        }

        while row < page_size {
            general.ui.empty_instruction(&Ownership::SongInd, row);
            row += 1;
        }
    }
}

pub fn draw_playlist(general: &mut GeneralState) {
    general.ui.empty_instruction(&Ownership::Playlist, 0);
    general.ui.write(
        &Ownership::Playlist,
        0,
        0,
        &general.songs.current_playlist(),
        0,
    );
}
pub fn draw_sliding(general: &mut GeneralState) {
    let sliding = general.sliding.visible_text();
    general.ui.write(
        &Ownership::Sliding,
        centered_x(general, Ownership::Sliding, &sliding),
        0,
        &sliding,
        ACTIVE_COLOR,
    );
}

pub fn draw_const(general: &mut GeneralState) {
    general.ui.write(&Ownership::ShuRep, 0, 0, "Shu Rep", 0);
    general.ui.write(&Ownership::RpcVol, 0, 0, "Rpc Vol", 0);
}

pub fn draw_shuffle_indc(general: &mut GeneralState) {
    write_bool_indicator(
        general,
        Ownership::ShuInd,
        general.songs.shuffle,
        "yes",
        "no",
    );
}
pub fn draw_loop_indc(general: &mut GeneralState) {
    write_bool_indicator(
        general,
        Ownership::LoopInd,
        general.state.isloop,
        "yes",
        "no",
    );
}
pub fn draw_time_cur(general: &mut GeneralState) {
    general.ui.write(
        &Ownership::Time1,
        0,
        0,
        &to_mm_ss(general.timer.maxlen.saturating_sub(general.timer.fcalc)),
        0,
    );
}
pub fn draw_time_max(general: &mut GeneralState) {
    general
        .ui
        .write(&Ownership::Time2, 0, 0, &to_mm_ss(general.timer.maxlen), 0);
}
pub fn draw_artist(general: &mut GeneralState) {
    let artist = general.songs.current_artist();
    general.ui.write(
        &Ownership::Artist,
        centered_x(general, Ownership::Artist, &artist),
        0,
        &artist,
        0,
    );
}

#[cfg(feature = "rpc")]
pub fn draw_rpc_indc(general: &mut GeneralState) {
    general.ui.write(
        &Ownership::RpcInd,
        0,
        0,
        match general.rpc.mode {
            ReinitMode::Init => "int",
            ReinitMode::Renew | ReinitMode::Pretend => "rnw",
            ReinitMode::None => "yes",
        }
        .into(),
        match general.rpc.mode {
            ReinitMode::None => 1,
            ReinitMode::Renew | ReinitMode::Pretend => 4,
            ReinitMode::Init => 2,
        },
    );
}

#[cfg(not(feature = "rpc"))]
pub fn draw_rpc_indc(general: &mut GeneralState) {
    general.ui.write(&Ownership::RpcInd, 0, 0, "no".into(), 2)
}

pub fn draw_vol_indc(general: &mut GeneralState) {
    general.ui.write(
        &Ownership::VolInd,
        0,
        0,
        &format!("{:>3}", general.volume.steps),
        0,
    );
}

pub fn draw_footer(general: &mut GeneralState) {
    draw_playlist(general);
    draw_sliding(general);
    draw_const(general);
    draw_shuffle_indc(general);
    draw_loop_indc(general);
    draw_time_cur(general);
    draw_time_max(general);
    draw_artist(general);
    draw_rpc_indc(general);
    draw_vol_indc(general);
}

pub fn draw_progress(general: &mut GeneralState) {
    general.ui.write_simx(
        &Ownership::Progress,
        0,
        0,
        "─",
        ACTIVE_COLOR,
        calc(general.timer.maxlen, general.timer.fcalc),
    );
}

pub fn draw_all(general: &mut GeneralState, page: &mut PageData) {
    autoalloc(general);
    draw_frame(general);
    draw_progress(general);
    draw_header(general);
    page.draw_indicators(general);
    page.draw_changed_moved_page(general);
    draw_footer(general);
}

pub fn update(general: &mut GeneralState, window: &mut Window) {
    general.ui.draw::<Window, NcursesExec>(window);
}

pub fn init_curses(window: &mut Window) {
    pancurses::curs_set(0);
    window.keypad(true);
    pancurses::noecho();
    window.nodelay(true);
    mousemask(0x2 as u32, None);
    window.resize(WINDOW_HEIGHT, WINDOW_WIDTH);
    pancurses::start_color();
    pancurses::init_pair(0, pancurses::COLOR_WHITE, pancurses::COLOR_BLACK);
    pancurses::init_pair(
        ACTIVE_COLOR as i16,
        pancurses::COLOR_GREEN,
        pancurses::COLOR_BLACK,
    );
    pancurses::init_pair(
        INACTIVE_COLOR as i16,
        pancurses::COLOR_RED,
        pancurses::COLOR_BLACK,
    );
    pancurses::init_pair(
        HIGHLIGHT_COLOR as i16,
        pancurses::COLOR_BLACK,
        pancurses::COLOR_WHITE,
    );
    pancurses::init_pair(
        PENDING_COLOR as i16,
        pancurses::COLOR_YELLOW,
        pancurses::COLOR_BLACK,
    );
    pancurses::init_pair(
        TITLE_COLOR as i16,
        pancurses::COLOR_CYAN,
        pancurses::COLOR_BLACK,
    );
}

pub fn exit_curses(window: &mut Window) {
    pancurses::curs_set(1);
    pancurses::echo();
    window.nodelay(false);
    pancurses::endwin();
}
