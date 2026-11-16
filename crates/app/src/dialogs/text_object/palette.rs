//! A bounded subset of the published Origin Classic text palette (graphing 7.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TextColor {
    Black,
    Red,
    Green,
    Blue,
    Cyan,
    Magenta,
    Yellow,
    White,
}

impl TextColor {
    pub const ALL: [Self; 8] =
        [Self::Black, Self::Red, Self::Green, Self::Blue, Self::Cyan, Self::Magenta, Self::Yellow, Self::White];

    pub fn label(self) -> &'static str {
        match self {
            Self::Black => "Black",
            Self::Red => "Red",
            Self::Green => "Green",
            Self::Blue => "Blue",
            Self::Cyan => "Cyan",
            Self::Magenta => "Magenta",
            Self::Yellow => "Yellow",
            Self::White => "White",
        }
    }

    pub fn prefix(self) -> &'static str {
        match self {
            Self::Black => r"\c1(",
            Self::Red => r"\c2(",
            Self::Green => r"\c3(",
            Self::Blue => r"\c4(",
            Self::Cyan => r"\c5(",
            Self::Magenta => r"\c6(",
            Self::Yellow => r"\c7(",
            Self::White => r"\c18(",
        }
    }
}
