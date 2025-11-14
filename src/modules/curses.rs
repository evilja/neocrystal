use pancurses::{Window, mousemask,ACS_VLINE,ACS_HLINE,ACS_ULCORNER, ACS_URCORNER, ACS_LLCORNER, ACS_LRCORNER, ACS_LTEE, ACS_RTEE, COLOR_PAIR};
use std::time::Duration;

use super::songs::Songs;
use std::ffi::CString;
use libc::{setlocale, LC_ALL};
const MAXX: i32 = 50;
const MAXY: i32 = 20;


#[cfg(target_os = "windows")]
type ColorIntegerSize = u64;

#[cfg(not(target_os = "windows"))]
type ColorIntegerSize = u32;












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

    // HEADER — Page indicator
    let page_indicator = format!(
        "Page {}/{}",
        page,
        (songs.filtered_songs.len() as f32 / songs.typical_page_size as f32).ceil() as usize
    );
    ui.add(UIElement::new(page_indicator.clone(), MAXX - 3 - page_indicator.len() as i32, 0, 0));
    ui.add(UIElement::clickable("< ".to_string(), MAXX - 5 - page_indicator.len() as i32, 0, 0, Action::PgUp));
    ui.add(UIElement::clickable(" >".to_string(), MAXX - 3, 0, 0, Action::PgDown));
    // HEADER — Search bar
    let search_text = if is_search != "false" {
        format!("Input: {}", is_search)
    } else {
        "Search or edit".to_string()
    };
    ui.add(UIElement::new(search_text, 2, 0, 9));

    ui.cycle(Part::Body);
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
            ui.add(UIElement::new("*".to_string(), element.text.chars().count() as i32 + 3, i as i32 + 1, if songs.stophandler { 4 } else { 1 }));
        } else if songs.is_blacklist(*song_index) {
            element.text = format!("{}", element.text); // bl
            ui.add(UIElement::new("BL".to_string(), element.text.chars().count() as i32 + 3, i as i32 + 1, 2));
        } else if *song_index == songs.get_next() {
            element.text = format!("{}", element.text); // next
            ui.add(UIElement::new("-".to_string(), element.text.chars().count() as i32 + 3, i as i32 + 1, 4));
        }

        ui.add(element);
    }


    ui.cycle(Part::Footer);


    ui.add(UIElement::new(sliding.clone(), MAXX/2 - sliding.chars().count() as i32/2, MAXY - 4, 1));

    // FOOTER — Shuffle / Loop / RPC / Volume
    let shuffle_text = format!("{}", if songs.shuffle { "yes" } else { "no" });
    let loop_text = format!("{}", if isloop { "yes" } else { "no" });
    let rpc_text = format!("{}", if reinit_rpc { "no" } else { "yes" });

    ui.add(UIElement::new("Shu".to_string(), 2, MAXY - 3, 0));
    ui.add(UIElement::new("Rep".to_string(), 7, MAXY - 3, 0));
    ui.add(UIElement::new("Rpc".to_string(), MAXX - 9, MAXY - 3, 0));
    ui.add(UIElement::new("Vol".to_string(), MAXX - 5, MAXY - 3, 0));
    ui.add(UIElement::clickable(shuffle_text, 2, MAXY - 2, if songs.shuffle { 1 } else { 2 }, Action::Shuffle));
    ui.add(UIElement::clickable(loop_text, 7, MAXY - 2, if isloop { 1 } else { 2 }, Action::Repeat));
    ui.add(UIElement::clickable(rpc_text, MAXX - 9, MAXY - 2, if reinit_rpc { 2 } else { 1 }, Action::Rpc));
    ui.add(UIElement::new(format!("{}", local_volume_counter), MAXX - ((format!("{} ", local_volume_counter)).len() as i32 + 1), MAXY - 2, 0));

    // FOOTER — Progress bar
    ui.add(UIElement::new(to_mm_ss(maxlen.checked_sub(fcalc).unwrap_or_default()), MAXX/2 - 13, MAXY - 3, 0));
    ui.add(UIElement::new(to_mm_ss(maxlen), MAXX/2 + 9, MAXY - 3, 0));
    {
        let artist_name = songs.get_artist_search();
        ui.add(UIElement::new(artist_name.clone(), MAXX/2 - artist_name.chars().count() as i32 /2, MAXY - 2, 0));
        let playlist_name = songs.get_playlist_search();
        ui.add(UIElement::new(playlist_name.clone(), 2, MAXY - 4, 0));
    }

    ui.cycle(Part::Header);


    // Çizim
    ui.draw_wrapper(window, &maxlen, &fcalc);
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

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Action {
    Play(usize, usize),
    Shuffle,
    Repeat,
    Rpc,
    PgDown,
    PgUp,
    Nothing,
}

#[derive(Copy, Clone, PartialEq)]
pub enum Part {
    Header,
    Body,
    Footer,
    _All,
    _None,
}
#[derive(Debug)]
pub struct UIElement {
    pub text: String,
    x: i32,
    y: i32,
    length: i32,
    pub color: u64, // color pair for pancurses
    button: bool,
    action: Action,
}

impl UIElement {
    pub fn new(text: String, x: i32, y: i32, color: u64) -> Self {
        Self {
            text: text.clone(),
            x,
            y,
            length: text.chars().count() as i32,
            color,
            button: false,
            action: Action::Nothing,
        }
    }
    
    pub fn clickable(text: String, x: i32, y: i32, color: u64, action: Action) -> Self {
        Self {
            text: text.clone(),
            x,
            y,
            length: text.chars().count() as i32,
            color,
            button: true,
            action,
        }
    }
    pub fn is_click(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.length && y == self.y && self.button
    }

    fn draw(&self, window: &Window) {
        window.attron(COLOR_PAIR(self.color as ColorIntegerSize));
        window.mvaddstr(self.y as i32, self.x as i32, &self.text);
        window.attroff(COLOR_PAIR(self.color as ColorIntegerSize));
    }
}
pub struct UI {
    pub header_elements: Vec<UIElement>,
    pub body_elements: Vec<UIElement>,
    pub footer_elements: Vec<UIElement>,
    _c_coord_cleanup: Vec<(i32, i32, i32)>,
    _c_redrw_cleanup: Vec<(Part, usize)>,
    _c_index: usize,
    _c_section: Part,
    redraw: bool,
}

impl UI {
    pub fn new() -> Self {
        Self {
            header_elements: vec![],
            body_elements: vec![],
            footer_elements: vec![],
            _c_coord_cleanup: vec![],
            _c_redrw_cleanup: vec![],
            _c_index: 0,
            _c_section: Part::_None,
            redraw: true,
        }
    }
    pub fn cycle(&mut self, to: Part) {
        if self._c_section != to {
            if let Some(target_vec) = match self._c_section {
                Part::Header => Some(&mut self.header_elements),
                Part::Body   => Some(&mut self.body_elements),
                Part::Footer => Some(&mut self.footer_elements),
                _ => None,
            } {
                if target_vec.len() > self._c_index {
                    for i in &target_vec[self._c_index..target_vec.len()] {
                        self._c_coord_cleanup.push((i.x, i.y, i.length));
                    }
                }

                target_vec.truncate(self._c_index);
            }
            self._c_index = 0;
            self._c_section = to;
        }
    }
    pub fn add(&mut self, element: UIElement) {
        let target_vec = match self._c_section {
            Part::Header => &mut self.header_elements,
            Part::Body   => &mut self.body_elements,
            Part::Footer => &mut self.footer_elements,
            _ => return,
        };
        if let Some(existing ) = target_vec.get(self._c_index) {
            if *existing != element {
                self._c_coord_cleanup.push((existing.x, existing.y, existing.length));
                self._c_redrw_cleanup.push((self._c_section, self._c_index));
                target_vec[self._c_index] = element;
                self.redraw = true;
            }
        } else {
            target_vec.push(element);
            self._c_redrw_cleanup.push((self._c_section, self._c_index));
            self.redraw = true;
        }
        self._c_index += 1
    }

    pub fn click(&self, x: i32, y: i32) -> Action {
        for i in &self.header_elements {
            if i.is_click(x, y) {
                return i.action;
            }
        }
        for i in &self.body_elements {
            if i.is_click(x, y) {
                return i.action;
            }
        }
        for i in &self.footer_elements {
            if i.is_click(x, y) {
                return i.action;
            }
        }
        Action::Nothing
    }
    
    pub fn draw_wrapper(&mut self, window: &Window, maxlen: &Duration, fcalc: &Duration) {
        if self.redraw { // _c_cleanup: Vec<(i32, i32, i32, Part, usize)>
            for (x, y, length) in self._c_coord_cleanup.drain(..) {
                if length != 0 {
                    window.mvaddstr(y, x, &" ".repeat(length as usize));
                }
            }
            if self._c_redrw_cleanup.iter().any(|(s, _)| *s == Part::Header) {
                self.draw_const(window);
                for j in &self.header_elements {
                    j.draw(window);
                }
            }       
            for (section, index) in self._c_redrw_cleanup.drain(..) {
                if let Some(target) = match section {
                    Part::Body   => Some(&self.body_elements),
                    Part::Footer => Some(&self.footer_elements),
                    _ => None, 
                } {
                    target[index].draw(window);
                }
            }
            self.draw_essential(window, maxlen, fcalc);
            self.redraw = false;
            window.noutrefresh();
            pancurses::doupdate();

            /*self.draw_helper(window, maxlen, fcalc);
            self.draw_header(window);
            self.draw_body(window);
            self.draw_footer(window);
            self.redraw = false;*/
        }


    }
    pub fn draw_const(&mut self, window: &Window) {
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
        window.mv(MAXY - 5, 0);
        window.addch(ACS_LTEE());
        for _ in 0..(MAXX-2) {
            window.addch(ACS_HLINE());
        }
        window.addch(ACS_RTEE());

    }
    fn draw_essential(&mut self, window: &Window, maxlen: &Duration, fcalc: &Duration) {
        let start = MAXX / 2 - 7;
        window.mv(MAXY-3, start);
        for _ in 0..15 {
            window.addch(ACS_HLINE());
        }
        if *maxlen != Duration::from_secs(0) {
            let filled = calc(*maxlen, *fcalc);
            window.mv(MAXY-3, start);
            for _ in 0..filled {
                window.attron(COLOR_PAIR(1));
                window.addch(ACS_HLINE());
                window.attroff(COLOR_PAIR(1));
            }
        }
    }

}

impl PartialEq for UIElement {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text && self.color == other.color
    }
}
