use crate::config::Paths;
use ratatui::style::Color;
use std::collections::BTreeMap;
use std::fs;

pub const FLAVORS: [&str; 4] = ["latte", "frappe", "macchiato", "mocha"];

const LATTE: &str = include_str!("../themes/latte.toml");
const FRAPPE: &str = include_str!("../themes/frappe.toml");
const MACCHIATO: &str = include_str!("../themes/macchiato.toml");
const MOCHA: &str = include_str!("../themes/mocha.toml");

#[derive(Debug, Clone)]
pub struct Theme {
    pub flavor: String,
    colors: BTreeMap<String, Color>,
}

impl Theme {
    pub fn load(paths: &Paths, flavor: &str) -> Self {
        let flavor = if FLAVORS.contains(&flavor) {
            flavor
        } else {
            "mocha"
        };
        let local = Paths::local_theme_file(flavor);
        let config = paths.theme_file(flavor);
        let from_disk = [local, config]
            .iter()
            .find_map(|path| fs::read_to_string(path).ok());
        let raw = from_disk.as_deref().unwrap_or_else(|| builtin(flavor));
        Self::parse(flavor, raw).unwrap_or_else(|_| {
            Self::parse(flavor, builtin(flavor)).expect("embedded catppuccin theme is valid")
        })
    }

    fn parse(flavor: &str, raw: &str) -> Result<Self, String> {
        let value: toml::Value = toml::from_str(raw).map_err(|err| err.to_string())?;
        let table = value.as_table().ok_or("theme root must be a table")?;
        let mut colors = BTreeMap::new();
        for (key, value) in table {
            let Some(hex) = value.as_str() else {
                continue;
            };
            if let Some(color) = parse_hex(hex) {
                colors.insert(key.clone(), color);
            }
        }
        if colors.len() < 12 {
            return Err("theme does not contain enough colors".to_string());
        }
        Ok(Self {
            flavor: flavor.to_string(),
            colors,
        })
    }

    pub fn color(&self, name: &str) -> Color {
        self.colors
            .get(name)
            .copied()
            .or_else(|| self.colors.get("text").copied())
            .unwrap_or(Color::White)
    }

    pub fn label(flavor: &str) -> &'static str {
        match flavor {
            "latte" => "Latte",
            "frappe" => "Frappe",
            "macchiato" => "Macchiato",
            _ => "Mocha",
        }
    }
}

fn builtin(flavor: &str) -> &'static str {
    match flavor {
        "latte" => LATTE,
        "frappe" => FRAPPE,
        "macchiato" => MACCHIATO,
        _ => MOCHA,
    }
}

fn parse_hex(hex: &str) -> Option<Color> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let value = u32::from_str_radix(hex, 16).ok()?;
    Some(Color::Rgb(
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_embedded_themes() {
        for flavor in FLAVORS {
            let theme = Theme::parse(flavor, builtin(flavor)).expect("valid theme");
            assert!(theme.colors.len() >= 20);
            assert_eq!(theme.flavor, flavor);
            assert_ne!(theme.color("base"), theme.color("text"));
        }
    }

    #[test]
    fn parses_hex() {
        assert_eq!(parse_hex("#89b4fa"), Some(Color::Rgb(137, 180, 250)));
        assert_eq!(parse_hex("89b4fa"), Some(Color::Rgb(137, 180, 250)));
        assert_eq!(parse_hex("#bad"), None);
    }

    #[test]
    fn unknown_theme_falls_back() {
        let paths = Paths::for_test(
            std::env::temp_dir().join("gitx-tui-theme-test"),
            std::env::temp_dir().join("gitx-tui-theme-cache"),
        );
        let theme = Theme::load(&paths, "unknown");
        assert_eq!(theme.flavor, "mocha");
    }
}
