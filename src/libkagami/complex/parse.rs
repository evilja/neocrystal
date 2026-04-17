use crate::libkagami::complex::overrides::ASSOverride;

pub fn parse_clip_args(inner: &str, is_iclip: bool) -> ASSOverride {
    // Either (drawing_commands) or (scale, drawing_commands)
    let parts: Vec<&str> = inner.splitn(2, ',').collect();
    if parts.len() == 2 {
        if let Ok(scale) = parts[0].trim().parse::<f32>() {
            let drawing = parts[1].trim().to_string();
            return if is_iclip {
                ASSOverride::IclipII(scale, drawing)
            } else {
                ASSOverride::ClipII(scale, drawing)
            };
        }
    }
    let drawing = inner.trim().to_string();
    if is_iclip {
        ASSOverride::IclipI(drawing)
    } else {
        ASSOverride::ClipI(drawing)
    }
}
