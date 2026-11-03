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
    pub name: String,
    colors: BTreeMap<String, Color>,
}

impl Theme {
    pub fn load(paths: &Paths, flavor: &str) -> Self {
        let flavor = flavor.trim().to_ascii_lowercase();
        let has_file = is_valid_name(&flavor)
            && !FLAVORS.contains(&flavor.as_str())
            && paths.theme_file(&flavor).is_file();
        if !FLAVORS.contains(&flavor.as_str()) && !has_file {
            return Self::parse("mocha", builtin("mocha")).expect("embedded theme is valid");
        }
        let disk = paths.theme_file(&flavor);
        let local = Paths::local_theme_file(&flavor);
        let from_disk = [disk, local]
            .iter()
            .find_map(|path| fs::read_to_string(path).ok());
        let Some(raw) = from_disk else {
            return Self::parse(&flavor, builtin(&flavor)).expect("embedded theme is valid");
        };
        Self::parse(&flavor, &raw).unwrap_or_else(|_| {
            Self::parse("mocha", builtin("mocha")).expect("embedded theme is valid")
        })
    }

    pub fn display_name(&self) -> &str {
        &self.name
    }

    pub fn available(paths: &Paths) -> Vec<String> {
        let mut names: Vec<String> = FLAVORS.iter().map(|name| name.to_string()).collect();
        let mut custom = Vec::new();
        let dirs = [
            paths.themes_dir(),
            Paths::local_theme_file("")
                .parent()
                .map(|path| path.to_path_buf())
                .unwrap_or_default(),
        ];
        for dir in dirs {
            let Ok(entries) = fs::read_dir(dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
                    continue;
                }
                let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                    continue;
                };
                let name = stem.trim().to_ascii_lowercase();
                if is_valid_name(&name) && !names.contains(&name) && !custom.contains(&name) {
                    custom.push(name);
                }
            }
        }
        custom.sort();
        names.extend(custom);
        names
    }

    pub fn exists(paths: &Paths, flavor: &str) -> bool {
        let flavor = flavor.trim().to_ascii_lowercase();
        FLAVORS.contains(&flavor.as_str())
            || (is_valid_name(&flavor)
                && (paths.theme_file(&flavor).is_file()
                    || Paths::local_theme_file(&flavor).is_file()))
    }

    fn parse(flavor: &str, raw: &str) -> Result<Self, String> {
        let value: toml::Value = toml::from_str(raw).map_err(|err| err.to_string())?;
        let table = value.as_table().ok_or("theme root must be a table")?;
        let name = table
            .get("name")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or(flavor)
            .to_string();
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
            name,
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
}

pub fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
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
    fn loads_custom_theme_files() {
        let root = std::env::temp_dir().join(format!("gitx-theme-custom-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let paths = Paths::for_test(root.join("config"), root.join("cache"));
        fs::create_dir_all(paths.themes_dir()).unwrap();
        let raw = builtin("mocha")
            .replace("name = \"Mocha\"", "name = \"Neon\"")
            .replace("base = \"#1e1e2e\"", "base = \"#101820\"");
        fs::write(paths.theme_file("neon"), raw).unwrap();
        assert!(Theme::exists(&paths, "neon"));
        assert!(Theme::available(&paths).contains(&"neon".to_string()));
        let theme = Theme::load(&paths, "neon");
        assert_eq!(theme.display_name(), "Neon");
        assert_ne!(
            theme.color("base"),
            Theme::load(&paths, "mocha").color("base")
        );
        assert!(!Theme::exists(&paths, "../escape"));
        let _ = fs::remove_dir_all(&root);
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
