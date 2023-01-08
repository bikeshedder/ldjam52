use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Position {
    pub x: i16,
    pub y: i16,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Rect {
    #[serde(flatten)]
    pub position: Position,
    #[serde(flatten)]
    pub size: Size,
}
