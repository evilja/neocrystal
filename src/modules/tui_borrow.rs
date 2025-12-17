#![allow(dead_code)]

use pancurses::{COLOR_PAIR, Window};

type Range = (usize, usize);

#[cfg(target_os = "windows")]
type ColorIntegerSize = u64;

#[cfg(not(target_os = "windows"))]
type ColorIntegerSize = u32;



pub struct UI<T: PartialEq + Copy> {
    width: usize,
    height: usize,
    diff_layout: Vec<(usize, usize, ColorIntegerSize, String)>,
    ownership: Vec<UIElement<T>>,
}



impl<T: PartialEq + Copy> UI<T> {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            diff_layout: Vec::new(),
            ownership: Vec::new(),
        }
    }

    pub fn get_ownership(&self) -> &Vec<UIElement<T>> {
        &self.ownership
    }
    pub fn get_range(&self, id: &T) -> Option<usize> {
        if let Some(a) = self.find(id) {
            return Some(a.range_x.1);
        } else {
            return None;
        }
    }

    fn _alloc(&mut self, id: &T, rx: Range, ry: Range, cleanup: Option<u32>) {
        if rx.0 + rx.1 > self.width || ry.0 + ry.1 > self.height { 
            panic!("tried to allocate ui space more than width or height x start: {} x range: {} y start: {} y range: {}", rx.0, rx.1, ry.0, ry.1);
        }
        for i in &mut self.ownership {
            if i.eq(&id) || in_range_range(rx, ry, i.range_x, i.range_y){
                i.destruct_queue();
            }
        }
        self.ownership.push(UIElement { range_x: rx, range_y: ry, destruct: CleanupOptions::None, id: *id , character: cleanup});
    }
    pub fn alloc(&mut self, id: &T, rx: Range, ry: Range) {
        self._alloc(id, rx, ry, None);
    }
    pub fn c_alloc(&mut self, id: &T, rx: Range, ry: Range, cleanup: Option<u32>) {
        self._alloc(id, rx, ry, cleanup);
    }

    fn find(&self, id: &T) -> Option<&UIElement<T>> {
        self.ownership.iter().find(|e| e.id == *id && e.destruct != CleanupOptions::Destructive)
    }
    fn find_mut(&mut self, id: &T) -> Option<&mut UIElement<T>> {
        self.ownership.iter_mut().find(|e| e.id == *id && e.destruct != CleanupOptions::Destructive)
    }

    fn idx(&self, x: usize, y: usize) -> Option<usize> {
        if x < self.width && y < self.height {
            Some(y * self.width + x)
        } else {
            None
        }
    }

    pub fn write(&mut self, id: &T, mut x: usize, mut y: usize, text: String, color: ColorIntegerSize) {
        let (this_x, this_y) = {
            let o = match self.find_mut(id) {
                Some(o) => o,
                None => return,
            };

            o.destruct_queue_line(o.range_y.0 + y);
            (o.range_x, o.range_y)
        };

        x += this_x.0;
        y += this_y.0;

        let mut s = String::new();
        for (i, ch) in text.chars().enumerate() {
            if !in_range_single(x + i, y, this_x, this_y) { break; }
            if self.idx(x + i, y).is_none() { break; }
            s.push(ch);
        }

        self.diff_layout.push((x, y, color, s));
    }
    pub fn draw(&mut self, window: &mut Window) {
        for e in &mut self.ownership {
            match &e.destruct {
                CleanupOptions::Destructive => {
                    e.cleanup(window);
                }
                CleanupOptions::Linebase(lines) => {
                    for y in lines {
                        e.cleanup_line(window, y);
                    }
                    e.destruct = CleanupOptions::None;
                }
                CleanupOptions::None => {}
            }
        }

        self.ownership.retain(|e| e.destruct != CleanupOptions::Destructive);

        for (x, y, color, string) in self.diff_layout.drain(..) {
            window.attron(COLOR_PAIR(color));
            window.mvaddstr(y as i32, x as i32, string);
            window.attroff(COLOR_PAIR(color));
        }
    }

}

pub struct UIElement<T: PartialEq + Copy> {
    pub range_x: Range,
    pub range_y: Range,
    destruct: CleanupOptions,
    character: Option<u32>,
    id: T
}

#[derive(PartialEq)]
enum CleanupOptions {
    Destructive,
    Linebase(Vec<usize>),
    None,
}

impl<T: PartialEq + Copy> UIElement<T> {
    pub fn get_id(&self) -> &T {
        &self.id
    }
    /// Queues this element for destruction, done in space allocation check.
    pub fn destruct_queue(&mut self) {
        self.destruct = CleanupOptions::Destructive;
    }
    /// Queues line(s) for destruction, done in write function.
    pub fn destruct_queue_line(&mut self, y: usize) {
        match &mut self.destruct {
            CleanupOptions::Linebase(v) => {
                if !v.contains(&y) {
                    v.push(y);
                }
            }
            CleanupOptions::None => {
                self.destruct = CleanupOptions::Linebase(vec![y]);
            }
            CleanupOptions::Destructive => {}
        }
    }

    /// Cleans the characters that this element owns.
    pub fn cleanup(&self, win: &Window) {
        if self.destruct == CleanupOptions::Destructive {
            if let Some(char) = self.character {
                for i in self.range_y.0..self.range_y.0 + self.range_y.1 {
                    win.mv(i as i32, self.range_x.0 as i32);
                    for _ in 0..self.range_x.1 {
                        win.addch(char);
                    }
                }
            } else {
                for i in self.range_y.0..self.range_y.0 + self.range_y.1 {
                    win.mvaddstr(
                        i as i32,
                        self.range_x.0 as i32,
                        &" ".repeat(self.range_x.1)
                    );
                }
            }
        }

    }
    pub fn cleanup_line(&self, win: &Window, y: &usize) {
        if *y >= self.range_y.0 && *y < self.range_y.0 + self.range_y.1 {
            if let Some(char) = self.character {
                win.mv(*y as i32, self.range_x.0 as i32);
                for _ in 0..self.range_x.1 {
                    win.addch(char);
                }
            } else {
                win.mvaddstr(*y as i32, self.range_x.0 as i32, 
                &" ".repeat(self.range_x.1));
            }
        }
    }
}

impl<T: PartialEq + Copy> PartialEq<T> for UIElement<T> {
    fn eq(&self, other: &T) -> bool {
        self.id == *other
    }
}


/// Checks whether a single point (`x`, `y`) lies within this element’s ranges.
///
/// Legend:
/// - `#` = input position
/// - `[]` = this element’s range
///
/// Logic: returns `false` if the point is outside the range on **either axis**.
///
/// X-axis out-of-bounds cases:
/// - `-#-[-----]---`  (point left of range)
/// - `---[-----]-#-`  (point right of range)
///
/// Y-axis out-of-bounds cases:
/// - `-#-[-----]---`
/// - `---[-----]-#-`
pub fn in_range_single(x: usize, y: usize, this_x: Range, this_y: Range) -> bool {
    !(     x < this_x.0
        || x > this_x.0 + this_x.1
        || y < this_y.0
        || y > this_y.0 + this_y.1
    )
}

/// Checks whether this element overlaps with the given X and Y ranges.
///
/// Legend:
/// - `{}` = input ranges
/// - `[]` = this element’s ranges
///
/// Overlap cases checked per axis:
///
/// X-axis:
/// - `--{[}----]---`  (self starts inside input range)
/// - `--{[----}]---`  (self fully inside input range)
/// - `---[{----]---`  (self ends after input range start)
///
/// Y-axis:
/// - `--{[}----]---`
/// - `--{[----}]---`
/// - `---[{----]---`
/// 
#[inline]
fn overlap(a0: usize, a1: usize, b0: usize, b1: usize) -> bool {
    a0 < b1 && b0 < a1
}

pub fn in_range_range(range_x: Range, range_y: Range, this_x: Range, this_y: Range) -> bool {
    overlap(this_x.0, this_x.0 + this_x.1,
    range_x.0, range_x.0 + range_x.1)
    && 
    overlap(this_y.0, this_y.0 + this_y.1,
    range_y.0, range_y.0 + range_y.1)
}
