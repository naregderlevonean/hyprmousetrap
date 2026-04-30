use hyprland::data::{CursorPosition, Monitor};

#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Zone {
    Top, Bottom, Left, Right,
    TopLeft, TopRight, BottomLeft, BottomRight,
    #[default]
    None,
}

impl Zone {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::Right => "right",
            Self::TopLeft => "top-left",
            Self::TopRight => "top-right",
            Self::BottomLeft => "bottom-left",
            Self::BottomRight => "bottom-right",
            Self::None => "none",
        }
    }
}

pub fn get_logical_size(monitor: &Monitor) -> (i64, i64) {
    let is_rotated = (monitor.transform as u8) % 2 != 0;

    let (raw_w, raw_h) = if is_rotated {
        (monitor.height as f32, monitor.width as f32)
    } else {
        (monitor.width as f32, monitor.height as f32)
    };

    ((raw_w / monitor.scale) as i64, (raw_h / monitor.scale) as i64)
}

pub fn detect_zone(cursor: &CursorPosition, monitor: &Monitor, corner_size: i64, thickness: i64) -> Zone {
    let x = cursor.x - monitor.x as i64;
    let y = cursor.y - monitor.y as i64;

    let (width, height) = get_logical_size(monitor);

    if x < corner_size && y < corner_size { return Zone::TopLeft; }
    if x > width - corner_size && y < corner_size { return Zone::TopRight; }
    if x < corner_size && y > height - corner_size { return Zone::BottomLeft; }
    if x > width - corner_size && y > height - corner_size { return Zone::BottomRight; }

    if y < thickness { return Zone::Top; }
    if y > height - thickness { return Zone::Bottom; }
    if x < thickness { return Zone::Left; }
    if x > width - thickness { return Zone::Right; }

    Zone::None
}
