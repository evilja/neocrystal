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


struct Instruction {
    target: Target,
    offset: usize,
    length: usize,
    color: ColorIntegerSize,
}

struct InstructionTable {
    blob: Vec<u8>,
    inst: Vec<Instruction>,
}

impl InstructionTable {
    fn execute<I, E>(&self, interface: &mut I)
    where
        E: Execute<I>,
    {
        let base = self.blob.as_ptr();
        for ins in &self.inst {
            unsafe {
                E::cursor(ins.target.0, ins.target.1, interface);
                E::blob(base.add(ins.offset), ins.length, ins.color, interface);
            }
        }
        E::flush(interface);
    }
}


pub struct UI<Id: PartialEq + Copy> {
    width: usize,
    height: usize,
    ownership: Vec<UIElement<Id>>,
    table: InstructionTable,
}

impl<Id: PartialEq + Copy> UI<Id> {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            ownership: Vec::new(),
            table: InstructionTable { blob: Vec::new(), inst: Vec::new()}
        }
    }

    #[allow(dead_code)]
    pub fn get_ownership(&self) -> &Vec<UIElement<Id>> {
        &self.ownership
    }
    pub fn get_range(&self, id: &Id) -> Option<usize> {
        if let Some(a) = self.find(id) {
            return Some(a.range_x.1);
        } else {
            return None;
        }
    }

    fn _alloc(&mut self, id: &Id, rx: Range, ry: Range, cleanup: Option<String>) {
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
    pub fn alloc(&mut self, id: &Id, rx: Range, ry: Range) {
        self._alloc(id, rx, ry, None);
    }
    pub fn c_alloc(&mut self, id: &Id, rx: Range, ry: Range, cleanup: Option<String>) {
        self._alloc(id, rx, ry, cleanup);
    }

    fn find(&self, id: &Id) -> Option<&UIElement<Id>> {
        self.ownership.iter().find(|e| e.id == *id && e.destruct != CleanupOptions::Destructive)
    }

    fn idx(&self, x: usize, y: usize) -> Option<usize> {
        if x < self.width && y < self.height {
            Some(y * self.width + x)
        } else {
            None
        }
    }

    pub fn write(&mut self, id: &Id, mut x: usize, mut y: usize, text: &str, color: ColorIntegerSize) {
        let (this_x, this_y, character) = {
            let o = match self.find(id) {
                Some(o) => o,
                None => return,
            };
            (
                o.range_x,
                o.range_y,
                o.character
                    .as_deref()
                    .map(|s| s.as_bytes())
                    .unwrap_or(&[b' '])
                    .repeat(o.range_x.1),
            )
        };

        x += this_x.0;
        y += this_y.0;

        if self.idx(this_x.0, y).is_none() {
            return;
        }

        let off = self.table.blob.len();
        self.table.blob.extend_from_slice(&character);
        self.table.inst.push(Instruction {
            target: (this_x.0, y),
            offset: off,
            length: character.len(),
            color: 0,
        });

        let off = self.table.blob.len();
        let bytes = text.as_bytes();
        self.table.blob.extend_from_slice(bytes);
        self.table.inst.push(Instruction {
            target: (x, y),
            offset: off,
            length: bytes.len(),
            color,
        });
    }

    pub fn draw<I, E>(&mut self, interface: &mut I)
    where
        E: Execute<I>,
    {
        self.ownership.retain(|e| e.destruct != CleanupOptions::Destructive);

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
    id: T
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
#[allow(dead_code)]
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
