//! Colours as `.pen` files write them: `#RGB`, `#RGBA`, `#RRGGBB` or `#RRGGBBAA`.

/// A straight-alpha (not premultiplied) sRGB colour, each channel 0–255.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rgba {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Alpha, 255 opaque.
    pub a: u8,
}

impl Rgba {
    /// Opaque black.
    pub const BLACK: Rgba = Rgba { r: 0, g: 0, b: 0, a: 255 };
    /// Opaque white.
    pub const WHITE: Rgba = Rgba { r: 255, g: 255, b: 255, a: 255 };

    /// Reads a hex colour; `None` when it isn't one.
    #[must_use]
    pub fn parse(text: &str) -> Option<Rgba> {
        let hex = text.trim().strip_prefix('#')?;
        if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let nibble = |i: usize| u8::from_str_radix(hex.get(i..=i)?, 16).ok();
        let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
        // Human: one hex digit doubles (`f` → `ff`), as in CSS.
        let short = |i: usize| nibble(i).map(|n| n * 17);
        match hex.len() {
            3 => Some(Rgba { r: short(0)?, g: short(1)?, b: short(2)?, a: 255 }),
            4 => Some(Rgba { r: short(0)?, g: short(1)?, b: short(2)?, a: short(3)? }),
            6 => Some(Rgba { r: byte(0)?, g: byte(2)?, b: byte(4)?, a: 255 }),
            8 => Some(Rgba { r: byte(0)?, g: byte(2)?, b: byte(4)?, a: byte(6)? }),
            _ => None,
        }
    }

    /// `#rrggbbaa`, the form the editor writes.
    #[must_use]
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
    }

    /// The same colour with its alpha multiplied by `opacity` (0–1).
    #[must_use]
    pub fn with_opacity(self, opacity: f32) -> Rgba {
        Rgba { a: unit_to_byte(f32::from(self.a) / 255.0 * opacity), ..self }
    }

    /// Mixes two colours, `t` = 0 gives `self`, 1 gives `other`.
    #[must_use]
    pub fn lerp(self, other: Rgba, t: f32) -> Rgba {
        let mix = |a: u8, b: u8| unit_to_byte((f32::from(a) + (f32::from(b) - f32::from(a)) * t.clamp(0.0, 1.0)) / 255.0);
        Rgba { r: mix(self.r, other.r), g: mix(self.g, other.g), b: mix(self.b, other.b), a: mix(self.a, other.a) }
    }

    /// WCAG relative luminance (0 black – 1 white).
    #[must_use]
    pub fn luminance(self) -> f32 {
        let lin = |c: u8| {
            let c = f32::from(c) / 255.0;
            if c <= 0.039_28 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * lin(self.r) + 0.7152 * lin(self.g) + 0.0722 * lin(self.b)
    }

    /// WCAG contrast ratio between two colours (1–21).
    #[must_use]
    pub fn contrast(self, other: Rgba) -> f32 {
        let (a, b) = (self.luminance() + 0.05, other.luminance() + 0.05);
        if a > b { a / b } else { b / a }
    }
}

/// 0–1 → 0–255, rounded and clamped.
#[must_use]
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped to 0..=255 before the cast")]
pub fn unit_to_byte(unit: f32) -> u8 {
    (unit.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_hex_form() {
        assert_eq!(Rgba::parse("#fff"), Some(Rgba::WHITE));
        assert_eq!(Rgba::parse("#ff8303ff"), Some(Rgba { r: 255, g: 131, b: 3, a: 255 }));
        assert_eq!(Rgba::parse("#ffffff1a").map(|c| c.a), Some(26));
        assert_eq!(Rgba::parse("$var"), None);
        assert_eq!(Rgba::parse("#12"), None);
        assert_eq!(Rgba { r: 1, g: 2, b: 3, a: 4 }.to_hex(), "#01020304");
    }
}
