use pancurses::{Window, mousemask,ACS_VLINE,ACS_HLINE,ACS_ULCORNER, ACS_URCORNER, ACS_LLCORNER, ACS_LRCORNER, ACS_LTEE, ACS_RTEE, COLOR_PAIR};
use std::time::Duration;
use super::songs::Songs;
use super::crystal_manager::{UI, UIElement, Action, Part};
use std::ffi::CString;
use libc::{setlocale, LC_ALL};

pub fn init_locale() {
    unsafe {
        let locale = CString::new("en_US.UTF-8").unwrap();
        setlocale(LC_ALL, locale.as_ptr());
    }
}

#[inline]
pub fn calc(maxlen: Duration, curr: Duration) -> usize {
    ((maxlen.as_secs_f64() - curr.as_secs_f64()) / (maxlen.as_secs_f64() / 15_f64)).clamp(0.0, 15.0).round() as usize
}

#[inline]
pub fn to_mm_ss(duration: Duration) -> String {
    format!("{:02}:{:02}", duration.as_secs() / 60, duration.as_secs() % 60)
}

pub fn redraw(
    ui: &mut UI,
    window: &mut pancurses::Window,
    maxx: i32,
    maxy: i32,
    songs: &Songs,
    page: usize,
    local_volume_counter: u8,
    is_search: &String,
    isloop: bool,
    reinit_rpc: bool,
    maxlen: Duration,
    fcalc: Duration,
    fun_index: usize,
    desel: bool,
    sliding: String,
) {
    window.erase();
    window.border(
        ACS_VLINE(),   // sol kenar
        ACS_VLINE(),   // sağ kenar
        ACS_HLINE(),   // üst kenar
        ACS_HLINE(),   // alt kenar
        ACS_ULCORNER(),// sol üst köşe
        ACS_URCORNER(),// sağ üst köşe
        ACS_LLCORNER(),// sol alt köşe
        ACS_LRCORNER() // sağ alt köşe
    );

    ui.cycle();

    // HEADER — Page indicator
    let page_indicator = format!(
        "Page {}/{}",
        page,
        (songs.filtered_songs.len() as f32 / songs.typical_page_size as f32).ceil() as usize
    );
    ui.add(UIElement::new(page_indicator.clone(), maxx - 3 - page_indicator.len() as i32, 0, 0), Part::Header);
    ui.add(UIElement::clickable("< ".to_string(), maxx - 5 - page_indicator.len() as i32, 0, 0, Action::PgUp), Part::Header);
    ui.add(UIElement::clickable(" >".to_string(), maxx - 3, 0, 0, Action::PgDown), Part::Header);
    // HEADER — Search bar
    let search_text = if is_search != "false" {
        format!("Input: {}", is_search)
    } else {
        "Search or edit".to_string()
    };
    ui.add(UIElement::new(search_text, 2, 0, 9), Part::Header);

    // BODY — Şarkı listesi
    let start_index = (page - 1) * songs.typical_page_size;
    let end_index = std::cmp::min(start_index + songs.typical_page_size, songs.filtered_songs.len());
    for (i, song_index) in songs.filtered_songs[start_index..end_index].iter().enumerate() {
        let name = &songs.all_songs[*song_index].name;
        let mut element = UIElement::clickable(name.to_string(), 2, i as i32 + 1, 0, Action::Play(page, i));

        if i == fun_index && !desel {
            element.color = 3;
        }
        if *name == songs.current_name() {
            element.text = format!("{}", element.text); // *
            ui.add(UIElement::new("*".to_string(), element.text.chars().count() as i32 + 3, i as i32 + 1, if songs.stophandler { 4 } else { 1 }), Part::Body);
        } else if songs.is_blacklist(*song_index) {
            element.text = format!("{}", element.text); // bl
            ui.add(UIElement::new("BL".to_string(), element.text.chars().count() as i32 + 3, i as i32 + 1, 2), Part::Body);
        } else if *song_index == songs.get_next() {
            element.text = format!("{}", element.text); // next
            ui.add(UIElement::new("-".to_string(), element.text.chars().count() as i32 + 3, i as i32 + 1, 4), Part::Body);
        }

        ui.add(element, Part::Body);
    }

    // FOOTER — Sliding text
    window.mv(maxy - 5, 0);
    window.addch(ACS_LTEE());
    for _ in 0..(maxx-2) {
        window.addch(ACS_HLINE());
    }
    window.addch(ACS_RTEE());
    ui.add(UIElement::new(sliding, 12, maxy - 4, 1), Part::Footer);

    // FOOTER — Shuffle / Loop / RPC / Volume
    let shuffle_text = format!("{}", if songs.shuffle { "yes" } else { "no" });
    let loop_text = format!("{}", if isloop { "yes" } else { "no" });
    let rpc_text = format!("{}", if reinit_rpc { "no" } else { "yes" });

    ui.add(UIElement::new("Shu".to_string(), 2, maxy - 3, 0), Part::Footer);
    ui.add(UIElement::new("Rep".to_string(), 7, maxy - 3, 0), Part::Footer);
    ui.add(UIElement::new("Rpc".to_string(), maxx - 9, maxy - 3, 0), Part::Footer);
    ui.add(UIElement::new("Vol".to_string(), maxx - 5, maxy - 3, 0), Part::Footer);
    ui.add(UIElement::clickable(shuffle_text, 2, maxy - 2, if songs.shuffle { 1 } else { 2 }, Action::Shuffle), Part::Footer);
    ui.add(UIElement::clickable(loop_text, 7, maxy - 2, if isloop { 1 } else { 2 }, Action::Repeat), Part::Footer);
    ui.add(UIElement::clickable(rpc_text, maxx - 9, maxy - 2, if reinit_rpc { 2 } else { 1 }, Action::Rpc), Part::Footer);
    ui.add(UIElement::new(format!("{}", local_volume_counter), maxx - ((format!("{} ", local_volume_counter)).len() as i32 + 1), maxy - 2, 0), Part::Footer);

    // FOOTER — Progress bar
    let mut start = maxx / 2 - 7;
    for _ in 0..15 {
        window.mv(maxy-3, start);
        window.addch(ACS_HLINE());
        start += 1
    }
    if maxlen != Duration::from_secs(0) {
        let filled = calc(maxlen, fcalc);
        let mut start = maxx / 2 - 7;
        for _ in 0..filled {
            window.mv(maxy-3, start);
            window.attron(COLOR_PAIR(1));
            window.addch(ACS_HLINE());
            window.attroff(COLOR_PAIR(1));
            start += 1
        }
    }
    ui.add(UIElement::new(to_mm_ss(maxlen.checked_sub(fcalc).unwrap_or_default()), maxx/2 - 13, maxy - 3, 0), Part::Footer);
    ui.add(UIElement::new(to_mm_ss(maxlen), maxx/2 + 9, maxy - 3, 0), Part::Footer);
    {
        let artist_name = songs.get_artist_search();
        ui.add(UIElement::new(artist_name.clone(), maxx/2 - artist_name.chars().count()as i32 /2, maxy - 2, 0), Part::Footer);
    }

    // Çizim
    ui.draw_header(window);
    ui.draw_body(window);
    ui.draw_footer(window);
    window.refresh();
}


pub fn init_curses(window: &mut Window) {
    (pancurses::curs_set(0), window.keypad(true), pancurses::noecho(), window.nodelay(true), mousemask(0x2 as u32, None));
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
