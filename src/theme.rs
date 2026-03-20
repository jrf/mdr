use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub border: Color,
    pub accent: Color,
    pub text: Color,
    pub text_bright: Color,
    pub text_dim: Color,
    pub text_muted: Color,
    pub heading: Color,
    pub error: Color,
    pub cursor_bg: Color,
    pub labels: CategoryLabels,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CategoryLabels {
    pub bugs: Color,
    pub features: Color,
    pub improvements: Color,
    pub refactor: Color,
    pub docs: Color,
    pub chore: Color,
    pub data: Color,
    pub model: Color,
    pub experiment: Color,
}

/// Fallback theme (tokyo night moon in RGB) used when config has no themes.
pub fn default_theme() -> Theme {
    Theme {
        border: Color::Rgb(59, 66, 97),       // #3b4261
        accent: Color::Rgb(192, 153, 255),     // #c099ff
        text: Color::Rgb(200, 211, 245),       // #c8d3f5
        text_bright: Color::Rgb(213, 223, 245),// #d5dff5
        text_dim: Color::Rgb(99, 109, 166),    // #636da6
        text_muted: Color::Rgb(59, 66, 97),    // #3b4261
        heading: Color::Rgb(130, 170, 255),    // #82aaff
        error: Color::Rgb(255, 117, 127),      // #ff757f
        cursor_bg: Color::Rgb(47, 51, 77),     // #2f334d
        labels: CategoryLabels {
            bugs: Color::Rgb(255, 117, 127),       // #ff757f
            features: Color::Rgb(195, 232, 141),   // #c3e88d
            improvements: Color::Rgb(192, 153, 255),// #c099ff
            refactor: Color::Rgb(255, 199, 119),   // #ffc777
            docs: Color::Rgb(130, 170, 255),       // #82aaff
            chore: Color::Rgb(99, 109, 166),       // #636da6
            data: Color::Rgb(79, 214, 190),        // #4fd6be
            model: Color::Rgb(252, 167, 234),      // #fca7ea
            experiment: Color::Rgb(255, 150, 108),  // #ff966c
        },
    }
}

// --- Config types ---

/// Per-theme config with named color palette and role/label mappings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Named color palette: name -> "#rrggbb"
    #[serde(default)]
    pub colors: BTreeMap<String, String>,
    /// UI role -> palette color name
    #[serde(default)]
    pub ui: Option<UiConfig>,
    /// Category label -> palette color name
    #[serde(default)]
    pub labels: Option<LabelsConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UiConfig {
    pub border: Option<String>,
    pub accent: Option<String>,
    pub text: Option<String>,
    pub text_bright: Option<String>,
    pub text_dim: Option<String>,
    pub text_muted: Option<String>,
    pub heading: Option<String>,
    pub error: Option<String>,
    pub cursor_bg: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LabelsConfig {
    pub bugs: Option<String>,
    pub features: Option<String>,
    pub improvements: Option<String>,
    pub refactor: Option<String>,
    pub docs: Option<String>,
    pub chore: Option<String>,
    pub data: Option<String>,
    pub model: Option<String>,
    pub experiment: Option<String>,
}

/// Parse "#rrggbb" to Color::Rgb.
fn parse_hex(s: &str) -> Option<Color> {
    let s = s.strip_prefix('#')?;
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

/// Look up a color name in the palette and parse it.
fn resolve_color(name: &str, palette: &BTreeMap<String, String>) -> Option<Color> {
    palette.get(name).and_then(|hex| parse_hex(hex))
}

impl ThemeConfig {
    /// Resolve this config into a Theme, falling back to `base` for any missing fields.
    pub fn resolve(&self, base: &Theme) -> Theme {
        let p = &self.colors;
        let ui = self.ui.as_ref();
        let lb = self.labels.as_ref();

        let r = |field: Option<&Option<String>>, fallback: Color| -> Color {
            field
                .and_then(|opt| opt.as_ref())
                .and_then(|name| resolve_color(name, p))
                .unwrap_or(fallback)
        };

        Theme {
            border: r(ui.map(|u| &u.border), base.border),
            accent: r(ui.map(|u| &u.accent), base.accent),
            text: r(ui.map(|u| &u.text), base.text),
            text_bright: r(ui.map(|u| &u.text_bright), base.text_bright),
            text_dim: r(ui.map(|u| &u.text_dim), base.text_dim),
            text_muted: r(ui.map(|u| &u.text_muted), base.text_muted),
            heading: r(ui.map(|u| &u.heading), base.heading),
            error: r(ui.map(|u| &u.error), base.error),
            cursor_bg: r(ui.map(|u| &u.cursor_bg), base.cursor_bg),
            labels: CategoryLabels {
                bugs: r(lb.map(|l| &l.bugs), base.labels.bugs),
                features: r(lb.map(|l| &l.features), base.labels.features),
                improvements: r(lb.map(|l| &l.improvements), base.labels.improvements),
                refactor: r(lb.map(|l| &l.refactor), base.labels.refactor),
                docs: r(lb.map(|l| &l.docs), base.labels.docs),
                chore: r(lb.map(|l| &l.chore), base.labels.chore),
                data: r(lb.map(|l| &l.data), base.labels.data),
                model: r(lb.map(|l| &l.model), base.labels.model),
                experiment: r(lb.map(|l| &l.experiment), base.labels.experiment),
            },
        }
    }
}

pub fn find_theme(themes: &[(String, Theme)], name: &str) -> Option<(usize, Theme)> {
    themes
        .iter()
        .enumerate()
        .find(|(_, (n, _))| n == name)
        .map(|(i, (_, t))| (i, *t))
}

/// Build theme list from config. If config has no themes, returns a single default.
pub fn resolve_themes(theme_configs: &BTreeMap<String, ThemeConfig>) -> Vec<(String, Theme)> {
    if theme_configs.is_empty() {
        return vec![("default".into(), default_theme())];
    }

    let base = default_theme();
    theme_configs
        .iter()
        .map(|(name, cfg)| (name.clone(), cfg.resolve(&base)))
        .collect()
}
