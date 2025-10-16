use pancurses::Window;
use std::time::Duration;
use super::songs::Songs;
use std::ffi::CString;
use libc::{setlocale, LC_ALL};
pub unsafe fn init_locale() {
    unsafe {
        let locale = CString::new("").unwrap();
        setlocale(LC_ALL, locale.as_ptr());
    }
}

#[inline]
pub fn calc(maxlen: Duration, curr: Duration) -> usize {
    ((maxlen.as_secs_f64() - curr.as_secs_f64()) / (maxlen.as_secs_f64() / 15_f64)).clamp(0.0, 15.0).round() as usize
}

pub fn redraw(window: &mut Window, maxx: i32, maxy: i32, songs: &mut Songs, page: usize, local_volume_counter: u8, 
        is_search: String, isloop: bool, reinit_rpc: bool, maxlen: Duration, fcalc: Duration, fun_index: usize, setnext: usize) {
    window.erase();
    //window.mvchgat(0, 0, 999, pancurses::A_NORMAL, 9);
    window.border('│', '│', '─', '─', '┌', '┐', '└', '┘');
    {
        let page_indicator = format!("Page {}/{}", page, (songs.filtered_songs.len() as f32 / songs.typical_page_size as f32).ceil() as usize);
        window.mvaddstr(0, maxx - (page_indicator.len() as i32 + 2), page_indicator.as_str());
        window.mvchgat(0, maxx - (page_indicator.len() as i32 + 2), page_indicator.len() as i32, pancurses::A_NORMAL, 0);
    }
    if is_search != "false" {
        window.mvaddstr(0, 2, format!("Input: {}", is_search).as_str());
        window.mvchgat(0, 2, format!("Input: {}", is_search).chars().count() as i32, pancurses::A_NORMAL, 9);
    } else {
        window.mvaddstr(0, 2, "Search or edit");
        window.mvchgat(0, 2, "Search or edit".len() as i32, pancurses::A_NORMAL, 9);
    }
    { // song draw
        let start_index = (page-1) * songs.typical_page_size;
        let end_index = std::cmp::min(start_index + songs.typical_page_size, songs.filtered_songs.len());
        for (i, song) in songs.filtered_songs[start_index..end_index].iter().enumerate() {
            let display_name = &song.name;
            window.mvaddstr(i as i32 + 1, 2, display_name.as_str());
            window.mvchgat(i as i32 + 1, 2, display_name.chars().count() as i32, pancurses::A_NORMAL, 0);
            if i == fun_index {
                // highlight with color pair 3
                window.mvchgat(i as i32 + 1, 2, display_name.chars().count() as i32, pancurses::A_NORMAL, 3);
            }
            if song.name == songs.current_name() {
                // highlight with a green * at the end or yellow if paused (stophandler)
                window.mvaddstr(i as i32 + 1, format!("{} *", display_name).chars().count() as i32, " *");
                window.mvchgat(i as i32 + 1, format!("{} *", display_name).chars().count() as i32, 2, pancurses::A_NORMAL, match songs.stophandler {true => 4, false => 1});

            } else if songs.is_blacklist(i) {
                window.mvaddstr(i as i32 + 1, format!("{} B", display_name).chars().count() as i32, " BL");
                window.mvchgat(i as i32 + 1, format!("{} B", display_name).chars().count() as i32, 3, pancurses::A_NORMAL, 2);
            } else if song.original_index == setnext {
                window.mvaddstr(i as i32 + 1, format!("{} N", display_name).chars().count() as i32, " N");
                window.mvchgat(i as i32 + 1, format!("{} N", display_name).chars().count() as i32, 2, pancurses::A_NORMAL, 4);
            }
        }  
    }
    window.mvaddstr(maxy-5, 0, "├".to_owned() + "─".repeat((maxx-2) as usize).as_str() + "┤");
    window.mvaddstr(maxy-4, 2, format!("{}", songs.current_name().replace("music/", "").replace("music\\", "").replace(".mp3", "")).as_str());
    window.mvchgat(maxy-4, 2, maxx-4, pancurses::A_NORMAL, 1);
    window.mvaddstr(maxy-3, 2, "Shuffle  Loop                     Rpc      Vol ");
    window.mvaddstr(maxy-2, 2, format!("{} ", match songs.shuffle { true => "true", false => "false" }));
    window.mvchgat(maxy-2, 2, format!("{} ", match songs.shuffle { true => "true", false => "false" }).len() as i32, pancurses::A_NORMAL, match songs.shuffle { true => 1, false => 2 });
    window.mvaddstr(maxy-2, 11, format!("{} ", match isloop { true => "true", false => "false" }));
    window.mvchgat(maxy-2, 11, format!("{} ", match isloop { true => "true", false => "false" }).len() as i32, pancurses::A_NORMAL, match isloop { true => 1, false => 2 });
    {
        let artist_name = songs.get_artist_search();
        window.mvaddstr(maxy-2, maxx/2 - (artist_name.chars().count() as i32)/2, artist_name.as_str());
        window.mvchgat(maxy-2, maxx/2 - (artist_name.chars().count() as i32)/2, artist_name.chars().count() as i32, pancurses::A_NORMAL, 0);
    }
    window.mvaddstr(
        maxy-2,
        maxx - ((format!("{} ", local_volume_counter)).len() as i32 + 1),
        format!("{}", local_volume_counter)
    );
    window.mvchgat(maxy-2, maxx - ((format!("{} ", local_volume_counter)).len() as i32 + 1), (format!("{}", local_volume_counter)).len() as i32, pancurses::A_NORMAL, 0);
    if reinit_rpc { // reinit display
        window.mvaddstr(maxy-2, maxx - 14, "init");
        window.mvchgat(maxy-2, maxx - 14, "init".len() as i32, pancurses::A_NORMAL, 2);
    } else {
        window.mvaddstr(maxy-2, maxx - 14, "done");
        window.mvchgat(maxy-2, maxx - 14, "done".len() as i32, pancurses::A_NORMAL, 1);
    }
    window.mvaddstr(maxy-3, maxx/2-7, "─".repeat(15));
    if maxlen != Duration::from_secs(0) {
        window.mvchgat(maxy-3, maxx/2-7, calc(maxlen, fcalc) as i32, pancurses::A_NORMAL, 1);
    }
    window.refresh();
}

pub fn init_curses(window: &mut Window) {
    (pancurses::curs_set(0), window.keypad(true), pancurses::noecho(), window.nodelay(true));
    window.resize(20, 50);
    (
        pancurses::start_color(),
        pancurses::init_pair(1, pancurses::COLOR_GREEN, pancurses::COLOR_BLACK),
        pancurses::init_pair(2, pancurses::COLOR_RED, pancurses::COLOR_BLACK),
        pancurses::init_pair(0, pancurses::COLOR_WHITE, pancurses::COLOR_BLACK),
        pancurses::init_pair(3, pancurses::COLOR_BLACK, pancurses::COLOR_WHITE),
        pancurses::init_pair(4, pancurses::COLOR_YELLOW, pancurses::COLOR_BLACK),
        pancurses::init_pair(9, pancurses::COLOR_CYAN, pancurses::COLOR_BLACK),
        window.attron(pancurses::A_NORMAL),
        window.attron(pancurses::A_NORMAL),
    );
}
