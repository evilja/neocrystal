use pancurses::{
    Window, ACS_HLINE, ACS_LLCORNER, ACS_LRCORNER, ACS_LTEE, ACS_RTEE, ACS_ULCORNER,
    ACS_URCORNER, ACS_VLINE, COLOR_PAIR, mousemask
};
#[cfg(not(feature = "rpc"))]
use std::sync::Mutex;
use std::time::Duration;
use unicode_width::UnicodeWidthStr;
const MAXX: usize = 50;
const MAXY: usize = 20;
use crate::modules::{utils::ReinitMode};

use super::general::GeneralState;


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
}

pub fn autoalloc(general: &mut GeneralState) {
    /* ---------------- BEGIN ALLOCATION ---------------- */
    general.ui.alloc(&Ownership::Songs,   (2, 46), (1, 14));
    general.ui.alloc(&Ownership::SongInd, (1, 1),  (1, 14));
    general.ui.alloc(&Ownership::Playlist,(2, 12), (16, 1));
    general.ui.alloc(&Ownership::Sliding, (14, 23),(16, 1));
    general.ui.alloc(&Ownership::ShuRep,  (2, 8),  (17, 1));
    general.ui.alloc(&Ownership::Time1,   (12, 4), (17, 1));
    general.ui.alloc(&Ownership::Time2,   (34, 4), (17, 1));
    general.ui.alloc(&Ownership::ShuInd,  (2, 3),  (18, 1));
    general.ui.alloc(&Ownership::LoopInd, (6, 3),  (18, 1));
    general.ui.alloc(&Ownership::Artist,  (12, 27),(18, 1));
    general.ui.alloc(&Ownership::RpcVol,  (41, 7), (17, 1));
    general.ui.alloc(&Ownership::RpcInd,  (41, 3), (18, 1));
    general.ui.alloc(&Ownership::VolInd,  (45, 3), (18, 1));
    general.ui.c_alloc(&Ownership::Search,  (2, 32), (0, 1), Some(ACS_HLINE()));
    general.ui.c_alloc(&Ownership::Page,    (35, 13),(0, 1), Some(ACS_HLINE()));
    /* ---------------- END ALLOCATION ---------------- */

}

pub fn draw_frame(window: &Window) {
    window.clear();
    window.border(
        ACS_VLINE(),
        ACS_VLINE(),
        ACS_HLINE(),
        ACS_HLINE(),
        ACS_ULCORNER(),
        ACS_URCORNER(),
        ACS_LLCORNER(),
        ACS_LRCORNER(),
    );

    window.mv(MAXY as i32 - 5, 0);
    window.addch(ACS_LTEE());
    for _ in 0..(MAXX - 2) {
        window.addch(ACS_HLINE());
    }
    window.addch(ACS_RTEE());
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
        format!("< Page: {}/{} >", cur_page, max_page.max(1)),
        0,
    );
}

pub fn draw_search(general: &mut GeneralState) {
    general.ui.write(
        &Ownership::Search,
        0,
        0,
        if general.searchquery.mode == 0 {
            "Search or edit".to_string()
        } else {
            format!("Search: {}", general.searchquery.query)
        },
        9,
    );
}

pub fn draw_header(general: &mut GeneralState) {
    draw_search(general);
    draw_page(general);
}

pub fn draw_song_indicators(general: &mut GeneralState) {
    let total = general.songs.filtered_songs.len();
    let psize = general.songs.typical_page_size.max(1);

    let page  = general.index.page.max(1);
    let start = (page - 1) * psize;
    let end   = (start + psize).min(total);

    let current = general.songs.match_c();
    let next    = general.songs.get_next();

    let mut row = 0;

    for abs in start..end {
        let original = general.songs.filtered_songs[abs];

        let mark = if abs == current {
            '>'
        } else if original == next && !general.state.isloop {
            '*'
        } else if general.songs.is_blacklist(original) {
            'x'
        } else {
            ' '
        };

        general.ui.write(
            &Ownership::SongInd,
            0,
            row,
            mark.to_string(),
            match mark {
                '>' => 1,
                '*' => 4,
                'x' => 2,
                _   => 0,
            },
        );

        row += 1;
    }

    while row < psize {
        general.ui.write(&Ownership::SongInd, 0, row, " ".into(), 0);
        row += 1;
    }
}

pub fn draw_song_text(general: &mut GeneralState) {
    let total = general.songs.filtered_songs.len();
    let psize = general.songs.typical_page_size.max(1);

    let page  = general.index.page.max(1);
    let start = (page - 1) * psize;
    let end   = (start + psize).min(total);

    let mut row = 0;

    for abs in start..end {
        let original = general.songs.filtered_songs[abs];
        let song     = &general.songs.all_songs[original];

        general.ui.write(
            &Ownership::Songs,
            0,
            row,
            song.name.clone(),
            if general.index.index == row && !general.state.desel { 3 } else { 0 },
        );

        row += 1;
    }

    // clear remaining rows
    while row < psize {
        general.ui.write(&Ownership::Songs, 0, row, "".into(), 0);
        row += 1;
    }
}

pub fn draw_playlist(general: &mut GeneralState) {
    general.ui.write(
        &Ownership::Playlist,
        0,
        0,
        general.songs.current_playlist(),
        0,
    );
}
pub fn draw_sliding(general: &mut GeneralState) {
    let sliding = general.sliding.visible_text();
    general.ui.write(
        &Ownership::Sliding,
        (general.ui.get_range(&Ownership::Sliding).unwrap() / 2)
            .saturating_sub(sliding.width() / 2),
        0,
        sliding,
        1,
    );
}

pub fn draw_const(general: &mut GeneralState) {
    general.ui.write(&Ownership::ShuRep, 0, 0, "Shu Rep".to_string(), 0);
    general.ui.write(&Ownership::RpcVol, 0, 0, "Rpc Vol".to_string(), 0);
}

pub fn draw_shuffle_indc(general: &mut GeneralState) {
    general.ui.write(
        &Ownership::ShuInd,
        0,
        0,
        if general.songs.shuffle { "yes" } else { "no" }.into(),
        if general.songs.shuffle { 1 } else { 2 },
    );
}
pub fn draw_loop_indc(general: &mut GeneralState) {
    general.ui.write(
        &Ownership::LoopInd,
        0,
        0,
        if general.state.isloop { "yes" } else { "no" }.into(),
        if general.state.isloop { 1 } else { 2 },
    );
}
pub fn draw_time_cur(general: &mut GeneralState) {
    general.ui.write(&Ownership::Time1, 0, 0, to_mm_ss(general.timer.maxlen - general.timer.fcalc), 0);
}
pub fn draw_time_max(general: &mut GeneralState) {
    general.ui.write(&Ownership::Time2, 0, 0, to_mm_ss(general.timer.maxlen), 0);
}
pub fn draw_artist(general: &mut GeneralState) {
    let artist = general.songs.current_artist();
    general.ui.write(
        &Ownership::Artist,
        (general.ui.get_range(&Ownership::Artist).unwrap() / 2)
            .saturating_sub(artist.width() / 2),
        0,
        artist,
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
        .into()
        ,
        match general.rpc.mode {
            ReinitMode::None => 1,
            ReinitMode::Renew | ReinitMode::Pretend => 4,
            ReinitMode::Init => 2,
        }
    );
}

#[cfg(not(feature = "rpc"))]
pub fn draw_rpc_indc(_: &mut GeneralState) {
    general.ui.write(
        &Ownership::RpcInd,
        0,
        0,
        "no".into(),
        2
    )
}

pub fn draw_vol_indc(general: &mut GeneralState) {
    general.ui.write(
        &Ownership::VolInd,
        0,
        0,
        format!("{:>3}", general.volume.steps),
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


pub fn draw_progress(window: &Window, maxlen: Duration, fcalc: Duration) {
    let start = MAXX as i32 / 2 - 7;

    window.mv(MAXY as i32 - 3, start);
    for _ in 0..15 {
        window.addch(ACS_HLINE());
    }

    if maxlen != Duration::ZERO {
        window.mv(MAXY as i32 - 3, start);
        window.attron(COLOR_PAIR(1));
        for _ in 0..calc(maxlen, fcalc) {
            window.addch(ACS_HLINE());
        }
        window.attroff(COLOR_PAIR(1));
    }
}

pub fn draw_all(general: &mut GeneralState, window: &mut Window) {
    draw_frame(window);
    draw_progress(window, general.timer.maxlen, general.timer.fcalc);
    draw_header(general);
    draw_song_indicators(general);
    draw_song_text(general);
    draw_footer(general);
}

pub fn update(general: &mut GeneralState, window: &mut Window) {
    general.ui.draw(window);
    window.noutrefresh();
    pancurses::doupdate();
}

pub fn init_curses(window: &mut Window) {
    (
        pancurses::curs_set(0),
        window.keypad(true),
        pancurses::noecho(),
        window.nodelay(true),
        mousemask(0x2 as u32, None),
    );
    window.resize(20, 50);
    (
        pancurses::start_color(),
        pancurses::init_pair(1, pancurses::COLOR_GREEN, pancurses::COLOR_BLACK),
        pancurses::init_pair(2, pancurses::COLOR_RED, pancurses::COLOR_BLACK),
        pancurses::init_pair(0, pancurses::COLOR_WHITE, pancurses::COLOR_BLACK),
        pancurses::init_pair(3, pancurses::COLOR_BLACK, pancurses::COLOR_WHITE),
        pancurses::init_pair(4, pancurses::COLOR_YELLOW, pancurses::COLOR_BLACK),
        pancurses::init_pair(9, pancurses::COLOR_CYAN, pancurses::COLOR_BLACK),
    );
}



pub fn exit_curses(window: &mut Window) {
    pancurses::curs_set(1);
    pancurses::echo();
    window.nodelay(false);
    pancurses::endwin();
}