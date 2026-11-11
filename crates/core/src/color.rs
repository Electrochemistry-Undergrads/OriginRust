//! Colors, stored as 8-bit RGBA and written to files as `#RRGGBB` or
//! `#RRGGBBAA` text so that project files stay readable.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const RED: Color = Color::rgb(255, 0, 0);
    pub const BLUE: Color = Color::rgb(0, 0, 255);
    pub const TRANSPARENT: Color = Color { r: 0, g: 0, b: 0, a: 0 };

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }

    /// `#RRGGBB` (alpha is not included; use [`Color::opacity`] for that).
    pub fn hex_rgb(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// Opacity from 0.0 (invisible) to 1.0 (solid).
    pub fn opacity(&self) -> f64 {
        f64::from(self.a) / 255.0
    }

    /// Linear blend between two colors; `t = 0` gives `self`, `t = 1` gives `other`.
    pub fn lerp(&self, other: &Color, t: f64) -> Color {
        let t = t.clamp(0.0, 1.0);
        let mix = |a: u8, b: u8| (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round() as u8;
        Color { r: mix(self.r, other.r), g: mix(self.g, other.g), b: mix(self.b, other.b), a: mix(self.a, other.a) }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.a == 255 {
            write!(f, "#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            write!(f, "#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("'{0}' is not a color; expected #RRGGBB, #RRGGBBAA or a basic color name")]
pub struct ParseColorError(pub String);

impl FromStr for Color {
    type Err = ParseColorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let t = s.trim();
        let err = || ParseColorError(s.to_string());
        if let Some(hex) = t.strip_prefix('#') {
            let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2).ok_or_else(err)?, 16).map_err(|_| err());
            return match hex.len() {
                6 => Ok(Color::rgb(byte(0)?, byte(2)?, byte(4)?)),
                8 => Ok(Color::rgba(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
                _ => Err(err()),
            };
        }
        match t.to_ascii_lowercase().as_str() {
            "black" => Ok(Color::BLACK),
            "white" => Ok(Color::WHITE),
            "red" => Ok(Color::RED),
            "green" => Ok(Color::rgb(0, 128, 0)),
            "blue" => Ok(Color::BLUE),
            "cyan" => Ok(Color::rgb(0, 255, 255)),
            "magenta" => Ok(Color::rgb(255, 0, 255)),
            "yellow" => Ok(Color::rgb(255, 255, 0)),
            "orange" => Ok(Color::rgb(255, 128, 0)),
            "purple" => Ok(Color::rgb(128, 0, 128)),
            "gray" | "grey" => Ok(Color::rgb(128, 128, 128)),
            _ => Err(err()),
        }
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        let c: Color = "#46B1E1".parse().unwrap();
        assert_eq!(c, Color::rgb(0x46, 0xB1, 0xE1));
        assert_eq!(c.to_string(), "#46B1E1");
        let t: Color = "#10203040".parse().unwrap();
        assert_eq!(t.to_string(), "#10203040");
        assert!("#12".parse::<Color>().is_err());
        assert_eq!("Red".parse::<Color>().unwrap(), Color::RED);
    }

    #[test]
    fn lerp_endpoints_and_middle() {
        let a = Color::rgb(0, 0, 0);
        let b = Color::rgb(200, 100, 50);
        assert_eq!(a.lerp(&b, 0.0), a);
        assert_eq!(a.lerp(&b, 1.0), b);
        assert_eq!(a.lerp(&b, 0.5), Color::rgb(100, 50, 25));
    }
}
