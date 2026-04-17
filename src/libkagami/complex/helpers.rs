pub fn take_parens(s: &str) -> Option<(&str, &str)> {
    debug_assert!(s.starts_with('('));
    let inner_start = 1;
    let mut depth = 1usize;
    for (i, c) in s[inner_start..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    let inner = &s[inner_start..inner_start + i];
                    let rest = &s[inner_start + i + 1..];
                    return Some((inner, rest));
                }
            }
            _ => {}
        }
    }
    None // unclosed
}

/// Read a boolean-ish integer (0/1).
/// Invariant: uppercase first char → false (treat as 0).
pub fn parse_bool_val(s: &str) -> (bool, &str) {
    if s.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
        return (false, s);
    }
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    if end == 0 {
        return (true, s);
    }
    let val = s[..end].parse::<u32>().unwrap_or(0) != 0;
    (val, &s[end..])
}

/// Read one f32.
/// Invariant: uppercase first char → 0, skip to next tag.
/// Invariant: space-separated extra values after the first are dropped.
pub fn parse_f32_val(s: &str) -> (f32, &str) {
    let s = s.trim_start_matches(' ');

    if s.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
        let end = s.find('\\').unwrap_or(s.len());
        return (0.0, &s[end..]);
    }

    let end = s
        .find(|c: char| c != '-' && c != '+' && c != '.' && !c.is_ascii_digit())
        .unwrap_or(s.len());
    let val = s[..end].parse::<f32>().unwrap_or(0.0);

    let rest = &s[end..];
    let rest = skip_to_next_tag(rest);
    (val, rest)
}

/// After consuming a value, if there is non-backslash/non-brace content remaining
/// (e.g. a second space-separated number), skip past it to the next tag boundary.
pub fn skip_to_next_tag(s: &str) -> &str {
    let trimmed = s.trim_start_matches(' ');
    if trimmed.is_empty() || trimmed.starts_with('\\') || trimmed.starts_with('}') {
        return s;
    }
    s.find('\\').map(|i| &s[i..]).unwrap_or(&s[s.len()..])
}

/// Parse a &HXXXXXXXX& hex value. Returns (value, rest_of_string).
pub fn parse_hex_val(s: &str) -> (u32, &str) {
    let s_trimmed = s.trim_start_matches(' ');
    let leading_spaces = s.len() - s_trimmed.len();

    let s2 = match s_trimmed.strip_prefix('&') {
        Some(r) => r,
        None => return (0, s),
    };
    let s3 = if s2.starts_with('H') || s2.starts_with('h') {
        &s2[1..]
    } else {
        s2
    };
    let end = s3
        .find(|c: char| !c.is_ascii_hexdigit())
        .unwrap_or(s3.len());
    let val = u32::from_str_radix(&s3[..end], 16).unwrap_or(0);
    let _rest = s3[end..].strip_prefix('&').unwrap_or(&s3[end..]);
    let consumed = leading_spaces + 1 + 1 + end + (s3[end..].starts_with('&') as usize);
    (val, &s[consumed..])
}

pub fn parse_csv_f32s(s: &str) -> Vec<f32> {
    s.split(',')
        .filter_map(|p| p.trim().parse::<f32>().ok())
        .collect()
}
