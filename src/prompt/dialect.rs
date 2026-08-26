//! Prompt dialect profiles (ADR-1115 D9): IR enum → phrase tables loaded from
//! JSON so a new target model needs a new file, not new code.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// The built-in model-agnostic dialect, embedded so the binary needs no files.
pub const GENERIC_DIALECT_JSON: &str = include_str!("../../profiles/prompt/generic.json");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PromptDialect {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub shot_size: BTreeMap<String, String>,
    pub horizontal_angle: BTreeMap<String, String>,
    pub vertical_angle: BTreeMap<String, String>,
    pub coverage_role: BTreeMap<String, String>,
    pub camera_rig: BTreeMap<String, String>,
    pub shot_purpose: BTreeMap<String, String>,
    pub composition_strategy: BTreeMap<String, String>,
    pub screen_region: BTreeMap<String, String>,
    pub depth_role: BTreeMap<String, String>,
    pub transition: BTreeMap<String, String>,
    pub operation: BTreeMap<String, String>,
    pub light_kind: BTreeMap<String, String>,
    pub light_role: BTreeMap<String, String>,
    pub phrases: BTreeMap<String, String>,
}

#[derive(Debug)]
pub enum DialectError {
    Parse(serde_json::Error),
    /// `(table, missing keys)` pairs for every incomplete table.
    Incomplete(Vec<(String, Vec<String>)>),
}

impl fmt::Display for DialectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DialectError::Parse(error) => write!(f, "invalid dialect JSON: {error}"),
            DialectError::Incomplete(missing) => {
                write!(f, "dialect is missing phrases:")?;
                for (table, keys) in missing {
                    write!(f, " {table}[{}]", keys.join(", "))?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for DialectError {}

/// Enumerates the serialized variant names of an enum via its JSON Schema so
/// completeness checks never drift from `model.rs`.
fn variants<T: schemars::JsonSchema>() -> Vec<String> {
    let schema = schemars::schema_for!(T);
    let values = schema.schema.enum_values.unwrap_or_default();
    values
        .into_iter()
        .filter_map(|value| value.as_str().map(str::to_owned))
        .collect()
}

/// Serialized `op` tags of `CameraOperation`, mapped to dialect keys.
pub const OPERATION_KEYS: [&str; 25] = [
    "hold",
    "pan_left",
    "pan_right",
    "tilt_up",
    "tilt_down",
    "roll",
    "dolly_in",
    "dolly_out",
    "truck_left",
    "truck_right",
    "pedestal_up",
    "pedestal_down",
    "translate",
    "rotate",
    "look_at",
    "orbit",
    "follow",
    "crane_up",
    "crane_down",
    "zoom_in",
    "zoom_out",
    "rack_focus",
    "dolly_zoom",
    "handheld_noise",
    "reveal",
];

pub const PHRASE_KEYS: [&str; 23] = [
    "very_shallow_dof",
    "shallow_dof",
    "deep_focus",
    "static",
    "framing",
    "and",
    "looking_at",
    "intent",
    "beat",
    "lighting",
    "lens",
    "over",
    "seconds",
    "then",
    "motivated_by",
    "color_binding",
    "scene",
    "notes",
    "cinematic",
    "while",
    "as",
    "to",
    "fixed_point",
];

impl PromptDialect {
    pub fn generic() -> PromptDialect {
        PromptDialect::from_json(GENERIC_DIALECT_JSON)
            .expect("embedded generic dialect must be valid and complete")
    }

    pub fn from_json(text: &str) -> Result<PromptDialect, DialectError> {
        let dialect: PromptDialect = serde_json::from_str(text).map_err(DialectError::Parse)?;
        dialect.check_complete()?;
        Ok(dialect)
    }

    /// Every enum variant and every phrase key used by the emitter must have
    /// an entry (possibly empty, meaning "say nothing").
    pub fn check_complete(&self) -> Result<(), DialectError> {
        use crate::model::*;
        let mut missing = Vec::new();
        let mut check = |table: &str, map: &BTreeMap<String, String>, keys: Vec<String>| {
            let absent: Vec<String> = keys
                .into_iter()
                .filter(|key| !map.contains_key(key))
                .collect();
            if !absent.is_empty() {
                missing.push((table.to_owned(), absent));
            }
        };
        check("shot_size", &self.shot_size, variants::<ShotSize>());
        check(
            "horizontal_angle",
            &self.horizontal_angle,
            variants::<HorizontalAngle>(),
        );
        check(
            "vertical_angle",
            &self.vertical_angle,
            variants::<VerticalAngle>(),
        );
        check(
            "coverage_role",
            &self.coverage_role,
            variants::<CoverageRole>(),
        );
        check("camera_rig", &self.camera_rig, variants::<CameraRig>());
        check(
            "shot_purpose",
            &self.shot_purpose,
            variants::<ShotPurpose>(),
        );
        check(
            "composition_strategy",
            &self.composition_strategy,
            variants::<CompositionStrategy>(),
        );
        check(
            "screen_region",
            &self.screen_region,
            variants::<ScreenRegion>(),
        );
        check("depth_role", &self.depth_role, variants::<DepthRole>());
        check("transition", &self.transition, variants::<Transition>());
        check("light_kind", &self.light_kind, variants::<LightKind>());
        check("light_role", &self.light_role, variants::<LightRole>());
        check(
            "operation",
            &self.operation,
            OPERATION_KEYS.iter().map(|k| (*k).to_owned()).collect(),
        );
        check(
            "phrases",
            &self.phrases,
            PHRASE_KEYS.iter().map(|k| (*k).to_owned()).collect(),
        );
        if missing.is_empty() {
            Ok(())
        } else {
            Err(DialectError::Incomplete(missing))
        }
    }

    /// Phrase for a serialized enum value; falls back to humanised key so a
    /// stale dialect degrades to readable text instead of panicking.
    pub fn phrase(&self, table: &BTreeMap<String, String>, key: &str) -> String {
        table
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.replace('_', " "))
    }

    pub fn word(&self, key: &str) -> String {
        self.phrase(&self.phrases, key)
    }
}

/// The serde name of an enum value (e.g. `ShotSize::MediumCloseUp` →
/// `"medium_close_up"`).
pub fn serde_name<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(text)) => text,
        Ok(other) => other.to_string().trim_matches('"').to_owned(),
        Err(_) => String::new(),
    }
}
