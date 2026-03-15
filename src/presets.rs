use serde::Deserialize;

#[derive(Clone, PartialEq, Deserialize)]
pub struct Preset {
    pub name: String,
    pub code: String,
}

/// Fallback when presets.json is not available
pub fn default_presets() -> Vec<Preset> {
    vec![
        Preset { name: "Basic beat".into(), code: "bd sd bd sd hh hh hh hh".into() },
        Preset { name: "Four on the floor".into(), code: "bd bd bd bd hh*4 hh*4 hh*4 hh*4 sd sd".into() },
        Preset { name: "Simple (bd sd hh)".into(), code: "bd sd hh".into() },
    ]
}
