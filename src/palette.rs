//! Deterministic subject colours (ADR-1115 D10).
//!
//! Every subject in a scene receives a stable, nameable colour used in all
//! conditioning artifacts (2D canvases, Blender ID passes) and in prompts
//! ("Alice (red)"). Colours come from a curated, mutually distinguishable
//! palette, assigned by the subject's rank among the scene's sorted ids so
//! the result depends only on the document.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::model::Scene;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };
    pub const WHITE: Rgb = Rgb {
        r: 255,
        g: 255,
        b: 255,
    };

    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    pub fn unit(self) -> (f32, f32, f32) {
        (
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
        )
    }
}

/// A palette entry with the English colour word prompts use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamedColor {
    pub name: &'static str,
    pub rgb: Rgb,
}

const PALETTE: [NamedColor; 12] = [
    NamedColor {
        name: "red",
        rgb: Rgb {
            r: 220,
            g: 50,
            b: 47,
        },
    },
    NamedColor {
        name: "blue",
        rgb: Rgb {
            r: 38,
            g: 105,
            b: 220,
        },
    },
    NamedColor {
        name: "green",
        rgb: Rgb {
            r: 56,
            g: 160,
            b: 70,
        },
    },
    NamedColor {
        name: "orange",
        rgb: Rgb {
            r: 235,
            g: 140,
            b: 30,
        },
    },
    NamedColor {
        name: "purple",
        rgb: Rgb {
            r: 130,
            g: 70,
            b: 190,
        },
    },
    NamedColor {
        name: "cyan",
        rgb: Rgb {
            r: 40,
            g: 175,
            b: 190,
        },
    },
    NamedColor {
        name: "magenta",
        rgb: Rgb {
            r: 200,
            g: 60,
            b: 160,
        },
    },
    NamedColor {
        name: "yellow",
        rgb: Rgb {
            r: 225,
            g: 195,
            b: 40,
        },
    },
    NamedColor {
        name: "brown",
        rgb: Rgb {
            r: 140,
            g: 90,
            b: 50,
        },
    },
    NamedColor {
        name: "pink",
        rgb: Rgb {
            r: 240,
            g: 130,
            b: 170,
        },
    },
    NamedColor {
        name: "teal",
        rgb: Rgb {
            r: 30,
            g: 130,
            b: 120,
        },
    },
    NamedColor {
        name: "lime",
        rgb: Rgb {
            r: 150,
            g: 200,
            b: 50,
        },
    },
];

/// Colour for the subject ranked `index` in a scene. Beyond the palette size
/// the sequence wraps with a lighter tint so colours stay distinguishable.
pub fn color_for_index(index: usize) -> NamedColor {
    let base = PALETTE[index % PALETTE.len()];
    let round = (index / PALETTE.len()) as u32;
    if round == 0 {
        return base;
    }
    let tint = |channel: u8| -> u8 {
        let c = channel as u32;
        (c + (255 - c) * round.min(3) / 4) as u8
    };
    NamedColor {
        name: base.name,
        rgb: Rgb {
            r: tint(base.rgb.r),
            g: tint(base.rgb.g),
            b: tint(base.rgb.b),
        },
    }
}

/// Stable colour assignment for every subject in `scene`, keyed by subject id.
pub fn scene_palette(scene: &Scene) -> BTreeMap<String, NamedColor> {
    let mut ids: Vec<&str> = scene.subjects.iter().map(|s| s.id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    ids.into_iter()
        .enumerate()
        .map(|(index, id)| (id.to_owned(), color_for_index(index)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_wraps_with_tint() {
        let first = color_for_index(0);
        let wrapped = color_for_index(PALETTE.len());
        assert_eq!(first.name, wrapped.name);
        assert_ne!(first.rgb, wrapped.rgb);
        assert!(wrapped.rgb.r >= first.rgb.r);
    }

    #[test]
    fn hex_formatting() {
        assert_eq!(
            Rgb {
                r: 255,
                g: 0,
                b: 16
            }
            .hex(),
            "#ff0010"
        );
    }
}
