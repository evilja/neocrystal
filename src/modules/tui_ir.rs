#![allow(dead_code)]

/// IR (Intermediate Representation) for TUI.
/// It is backend agnostic, neocrystal uses ncurses to back it.
/// You can use it with your own terminal tool
///
/// You need a struct that implements Execute<I> trait. I is your preferred backend like ncurses.
/// You can pass something like a ncurses window, and use window.mv in cursor, window.addstr in blob, and pancurses' flush functions in flush
/// in that trait:
///  cursor() -> move to x and y, using I
///  blob() -> print starting at *const u8 ptr, ending at ptr+len, with color ColorIntegerSize, using I
///  flush() -> flush the terminal, using I
/// My implementation of Execute<I>
///    impl Execute<Window> for NcursesExec {
///        fn cursor(x: usize, y: usize, w: &mut Window) {
///            w.mv(y as i32, x as i32);
///        }
///
///        fn blob(ptr: *const u8, len: usize, color: ColorIntegerSize, w: &mut Window) {
///            w.attron(COLOR_PAIR(color));
///            unsafe {
///                let bytes = std::slice::from_raw_parts(ptr, len);
///                let s = std::str::from_utf8_unchecked(bytes);
///                w.addstr(s);
///            }
///            w.attroff(COLOR_PAIR(color));
///        }
///
///        fn flush(w: &mut Window) {
///            w.refresh();
///        }
///    }
/// Then you'll need an ownership identifier. It can be anything as long as it implements PartialEq and Copy traits.
/// My ownership:
/// #[derive(Copy, Clone, PartialEq)]
/// enum Ownership {
///     Abc,
///     Bca
/// }
/// After these, you'll need to allocate regions on your terminal using UI's alloc or c_alloc functions
/// alloc is regular allocator
/// c_alloc is regular allocator but you can choose what character is used when cleaning up.
/// allocation is not coordinate based. Range (usize, usize) is basically start, length
/// like ui.alloc(&Ownership::Time1, (12, 5), (17, 1));
/// This means "Allocate X 12 range 5 and Y 17 range 1 for Ownership::Time1"
/// Ownership is passed as reference so you can use giant structs if you want.
///
/// write_* functions clean up the entire row before writing.
/// inject_* functions do not clean up, and they do not respect ownership.
/// Different ownerships can't write to other ownerships' regions using write_* functions.
///
/// sim* functions (write_simy, inject_simx) (means single instruction multiple *)
/// simx functions are not real at execute level. they get replaced to full blobs before getting turned into instructions.
/// simy functions are real. when executing, they get executed N times with Y value row + N, N starting at 0
///
/// also, write_* are relative to allocation region. If you allocated X 17, and gave X 0 to write; X 0 is actually X 17
/// but inject_* are not relative. they are just positions.
///
/// there's a "dev" feature to let you see the instructions. they are at /tmp/instructions.log
/// a SI instruction:
/// 0002: Instruction SI
///       goto 12 17
///       attr 0
///       byte 57 5
///       bytes: [30, 30, 3A, 35, 36]
/// a SIM instruction:
/// 0008: Instruction SIM(18)
///       goto 49 1
///       attr 0
///       byte 447 3
///       bytes: [E2, 94, 82]
///
extern crate unicode_width;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

type Range = (usize, usize);
type Target = (usize, usize);

#[cfg(target_os = "windows")]
pub type ColorIntegerSize = u64;

#[cfg(not(target_os = "windows"))]
pub type ColorIntegerSize = u32;

pub trait Execute<I> {
    fn cursor(x: usize, y: usize, interface: &mut I);
    fn blob(ptr: *const u8, len: usize, color: ColorIntegerSize, interface: &mut I);
    fn flush(interface: &mut I);
}
#[derive(Debug)]
enum InstructionModifier {
    SI, // Single Instruction Single Operation
    SIM(usize), // Single Instruction Multiple Operations
        // more, like Multiple Instruction Single Line etc... to concat more than one instruction
}

struct Instruction {
    target: Target,
    offset: usize,
    length: usize,
    color: ColorIntegerSize,
    modifier: InstructionModifier,
}

struct InstructionTable {
    blob: Vec<u8>,
    inst: Vec<Instruction>,
}

impl InstructionTable {
    fn new() -> Self {
        Self {
            blob: Vec::new(),
            inst: Vec::new(),
        }
    }

    fn execute<I, E>(&self, interface: &mut I)
    where
        E: Execute<I>,
    {
        let base: *const u8 = self.blob.as_ptr();
        for ins in &self.inst {
            unsafe {
                match ins.modifier {
                    InstructionModifier::SI => {
                        E::cursor(ins.target.0, ins.target.1, interface);
                        E::blob(base.add(ins.offset), ins.length, ins.color, interface);
                    }
                    InstructionModifier::SIM(l) => {
                        for i in 0..l {
                            E::cursor(ins.target.0, ins.target.1 + i, interface);
                            E::blob(base.add(ins.offset), ins.length, ins.color, interface);
                        }
                    }
                }
            }
        }
        E::flush(interface);
    }

    pub fn si_blob(&mut self, into: &[u8], x: usize, y: usize, color: ColorIntegerSize) {
        self.inst.push(Instruction {
            target: (x, y),
            offset: self.blob.len(),
            length: into.len(),
            color,
            modifier: InstructionModifier::SI,
        });
        self.blob.extend_from_slice(into);
    }

    pub fn sim_blob(&mut self, into: &[u8], x: usize, y: usize, color: ColorIntegerSize, l: usize) {
        self.inst.push(Instruction {
            target: (x, y),
            offset: self.blob.len(),
            length: into.len(),
            color,
            modifier: InstructionModifier::SIM(l),
        });
        self.blob.extend_from_slice(into);
    }

    pub fn fake_sim(&mut self, into: &[u8], x: usize, y: usize, color: ColorIntegerSize, l: usize) {
        self.inst.push(Instruction {
            target: (x, y),
            offset: self.blob.len(),
            length: into.len() * l,
            color,
            modifier: InstructionModifier::SI,
        });
        for _ in 0..l {
            self.blob.extend_from_slice(into);
        }
    }
}

pub struct UI<Id: PartialEq + Copy> {
    width: usize,
    height: usize,
    ownership: Vec<UIElement<Id>>,
    table: InstructionTable,
}

struct PreparedRegion {
    range_x: Range,
    range_y: Range,
    cleanup_bytes: Vec<u8>,
    cleanup_width: usize,
}

impl<Id: PartialEq + Copy> UI<Id> {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            ownership: Vec::new(),
            table: InstructionTable::new(),
        }
    }

    #[allow(dead_code)]
    pub fn get_ownership(&self) -> &Vec<UIElement<Id>> {
        &self.ownership
    }
    pub fn get_range(&self, id: &Id) -> Option<usize> {
        self.find(id).map(|a| a.range_x.1)
    }

    fn _alloc(&mut self, id: &Id, rx: Range, ry: Range, cleanup: Option<String>) {
        if rx.0 + rx.1 > self.width || ry.0 + ry.1 > self.height {
            panic!(
                "tried to allocate ui space more than width or height x start: {} x range: {} y start: {} y range: {}",
                rx.0, rx.1, ry.0, ry.1
            );
        }
        for i in &mut self.ownership {
            if i.eq(&id) || in_range_range(rx, ry, i.range_x, i.range_y) {
                i.destruct_queue();
            }
        }
        self.ownership.push(UIElement {
            range_x: rx,
            range_y: ry,
            destruct: CleanupOptions::None,
            character: cleanup,
            id: *id,
        });
    }

    pub fn alloc(&mut self, id: &Id, rx: Range, ry: Range) {
        self._alloc(id, rx, ry, None);
    }

    pub fn c_alloc(&mut self, id: &Id, rx: Range, ry: Range, cleanup: Option<String>) {
        self._alloc(id, rx, ry, cleanup);
    }

    pub fn drop_ownership(&mut self, id: &Id) {
        for i in &mut self.ownership {
            if i.get_id() == id {
                i.destruct_queue();
            }
        }
    }

    fn find(&self, id: &Id) -> Option<&UIElement<Id>> {
        self.ownership
            .iter()
            .find(|e| e.id == *id && e.destruct != CleanupOptions::Destructive)
    }

    fn idx(&self, x: usize, y: usize) -> Option<usize> {
        if x < self.width && y < self.height {
            Some(y * self.width + x)
        } else {
            None
        }
    }

    fn prepare_region(&self, id: &Id) -> Option<PreparedRegion> {
        let o = self.find(id)?;
        let cleanup = o.character.as_deref().unwrap_or(" ");

        Some(PreparedRegion {
            range_x: o.range_x,
            range_y: o.range_y,
            cleanup_bytes: cleanup.as_bytes().to_vec(),
            cleanup_width: UnicodeWidthStr::width(cleanup),
        })
    }

    fn fill_cleanup_row(
        &mut self,
        region: &PreparedRegion,
        y: usize,
        text_width: usize,
        fill_from_start: bool,
    ) {
        if region.cleanup_width == 0 {
            return;
        }

        let reps = if fill_from_start {
            region.range_x.1 / region.cleanup_width
        } else {
            region.range_x.1.saturating_sub(text_width) / region.cleanup_width
        };

        if reps == 0 {
            return;
        }

        let fill_x = if fill_from_start {
            region.range_x.0
        } else {
            region.range_x.0 + text_width
        };

        self.table
            .fake_sim(&region.cleanup_bytes, fill_x, y, 0, reps);
    }

    pub fn empty_instruction(&mut self, id: &Id, y: usize) {
        let region = match self.prepare_region(id) {
            Some(region) => region,
            None => return,
        };
        let y = y + region.range_y.0;

        if self.idx(region.range_x.0, y).is_none() {
            return;
        }
        let character = region.cleanup_bytes.repeat(region.range_x.1);
        self.table.si_blob(&character, region.range_x.0, y, 0);
    }

    pub fn write(
        &mut self,
        id: &Id,
        mut x: usize,
        mut y: usize,
        text: &str,
        color: ColorIntegerSize,
    ) {
        let w = UnicodeWidthStr::width(text);
        let b = text.as_bytes();

        let region = match self.prepare_region(id) {
            Some(region) => region,
            None => return,
        };
        x += region.range_x.0;
        y += region.range_y.0;

        if self.idx(region.range_x.0, y).is_none() {
            return;
        }

        if !blob_fit(x, w, region.range_x) {
            return;
        }

        self.fill_cleanup_row(&region, y, w, x != region.range_x.0);

        if b.is_empty() {
            return;
        }
        self.table.si_blob(b, x, y, color);
    }

    pub fn write_shimmer(
        &mut self,
        id: &Id,
        mut x: usize,
        mut y: usize,
        text: &str,
        base_color: ColorIntegerSize,
        mid_color: ColorIntegerSize,
        highlight_color: ColorIntegerSize,
        phase: usize,
    ) {
        let w = UnicodeWidthStr::width(text);

        let region = match self.prepare_region(id) {
            Some(region) => region,
            None => return,
        };
        x += region.range_x.0;
        y += region.range_y.0;

        if self.idx(region.range_x.0, y).is_none() {
            return;
        }

        if !blob_fit(x, w, region.range_x) {
            return;
        }

        self.fill_cleanup_row(&region, y, w, x != region.range_x.0);

        if text.is_empty() {
            return;
        }

        let highlight = phase % (w + 6);
        let mut cell = 0;
        for grapheme in text.graphemes(true) {
            let grapheme_width = UnicodeWidthStr::width(grapheme).max(1);
            let end = cell + grapheme_width - 1;
            let distance = if highlight < cell {
                cell - highlight
            } else if highlight > end {
                highlight - end
            } else {
                0
            };
            let color = match distance {
                0 => highlight_color,
                1 => mid_color,
                _ => base_color,
            };

            self.table.si_blob(grapheme.as_bytes(), x + cell, y, color);
            cell += UnicodeWidthStr::width(grapheme);
        }
    }

    pub fn write_simy(
        &mut self,
        id: &Id,
        mut x: usize,
        mut y: usize,
        text: &str,
        color: ColorIntegerSize,
        l: usize,
    ) {
        let w = UnicodeWidthStr::width(text);
        let b = text.as_bytes();

        let region = match self.prepare_region(id) {
            Some(region) => region,
            None => return,
        };

        x += region.range_x.0;
        y += region.range_y.0;

        if self.idx(region.range_x.0, y).is_none() {
            return;
        }
        if !blob_fit(x, w, region.range_x) || !blob_fit(y, l, region.range_y) {
            return;
        }

        self.fill_cleanup_row(&region, y, w, x != region.range_x.0);

        if l == 0 {
            return;
        }

        self.table.sim_blob(b, x, y, color, l);
    }

    pub fn write_simx(
        &mut self,
        id: &Id,
        mut x: usize,
        mut y: usize,
        text: &str,
        color: ColorIntegerSize,
        l: usize,
    ) {
        let w = UnicodeWidthStr::width(text) * l;
        let b = text.as_bytes();

        let region = match self.prepare_region(id) {
            Some(region) => region,
            None => return,
        };

        x += region.range_x.0;
        y += region.range_y.0;

        if self.idx(region.range_x.0, y).is_none() {
            return;
        }
        if !blob_fit(x, w, region.range_x) {
            return;
        }

        self.fill_cleanup_row(&region, y, w, x != region.range_x.0);
        if l == 0 {
            return;
        }
        self.table.fake_sim(b, x, y, color, l);
    }

    /// This function does not participate in the UI logic.
    /// It is for when you don't want to allocate at all because
    /// you'll use it only once. Like drawing borders.
    /// You don't need to respect borders, I don't as well in my app.
    /// I have page indicators and search texts over borders in neocrystal.
    pub fn inject_si(&mut self, x: usize, y: usize, text: &str, color: ColorIntegerSize) {
        let bytes = text.as_bytes();
        if bytes.is_empty() {
            return;
        }

        self.table.si_blob(bytes, x, y, color);
    }

    pub fn inject_simx(
        &mut self,
        x: usize,
        y: usize,
        text: &str,
        color: ColorIntegerSize,
        l: usize,
    ) {
        let bytes = text.as_bytes();
        if bytes.is_empty() {
            return;
        }
        self.table.fake_sim(bytes, x, y, color, l);
    }

    pub fn inject_simy(
        &mut self,
        x: usize,
        y: usize,
        text: &str,
        color: ColorIntegerSize,
        l: usize,
    ) {
        let bytes = text.as_bytes();
        if bytes.is_empty() {
            return;
        }
        self.table.sim_blob(bytes, x, y, color, l);
    }

    pub fn inject_simyx(
        &mut self,
        x: usize,
        y: usize,
        text: &str,
        color: ColorIntegerSize,
        l: usize,
        l2: usize,
    ) {
        let bytes = text.as_bytes();
        if bytes.is_empty() {
            return;
        }
        let off = self.table.blob.len();
        for _ in 0..l2 {
            self.table.blob.extend_from_slice(bytes);
        }

        self.table.inst.push(Instruction {
            target: (x, y),
            offset: off,
            length: bytes.len() * l2,
            color,
            modifier: InstructionModifier::SIM(l),
        });
    }

    pub fn draw<I, E>(&mut self, interface: &mut I)
    where
        E: Execute<I>,
    {
        self.ownership
            .retain(|e| e.destruct != CleanupOptions::Destructive);
        #[cfg(feature = "dev")]
        self.table
            .dump_to_file("/home/myisha/tui_instructions.log")
            .unwrap();
        // execute instructions on the passed interface
        self.table.execute::<I, E>(interface);

        // important: clear program for next frame
        self.table.inst.clear();
        self.table.blob.clear();
    }
}

pub struct UIElement<T: PartialEq + Copy> {
    pub range_x: Range,
    pub range_y: Range,
    destruct: CleanupOptions,
    character: Option<String>,
    id: T,
}

#[derive(PartialEq)]
enum CleanupOptions {
    Destructive,
    None,
}

impl<T: PartialEq + Copy> UIElement<T> {
    /// Queues this element for destruction, done in space allocation check.
    pub fn destruct_queue(&mut self) {
        self.destruct = CleanupOptions::Destructive;
    }
    pub fn get_id(&self) -> &T {
        &self.id
    }
}

impl<T: PartialEq + Copy> PartialEq<T> for UIElement<T> {
    fn eq(&self, other: &T) -> bool {
        self.id == *other
    }
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
    overlap(
        this_x.0,
        this_x.0 + this_x.1,
        range_x.0,
        range_x.0 + range_x.1,
    ) && overlap(
        this_y.0,
        this_y.0 + this_y.1,
        range_y.0,
        range_y.0 + range_y.1,
    )
}

#[inline]
fn blob_fit(reference: usize, blob_len: usize, range: Range) -> bool {
    reference >= range.0 && reference + blob_len <= range.0 + range.1
}

#[cfg(feature = "dev")]
use std::fmt;
#[cfg(feature = "dev")]
impl fmt::Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Instruction {:?}\n      goto {} {}\n      attr {}\n      byte {} {}",
            self.modifier, self.target.0, self.target.1, self.color, self.offset, self.length
        )
    }
}
#[cfg(feature = "dev")]
use std::fs::File;
#[cfg(feature = "dev")]
use std::io::Read;
#[cfg(feature = "dev")]
use std::io::{BufWriter, Write};
#[cfg(feature = "dev")]
impl InstructionTable {
    pub fn dump_to_file(&self, path: &str) -> std::io::Result<()> {
        let file = File::create(path)?;
        let mut w = BufWriter::new(file);

        writeln!(w, "=== Instruction dump ===")?;
        writeln!(w, "blob size: {} bytes", self.blob.len())?;
        writeln!(w, "instruction count: {}", self.inst.len())?;
        writeln!(w)?;

        for (i, ins) in self.inst.iter().enumerate() {
            writeln!(w, "{:04}: {:?}", i, ins)?;

            // optional: dump bytes
            let start = ins.offset;
            let bytes = &self.blob[start..start + ins.length];

            writeln!(w, "      bytes: {:02X?}", bytes)?;
        }

        Ok(())
    }
}
