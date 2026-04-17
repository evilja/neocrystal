use std::{path::PathBuf, time::Duration};
use crate::libkagami::core::SubstationAlpha;
use crate::libkagami::tags::ASSText;

pub struct PreciseSubtitleImport {
    subtitles: SubstationAlpha
}

impl PreciseSubtitleImport {
    pub fn new() -> Self {
            Self {
                subtitles: SubstationAlpha {
                    script_info: crate::libkagami::core::ScriptInfo {
                        title: String::new(),
                        script_type: String::new(),
                        wrap_style: 0,
                        scaled_border_and_shadow: false,
                        ycbcr_matrix: String::new(),
                        playresx: 0,
                        playresy: 0,
                    },
                    v4p_styles: vec![],
                    events: vec![],
                },
            }
        }
    pub fn asyncgate(&mut self, path: &str) {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.load_subtitles(path));
    }

    pub async fn load_subtitles(&mut self, path: &str) {
        self.subtitles = SubstationAlpha::load(PathBuf::from(path), false).await;
    }
    pub fn get_from_time(&self, time: Duration) -> Option<String> {
        let time_secs = time.as_secs_f64();

        let event = self.subtitles.events.iter().find(|e| {
            let start = ass_time_to_secs(&e.start);
            let end   = ass_time_to_secs(&e.end);
            time_secs >= start && time_secs < end
        })?;

        let text: String = event.text.data.iter()
            .filter_map(|node| match node {
                ASSText::RawText(s) => Some(s.as_str()),
                ASSText::Override(_) => None,
            })
            .collect();

        if text.is_empty() { None } else { Some(text) }
    }
}

fn ass_time_to_secs(t: &crate::libkagami::complex::types::AssTime) -> f64 {
    t.hours   as f64 * 3600.0
    + t.minutes as f64 * 60.0
    + t.seconds as f64
    + t.centiseconds as f64 / 100.0
}
