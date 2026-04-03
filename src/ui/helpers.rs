use lazy_static::lazy_static;
use ratatui::{
    style::{Color, Style, Stylize},
    text::{Line, Span},
};
use regex::{Regex, RegexBuilder};

lazy_static! {
    static ref LINK_REGEX: Regex = RegexBuilder::new(r"[^:/?#\s]+://[^\s]+(?:[\s,.!?;:)\]}>]|$)")
        .multi_line(true)
        .unicode(true)
        .build()
        .unwrap();
}

pub trait ColorExt {
    fn snr_to_color(&self) -> Color;
}

impl ColorExt for f32 {
    fn snr_to_color(&self) -> Color {
        match self {
            ..=-10.0 => Color::Red,
            -10.0..=-7.0 => Color::Yellow,
            -7.0.. => Color::Green,
            _ => Color::DarkGray,
        }
    }
}

pub trait LinkExt {
    fn str_to_hyperlinked_lines(value: &str) -> Vec<Line<'_>>;
}

#[allow(dead_code)]
pub fn str_to_hyperlinked_lines(value: &str) -> Vec<Line<'_>> {
    let mut result = Vec::new();

    for line in value.split('\n') {
        let mut spans = Vec::new();
        let mut last_end = 0;

        for mat in LINK_REGEX.find_iter(line) {
            let start = mat.start();
            let end = mat.end();

            if start > last_end {
                spans.push(Span::raw(&line[last_end..start]).style(Style::new()));
            }

            spans.push(Span::from(&line[start..end]).underlined().magenta());
            last_end = end;
        }

        if last_end < line.len() {
            spans.push(Span::from(&line[last_end..]));
        }

        result.push(Line::from(spans));
    }

    result
}
