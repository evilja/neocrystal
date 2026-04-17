#[derive(Debug, Clone, Copy)]
pub struct AssColour(u32);

pub struct AssTime {
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
    pub centiseconds: u8,   // ASS uses 0:00:00.00 format, not milliseconds
}

impl AssColour {
    pub fn new(alpha: u8, blue: u8, green: u8, red: u8) -> Self {
        Self(((alpha as u32) << 24) | ((blue as u32) << 16) | ((green as u32) << 8) | (red as u32))
    }
    pub fn opaque_white() -> Self { Self::new(0x00, 0xFF, 0xFF, 0xFF) }
    pub fn transparent() -> Self { Self::new(0xFF, 0x00, 0x00, 0x00) }
    pub fn as_u32(&self) -> u32 { self.0 }
}

impl std::fmt::Display for AssColour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "&H{:08X}", self.0)
    }
}

impl std::str::FromStr for AssColour {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex = s.trim_start_matches("&H");
        Ok(Self(u32::from_str_radix(hex, 16)?))
    }
}

impl std::str::FromStr for AssTime {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.splitn(3, ':').collect();
        if parts.len() != 3 { return Err(()); }
        let sec_cs: Vec<&str> = parts[2].splitn(2, '.').collect();
        if sec_cs.len() != 2 { return Err(()); }
        Ok(AssTime {
            hours:        parts[0].parse().map_err(|_| ())?,
            minutes:      parts[1].parse().map_err(|_| ())?,
            seconds:      sec_cs[0].parse().map_err(|_| ())?,
            centiseconds: sec_cs[1].parse().map_err(|_| ())?,
        })
    }
}

impl std::fmt::Display for AssTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{:02}:{:02}.{:02}", self.hours, self.minutes, self.seconds, self.centiseconds)
    }
}
