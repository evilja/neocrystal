use crate::libkagami::complex::overrides::ASSOverride;
use crate::libkagami::complex::helpers::{
    take_parens, parse_bool_val, parse_f32_val, parse_hex_val, parse_csv_f32s,
};
use crate::libkagami::complex::parse::parse_clip_args;

/// Parse the interior of \t(...).
/// Format: [\tag...] | [accel,\tag...] | [t1,t2,\tag...] | [t1,t2,accel,\tag...]
pub fn parse_transform(inner: &str) -> Option<ASSOverride> {
    let backslash_pos = inner.find('\\')?;
    let prefix = &inner[..backslash_pos];
    let style_str = &inner[backslash_pos..];

    let (style_overrides, _) = parse_override_block_content(style_str);

    let nums: Vec<f32> = prefix
        .split(',')
        .filter_map(|p| {
            let p = p.trim();
            if p.is_empty() { None } else { p.parse::<f32>().ok() }
        })
        .collect();

    let ov = match nums.len() {
        0 => ASSOverride::TransformI(style_overrides),
        1 => ASSOverride::TransformII(nums[0], style_overrides),
        2 => ASSOverride::TransformIII(nums[0], nums[1], style_overrides),
        _ => ASSOverride::TransformIV(nums[0], nums[1], nums[2], style_overrides),
    };
    Some(ov)
}

/// Parse one override tag. `s` begins immediately after the leading backslash.
/// Returns (tag, bytes_consumed, is_malformed).
/// is_malformed = true means an unclosed paren was found; caller should drop
/// all subsequent tags in the block.
pub fn parse_one_tag(s: &str) -> Option<(ASSOverride, usize, bool)> {
    let orig_len = s.len();

    macro_rules! consumed {
        ($rest:expr) => { orig_len - $rest.len() };
    }
    macro_rules! try_flag {
        ($prefix:literal, $variant:expr) => {
            if let Some(rest) = s.strip_prefix($prefix) {
                let (val, rest2) = parse_bool_val(rest);
                return Some(($variant(val), consumed!(rest2), false));
            }
        };
    }
    macro_rules! try_f32 {
        ($prefix:literal, $variant:expr) => {
            if let Some(rest) = s.strip_prefix($prefix) {
                let (val, rest2) = parse_f32_val(rest);
                return Some(($variant(val), consumed!(rest2), false));
            }
        };
    }
    macro_rules! try_hex {
        ($prefix:literal, $ctor:expr) => {
            if let Some(rest) = s.strip_prefix($prefix) {
                let (val, rest2) = parse_hex_val(rest);
                return Some(($ctor(val), consumed!(rest2), false));
            }
        };
    }

    // ── \fn — before anything else starting with 'f' ────────────────────────
    if let Some(rest) = s.strip_prefix("fn") {
        let end = rest.find('\\').unwrap_or(rest.len());
        let name = rest[..end].to_string();
        return Some((ASSOverride::Fn(name), consumed!(&rest[end..]), false));
    }

    // ── float tags — longest prefix first ───────────────────────────────────
    try_f32!("xbord", ASSOverride::Xbord);
    try_f32!("ybord", ASSOverride::Ybord);
    try_f32!("xshad", ASSOverride::Xshad);
    try_f32!("yshad", ASSOverride::Yshad);
    try_f32!("fscx",  ASSOverride::Fscx);
    try_f32!("fscy",  ASSOverride::Fscy);
    try_f32!("fsc",   ASSOverride::Fsc);
    try_f32!("fsp",   ASSOverride::Fsp);
    try_f32!("frx",   ASSOverride::Frx);
    try_f32!("fry",   ASSOverride::Fry);
    try_f32!("frz",   ASSOverride::Frz);
    try_f32!("fax",   ASSOverride::Fax);
    try_f32!("fay",   ASSOverride::Fay);
    try_f32!("fe",    ASSOverride::Fe);
    try_f32!("pbo",   ASSOverride::Pbo);
    try_f32!("fr",    ASSOverride::Frz);  // alias — after frx/fry/frz
    try_f32!("blur",  ASSOverride::Blur);
    try_f32!("bord",  ASSOverride::Bord);
    try_f32!("shad",  ASSOverride::Shad);
    try_f32!("be",    ASSOverride::Be);
    try_f32!("fs",    ASSOverride::Fs);

    // ── \an — before \alpha ──────────────────────────────────────────────────
    if let Some(rest) = s.strip_prefix("an") {
        let (val, rest2) = parse_f32_val(rest);
        return Some((ASSOverride::An(val as u8), consumed!(rest2), false));
    }

    // ── \q ───────────────────────────────────────────────────────────────────
    if let Some(rest) = s.strip_prefix("q") {
        let (val, rest2) = parse_f32_val(rest);
        return Some((ASSOverride::Q(val as u8), consumed!(rest2), false));
    }

    // ── \r — consumes until next \ like \fn ──────────────────────────────────
    if let Some(rest) = s.strip_prefix("r") {
        let end = rest.find('\\').unwrap_or(rest.len());
        let name = rest[..end].trim().to_string();
        let tag = ASSOverride::R(if name.is_empty() { None } else { Some(name) });
        return Some((tag, consumed!(&rest[end..]), false));
    }

    // ── alpha / color — longest prefix first ────────────────────────────────
    try_hex!("alpha", ASSOverride::Alpha);
    try_hex!("1a",    ASSOverride::AlphaI);
    try_hex!("2a",    ASSOverride::AlphaII);
    try_hex!("3a",    ASSOverride::AlphaIII);
    try_hex!("4a",    ASSOverride::AlphaIV);
    try_hex!("1c",    ASSOverride::ColorI);
    try_hex!("2c",    ASSOverride::ColorII);
    try_hex!("3c",    ASSOverride::ColorIII);
    try_hex!("4c",    ASSOverride::ColorIV);

    // ── karaoke — ko/kf before k, K uppercase before k ───────────────────────
    if let Some(rest) = s.strip_prefix("ko") {
        let (val, rest2) = parse_f32_val(rest);
        return Some((ASSOverride::Ko(val as u32), consumed!(rest2), false));
    }
    if let Some(rest) = s.strip_prefix("kf") {
        let (val, rest2) = parse_f32_val(rest);
        return Some((ASSOverride::Kf(val as u32), consumed!(rest2), false));
    }
    if let Some(rest) = s.strip_prefix("K") {
        let (val, rest2) = parse_f32_val(rest);
        return Some((ASSOverride::KSweep(val as u32), consumed!(rest2), false));
    }
    if let Some(rest) = s.strip_prefix("k") {
        let (val, rest2) = parse_f32_val(rest);
        return Some((ASSOverride::K(val as u32), consumed!(rest2), false));
    }

    // ── \c — primary color alias, guard against "clip" ───────────────────────
    if s.starts_with('c') && !s.starts_with("clip") {
        let rest = &s[1..];
        let (val, rest2) = parse_hex_val(rest);
        return Some((ASSOverride::ColorI(val), consumed!(rest2), false));
    }

    // ── paren-based tags ─────────────────────────────────────────────────────

    if let Some(rest) = s.strip_prefix("fade") {
        if rest.starts_with('(') {
            match take_parens(rest) {
                None => return Some((ASSOverride::Fade(0.,0.,0.,0.,0.,0.,0.), 0, true)),
                Some((inner, after)) => {
                    let n = parse_csv_f32s(inner);
                    let g = |i: usize| n.get(i).copied().unwrap_or(0.0);
                    return Some((
                        ASSOverride::Fade(g(0),g(1),g(2),g(3),g(4),g(5),g(6)),
                        consumed!(after),
                        false,
                    ));
                }
            }
        }
    }

    if let Some(rest) = s.strip_prefix("move") {
        if rest.starts_with('(') {
            match take_parens(rest) {
                None => return Some((ASSOverride::MoveI(0.,0.,0.,0.), 0, true)),
                Some((inner, after)) => {
                    let n = parse_csv_f32s(inner);
                    let g = |i: usize| n.get(i).copied().unwrap_or(0.0);
                    let tag = if n.len() >= 6 {
                        ASSOverride::MoveII(g(0),g(1),g(2),g(3),g(4),g(5))
                    } else {
                        ASSOverride::MoveI(g(0),g(1),g(2),g(3))
                    };
                    return Some((tag, consumed!(after), false));
                }
            }
        }
    }

    if let Some(rest) = s.strip_prefix("org") {
        if rest.starts_with('(') {
            match take_parens(rest) {
                None => return Some((ASSOverride::Org(0.,0.), 0, true)),
                Some((inner, after)) => {
                    let n = parse_csv_f32s(inner);
                    return Some((
                        ASSOverride::Org(
                            n.get(0).copied().unwrap_or(0.0),
                            n.get(1).copied().unwrap_or(0.0),
                        ),
                        consumed!(after),
                        false,
                    ));
                }
            }
        }
    }

    if let Some(rest) = s.strip_prefix("fad") {
        if rest.starts_with('(') {
            match take_parens(rest) {
                None => return Some((ASSOverride::Fad(0.0, 0.0), 0, true)),
                Some((inner, after)) => {
                    let n = parse_csv_f32s(inner);
                    return Some((
                        ASSOverride::Fad(
                            n.get(0).copied().unwrap_or(0.0),
                            n.get(1).copied().unwrap_or(0.0),
                        ),
                        consumed!(after),
                        false,
                    ));
                }
            }
        }
    }

    if let Some(rest) = s.strip_prefix("pos") {
        if rest.starts_with('(') {
            match take_parens(rest) {
                None => return Some((ASSOverride::Pos(0.0, 0.0), 0, true)),
                Some((inner, after)) => {
                    let n = parse_csv_f32s(inner);
                    return Some((
                        ASSOverride::Pos(
                            n.get(0).copied().unwrap_or(0.0),
                            n.get(1).copied().unwrap_or(0.0),
                        ),
                        consumed!(after),
                        false,
                    ));
                }
            }
        }
    }

    if let Some(rest) = s.strip_prefix("t") {
        if rest.starts_with('(') {
            match take_parens(rest) {
                None => return None,
                Some((inner, after)) => {
                    if let Some(ov) = parse_transform(inner) {
                        return Some((ov, consumed!(after), false));
                    }
                    return None;
                }
            }
        }
    }

    if let Some(rest) = s.strip_prefix("iclip") {
        if rest.starts_with('(') {
            match take_parens(rest) {
                None => return Some((ASSOverride::IclipI(String::new()), 0, true)),
                Some((inner, after)) => {
                    return Some((parse_clip_args(inner, true), consumed!(after), false));
                }
            }
        }
    }

    if let Some(rest) = s.strip_prefix("clip") {
        if rest.starts_with('(') {
            match take_parens(rest) {
                None => return Some((ASSOverride::ClipI(String::new()), 0, true)),
                Some((inner, after)) => {
                    return Some((parse_clip_args(inner, false), consumed!(after), false));
                }
            }
        }
    }

    // ── single-char / short tags — LAST ──────────────────────────────────────
    try_flag!("b", ASSOverride::Bold);
    try_flag!("i", ASSOverride::Italic);
    try_flag!("u", ASSOverride::Underline);
    try_flag!("s", ASSOverride::Strikeout);

    if let Some(rest) = s.strip_prefix("p") {
        let (val, rest2) = parse_f32_val(rest);
        return Some((ASSOverride::P(val as u8), consumed!(rest2), false));
    }

    None
}

/// Parse the content of a `{...}` block (without the surrounding braces).
/// Returns (tags, malformed) where malformed = true means an unclosed paren
/// was encountered and remaining tags were dropped.
pub fn parse_override_block_content(mut s: &str) -> (Vec<ASSOverride>, bool) {
    let mut result = Vec::new();

    loop {
        let bs = match s.find('\\') {
            Some(i) => i,
            None => break,
        };
        s = &s[bs + 1..];

        if s.is_empty() {
            break;
        }

        match parse_one_tag(s) {
            None => {
                let next = s.find('\\').unwrap_or(s.len());
                s = &s[next..];
            }
            Some((tag, consumed, is_malformed)) => {
                result.push(tag);
                if is_malformed {
                    return (result, true);
                }
                s = &s[consumed..];
            }
        }
    }

    (result, false)
}
