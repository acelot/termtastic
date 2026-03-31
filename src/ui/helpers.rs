use ratatui::style::Color;

pub fn snr_to_color(value: f32) -> Color {
    match value {
        ..=-10.0 => Color::Red,
        -10.0..=-7.0 => Color::Yellow,
        -7.0.. => Color::Green,
        _ => Color::DarkGray,
    }
}
