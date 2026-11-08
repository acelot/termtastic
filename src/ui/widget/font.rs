use std::{collections::HashMap, sync::LazyLock};

use itertools::Itertools;
use ratatui::prelude::*;

static FONT_MAP: LazyLock<HashMap<char, usize>> = LazyLock::new(|| {
    HashMap::from([
        ('0', 0),
        ('1', 1),
        ('2', 2),
        ('3', 3),
        ('4', 4),
        ('5', 5),
        ('6', 6),
        ('7', 7),
        ('8', 8),
        ('9', 9),
    ])
});

static LARGE_REGULAR: LazyLock<(Vec<Vec<char>>, usize)> = LazyLock::new(|| {
    (
        [
            "╭────╮   ╮  ╶────╮╶────╮╷    ╷╭────╴╭────╴╶────╮╭────╮╭────╮",
            "│    │   │       │     ││    ││     │          ││    ││    │",
            "│    │   │  ╭────╯╶────┤╰────┤╰────╮├────╮     │├────┤╰────┤",
            "│    │   │  │          │     │     ││    │     ││    │     │",
            "╰────╯   ╵  ╰────╴╶────╯     ╵╶────╯╰────╯     ╵╰────╯╶────╯",
        ]
        .map(|l| l.chars().collect())
        .into_iter()
        .collect(),
        6,
    )
});

static LARGE_BOLD: LazyLock<(Vec<Vec<char>>, usize)> = LazyLock::new(|| {
    (
        [
            "┏━━━━┓   ┓  ╺━━━━┓╺━━━━┓╻    ╻┏━━━━╸┏━━━━╸╺━━━━┓┏━━━━┓┏━━━━┓",
            "┃    ┃   ┃       ┃     ┃┃    ┃┃     ┃          ┃┃    ┃┃    ┃",
            "┃    ┃   ┃  ┏━━━━┛╺━━━━┫┗━━━━┫┗━━━━┓┣━━━━┓     ┃┣━━━━┫┗━━━━┫",
            "┃    ┃   ┃  ┃          ┃     ┃     ┃┃    ┃     ┃┃    ┃     ┃",
            "┗━━━━┛   ╹  ┗━━━━╸╺━━━━┛     ╹╺━━━━┛┗━━━━┛     ╹┗━━━━┛╺━━━━┛",
        ]
        .map(|l| l.chars().collect())
        .into_iter()
        .collect(),
        6,
    )
});

static MEDIUM_REGULAR: LazyLock<(Vec<Vec<char>>, usize)> = LazyLock::new(|| {
    (
        [
            "╭──╮  ╮ ╶──╮╶──╮╷  ╷╭──╴╭──╴╶──╮╭──╮╭──╮",
            "│  │  │ ╭──╯╶──┤╰──┤╰──╮├──╮   │├──┤╰──┤",
            "╰──╯  ╵ ╰──╴╶──╯   ╵╶──╯╰──╯   ╵╰──╯╶──╯",
        ]
        .map(|l| l.chars().collect())
        .into_iter()
        .collect(),
        4,
    )
});

static MEDIUM_BOLD: LazyLock<(Vec<Vec<char>>, usize)> = LazyLock::new(|| {
    (
        [
            "┏━━┓  ┓ ╺━━┓╺━━┓╻  ╻┏━━╸┏━━╸╺━━┓┏━━┓┏━━┓",
            "┃  ┃  ┃ ┏━━┛╺━━┫┗━━┫┗━━┓┣━━┓   ┃┣━━┫┗━━┫",
            "┗━━┛  ╹ ┗━━╸╺━━┛   ╹╺━━┛┗━━┛   ╹┗━━┛╺━━┛",
        ]
        .map(|l| l.chars().collect())
        .into_iter()
        .collect(),
        4,
    )
});

pub struct FontWidget<'a> {
    text: &'a str,
    font_lines: &'a Vec<Vec<char>>,
    font_width: usize,
    spacing: usize,
}

#[allow(dead_code)]
impl<'a> FontWidget<'a> {
    pub fn large_regular(text: &'a str, spacing: usize) -> Self {
        let (font_lines, font_width): &(Vec<Vec<char>>, usize) = &LARGE_REGULAR;

        Self {
            text,
            font_lines,
            font_width: *font_width,
            spacing,
        }
    }

    pub fn large_bold(text: &'a str, spacing: usize) -> Self {
        let (font_lines, font_width): &(Vec<Vec<char>>, usize) = &LARGE_BOLD;

        Self {
            text,
            font_lines,
            font_width: *font_width,
            spacing,
        }
    }

    pub fn medium_regular(text: &'a str, spacing: usize) -> Self {
        let (font_lines, font_width): &(Vec<Vec<char>>, usize) = &MEDIUM_REGULAR;

        Self {
            text,
            font_lines,
            font_width: *font_width,
            spacing,
        }
    }

    pub fn medium_bold(text: &'a str, spacing: usize) -> Self {
        let (font_lines, font_width): &(Vec<Vec<char>>, usize) = &MEDIUM_BOLD;

        Self {
            text,
            font_lines,
            font_width: *font_width,
            spacing,
        }
    }

    pub fn size(&self) -> (u16, u16) {
        let chars_count = self.text.chars().count() as u16;

        (
            self.font_width as u16 * chars_count + self.spacing as u16 * (chars_count - 1),
            self.font_lines.len() as u16,
        )
    }
}

impl<'a> Widget for FontWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let text: String = (0..self.font_lines.len())
            .map(|y| {
                self.text
                    .chars()
                    .map(|c| {
                        let x = FONT_MAP[&c] * self.font_width;
                        let chunk: String = self.font_lines[y][x..(x + self.font_width)].into_iter().collect();

                        chunk
                    })
                    .join(&" ".repeat(self.spacing))
            })
            .join("\n");

        Text::from(text).render(area, buf);
    }
}
