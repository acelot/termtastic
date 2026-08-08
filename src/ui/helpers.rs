use crate::state::State;
use crate::types::{ChannelRole, Chat, HopsSnrRssiAware, Node};
use base64::prelude::BASE64_STANDARD;
use base64::{DecodeError, Engine};
use chrono::{DateTime, SubsecRound, TimeDelta, Utc};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};
use emoji::Emoji;
use itertools::Itertools;
use meshtastic::protobufs::config;
use meshtastic::protobufs::routing;
use ratatui::style::Modifier;
use ratatui::{
    style::{Color, Style, Stylize},
    symbols::scrollbar::Set as ScrollbarSet,
    text::{Line, Span},
    widgets::{Scrollbar, ScrollbarOrientation},
};
use ratatui_textarea::TextArea;
use regex::{Regex, RegexBuilder};
use std::ops::Add;
use std::sync::LazyLock;
use tui_widget_list::ListState;

static NEWLINE_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\r?\n|\r").unwrap());

static LINK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"[^:/?#\s]+://[^\s]+(?:[\s,.!?;:)\]}>]|$)")
        .multi_line(true)
        .unicode(true)
        .build()
        .unwrap()
});

pub trait BatteryLevelColorExt {
    fn battery_level_to_color(&self) -> Color;
}

impl BatteryLevelColorExt for u32 {
    fn battery_level_to_color(&self) -> Color {
        match self {
            ..20 => Color::Red,
            20..50 => Color::Yellow,
            50.. => Color::Green,
        }
    }
}

pub trait SnrColorExt {
    fn snr_to_color(&self) -> Color;
}

impl SnrColorExt for f32 {
    fn snr_to_color(&self) -> Color {
        match self {
            ..=-10.0 => Color::Red,
            -10.0..=-7.0 => Color::Yellow,
            -7.0.. => Color::Green,
            _ => Color::DarkGray,
        }
    }
}

pub trait RssiColorExt {
    fn rssi_to_color(&self) -> Color;
}

impl RssiColorExt for i32 {
    fn rssi_to_color(&self) -> Color {
        match self {
            ..-100 => Color::Red,
            -100..-85 => Color::Yellow,
            -85.. => Color::Green,
        }
    }
}

pub trait PskLenColorExt {
    fn psk_len_to_color(&self) -> Color;
}

impl PskLenColorExt for usize {
    fn psk_len_to_color(&self) -> Color {
        match self {
            0 => Color::Red,
            1 => Color::Yellow,
            _ => Color::Green,
        }
    }
}

pub trait Base64EncoderExt {
    fn base64_encode(&self) -> String;
}

impl Base64EncoderExt for Vec<u8> {
    fn base64_encode(&self) -> String {
        BASE64_STANDARD.encode(self)
    }
}

pub trait Base64DecoderExt {
    fn base64_decode(&self) -> Result<Vec<u8>, DecodeError>;
}

impl Base64DecoderExt for &str {
    fn base64_decode(&self) -> Result<Vec<u8>, DecodeError> {
        BASE64_STANDARD.decode(self)
    }
}

impl Base64DecoderExt for String {
    fn base64_decode(&self) -> Result<Vec<u8>, DecodeError> {
        BASE64_STANDARD.decode(self)
    }
}

pub trait ModemPresetExt {
    fn as_channel_name(&self) -> String;
}

impl ModemPresetExt for config::lo_ra_config::ModemPreset {
    fn as_channel_name(&self) -> String {
        self.as_str_name()
            .to_lowercase()
            .split('_')
            .map(|w| {
                let mut chars = w.chars();

                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .join("")
    }
}

pub trait StringExt {
    fn to_hyperlinked_lines(&self) -> Vec<Line<'_>>;
}

impl StringExt for String {
    fn to_hyperlinked_lines(&self) -> Vec<Line<'_>> {
        let mut result = Vec::new();

        for line in self.split('\n') {
            let mut spans = Vec::new();
            let mut last_end = 0;

            for mat in LINK_REGEX.find_iter(line) {
                let start = mat.start();
                let end = mat.end();

                if start > last_end {
                    spans.push(Span::raw(&line[last_end..start]).style(Style::new()));
                }

                spans.push(Span::from(&line[start..end]).underlined().blue());
                last_end = end;
            }

            if last_end < line.len() {
                spans.push(Span::from(&line[last_end..]));
            }

            result.push(Line::from(spans));
        }

        result
    }
}

pub trait DateTimeExt {
    fn timestamp_to_datetime(self) -> Option<DateTime<Utc>>;
}

impl DateTimeExt for u32 {
    fn timestamp_to_datetime(self) -> Option<DateTime<Utc>> {
        if self == 0 {
            return None;
        }

        DateTime::from_timestamp_secs(self as i64)
    }
}

pub trait TextAreaExt {
    fn insert_as_lines(&mut self, text: &String);

    fn trimmed_lines(&self) -> Vec<String>;

    fn trimmed_len(&self) -> usize;

    fn get_single_emoji(&self) -> Option<&'static Emoji>;
}

impl TextAreaExt for TextArea<'_> {
    fn insert_as_lines(&mut self, text: &String) {
        for line in trim_lines(NEWLINE_REGEX.split(text)).into_iter() {
            self.insert_str(line);
            self.insert_newline();
        }

        self.delete_newline();
    }

    fn trimmed_lines(&self) -> Vec<String> {
        trim_lines(self.lines().iter())
    }

    fn trimmed_len(&self) -> usize {
        let trimmed = self.trimmed_lines();

        trimmed
            .iter()
            .map(|s| s.len())
            .sum::<usize>()
            .add(trimmed.len())
            .saturating_sub(1)
    }

    fn get_single_emoji(&self) -> Option<&'static Emoji> {
        if self.lines().len() != 1 {
            return None;
        }

        emoji::lookup_by_glyph::lookup(&self.lines()[0])
    }
}

pub fn chat_to_spans<'a>(chat: &'a Chat, state: &'a State) -> Vec<Span<'a>> {
    match chat {
        Chat::Channel(channel_key) => {
            let radio_preset_name = state
                .device_config
                .lora
                .as_ref()
                .and_then(|lora| config::lo_ra_config::ModemPreset::try_from(lora.modem_preset).ok()).map(|preset| preset.as_channel_name());

            let Some(channel) = state.channels.get(channel_key) else {
                return vec![Span::from(format!("!{:x}", channel_key))];
            };

            match channel.role {
                ChannelRole::Primary => vec![
                    Span::from(format!("#{}", &channel.key)).dark_gray(),
                    Span::from(" "),
                    Span::from(if !channel.name.is_empty() {
                        channel.name.clone()
                    } else if let Some(preset_name) = radio_preset_name {
                        preset_name.clone()
                    } else {
                        "Primary".to_owned()
                    })
                    .fg(channel.psk.len().psk_len_to_color()),
                ],
                ChannelRole::Secondary => vec![
                    Span::from(format!("#{}", channel.key)).dark_gray(),
                    Span::from(" "),
                    Span::from(if !channel.name.is_empty() {
                        &channel.name
                    } else {
                        "Secondary"
                    })
                    .fg(channel.psk.len().psk_len_to_color()),
                ],
                ChannelRole::Disabled => {
                    vec![
                        Span::from(format!("#{}", channel.key)).dark_gray(),
                        Span::from(" "),
                        Span::from("Secondary Disabled")
                            .dark_gray()
                            .add_modifier(Modifier::CROSSED_OUT),
                    ]
                }
            }
        }
        Chat::Direct(node_key) => {
            let node = state.nodes.get(node_key);

            match node {
                Some(node) => {
                    vec![
                        short_name_to_span(node, state.is_my_node(node.key)),
                        Span::from(" "),
                        Span::from(node.long_name()),
                    ]
                }
                None => {
                    vec![Span::from(format!("!{:x}", node_key))]
                }
            }
        }
    }
}

pub fn default_scrollbar() -> Scrollbar<'static> {
    Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .symbols(ScrollbarSet {
            begin: "┬",
            thumb: "█",
            track: "│",
            end: "┴",
        })
        .style(Style::new().dark_gray())
}

pub fn pad_center(s: &str, width: usize) -> String {
    let w = unicode_width::UnicodeWidthStr::width(s);
    if w >= width {
        return s.to_string();
    }

    let padding = width - w;
    let left = padding / 2;
    let right = padding - left;

    format!("{}{}{}", " ".repeat(left), s, " ".repeat(right))
}

fn trim_lines<I, S>(lines: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut trimmed: Vec<String> = lines.into_iter().map(|s| s.as_ref().trim_end().to_string()).collect();

    while matches!(trimmed.last(), Some(s) if s.is_empty()) {
        trimmed.pop();
    }

    trimmed
}

pub fn humanize_time_delta<'a>(delta: TimeDelta) -> Vec<Span<'a>> {
    if delta.num_seconds() < 60 {
        return vec![Span::from("now").green()];
    }

    if delta.num_minutes() < 60 {
        return vec![
            Span::from(format!("{}m", delta.num_minutes())).yellow(),
            Span::from(" ago").dark_gray(),
        ];
    }

    if delta.num_hours() < 48 {
        return vec![
            Span::from(format!("{}h", delta.num_hours())),
            Span::from(" ago").dark_gray(),
        ];
    }

    vec![
        Span::from(format!("{}d", delta.num_days())).dark_gray(),
        Span::from(" ago").dark_gray(),
    ]
}

pub fn humanize_uptime<'a>(secs: u32) -> String {
    match secs {
        0..60 => format!("{}s", secs),
        60..3600 => format!("{}m", secs / 60),
        3600..86400 => format!("{}h {}m", secs / 3600, secs % 3600 / 60),
        _ => format!("{}d {}h", secs / 86400, secs % 86400 / 3600),
    }
}

pub fn hops_to_spans<'a>(provider: &impl HopsSnrRssiAware, my: bool) -> Vec<Span<'a>> {
    if my {
        return vec![Span::from("my node").blue()];
    }

    match (provider.hops(), provider.rssi()) {
        (Some(0), None) => {
            vec![Span::from(format!("{:.2}dB", provider.snr())).style(Style::new().fg(provider.snr().snr_to_color()))]
        }
        (Some(0), Some(rssi)) => vec![
            Span::from(format!("{:.2}dB", provider.snr())).style(Style::new().fg(provider.snr().snr_to_color())),
            Span::from("/").dim(),
            Span::from(format!("{}dbm", rssi)).fg(rssi.rssi_to_color()),
        ],
        (Some(1), _) => vec![Span::from("❱").green()],
        (Some(2), _) => vec![Span::from("❱❱").green()],
        (Some(3), _) => vec![Span::from("❱❱❱").green()],
        (Some(4), _) => vec![Span::from("❱❱❱").green(), Span::from("❱").yellow()],
        (Some(5), _) => vec![Span::from("❱❱❱").green(), Span::from("❱❱").yellow()],
        (Some(6), _) => vec![
            Span::from("❱❱❱").green(),
            Span::from("❱❱").yellow(),
            Span::from("❱").red(),
        ],
        (Some(7), _) => vec![
            Span::from("❱❱❱").green(),
            Span::from("❱❱").yellow(),
            Span::from("❱❱").red(),
        ],
        _ => vec![Span::from("unknown").dark_gray()],
    }
}

pub fn short_name_to_span(node: &Node, my: bool) -> Span<'_> {
    Span::from(pad_center(&node.short_name(), 6))
        .black()
        .patch_style(if my {
            Style::new().white().on_blue()
        } else if node.user.is_none() {
            Style::new().on_yellow()
        } else if node.id() == "?" {
            Style::new().on_red()
        } else {
            Style::new().on_green()
        })
}

pub fn last_heard_to_spans(node: &Node, my: bool) -> Vec<Span<'_>> {
    match node.last_heard {
        Some(_) if my => vec![Span::from("now").blue()],
        Some(dt) => humanize_time_delta(Utc::now().round_subsecs(0) - dt),
        None => vec![Span::from("?").dark_gray()],
    }
}

pub fn routing_error_to_span<'a>(error: Option<routing::Error>) -> Span<'a> {
    match error {
        Some(routing::Error::None) => Span::from("ACK").green(),
        Some(e) => Span::from(e.as_str_name()).red(),
        None => Span::from("sent").dark_gray(),
    }
}

pub trait ListStateExt {
    fn handle_navigation_events(&mut self, event: &Event, items_count: Option<usize>) -> bool;

    fn fix_selection(&mut self, items_count: usize);
}

impl ListStateExt for ListState {
    fn handle_navigation_events(&mut self, event: &Event, items_count: Option<usize>) -> bool {
        match event {
            Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                modifiers,
                ..
            }) if modifiers.is_empty() => match code {
                KeyCode::Home => {
                    if let Some(count) = items_count
                        && count > 0
                    {
                        self.select(Some(0));
                        return true;
                    }
                }
                KeyCode::End => {
                    if let Some(count) = items_count
                        && count > 0
                    {
                        self.select(Some(count - 1));
                        return true;
                    }
                }
                KeyCode::Up => {
                    self.previous();
                    return true;
                }
                KeyCode::Down => {
                    self.next();
                    return true;
                }
                _ => {}
            },
            Event::Mouse(MouseEvent { kind, .. }) => match kind {
                MouseEventKind::ScrollUp => {
                    self.previous();
                    return true;
                }
                MouseEventKind::ScrollDown => {
                    self.next();
                    return true;
                }
                _ => {}
            },
            _ => {}
        }

        false
    }

    fn fix_selection(&mut self, items_count: usize) {
        if self.selected.map(|i| i >= items_count).unwrap_or(false) {
            self.selected = None;
        }

        if self.selected.is_none() && items_count > 0 {
            self.select(Some(0));
        }
    }
}
