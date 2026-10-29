use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_PROXY: &str = "https://v4.gh-proxy.org";
const DEFAULT_CONFIG: &str = include_str!("../config.toml");

pub fn default_download_dir() -> String {
    "~/Downloads/gitx-tui".to_string()
}

pub fn expand_tilde(value: &str) -> PathBuf {
    let value = value.trim();
    if value == "~" {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
    }
    if let Some(rest) = value.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(value)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: String,
    pub proxy: String,
    pub repos_ttl_secs: u64,
    pub detail_ttl_secs: u64,
    pub search_ttl_secs: u64,
    pub search_history: Vec<String>,
    pub show_file_tree: bool,
    pub show_readme: bool,
    pub download_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "mocha".to_string(),
            proxy: DEFAULT_PROXY.to_string(),
            repos_ttl_secs: 300,
            detail_ttl_secs: 900,
            search_ttl_secs: 600,
            search_history: Vec::new(),
            show_file_tree: false,
            show_readme: false,
            download_dir: default_download_dir(),
        }
    }
}

impl Config {
    pub fn load(paths: &Paths) -> Self {
        let path = paths.config_file();
        if !path.exists() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
                secure_dir(parent);
            }
            let _ = fs::write(&path, DEFAULT_CONFIG);
            secure_file(&path);
        }
        let mut config = fs::read_to_string(&path)
            .ok()
            .and_then(|raw| toml::from_str::<Config>(&raw).ok())
            .unwrap_or_default();
        config.normalize();
        config
    }

    fn normalize(&mut self) {
        self.theme = self.theme.trim().to_ascii_lowercase();
        if !crate::theme::FLAVORS.contains(&self.theme.as_str()) {
            self.theme = "mocha".to_string();
        }
        self.proxy = self.proxy.trim().trim_end_matches('/').to_string();
        if self.proxy.is_empty() {
            self.proxy = DEFAULT_PROXY.to_string();
        }
        self.repos_ttl_secs = self.repos_ttl_secs.clamp(60, 86_400);
        self.detail_ttl_secs = self.detail_ttl_secs.clamp(60, 86_400);
        self.search_ttl_secs = self.search_ttl_secs.clamp(60, 86_400);
        self.search_history.truncate(8);
        self.download_dir = self.download_dir.trim().to_string();
        if self.download_dir.is_empty() {
            self.download_dir = default_download_dir();
        }
    }

    pub fn expanded_download_dir(&self) -> PathBuf {
        expand_tilde(&self.download_dir)
    }

    pub fn save(&self, paths: &Paths) -> std::io::Result<()> {
        fs::create_dir_all(paths.config_dir())?;
        secure_dir(paths.config_dir());
        let raw = toml::to_string_pretty(self)
            .map_err(|err| std::io::Error::other(format!("serialize config: {err}")))?;
        let path = paths.config_file();
        let tmp = path.with_extension("toml.tmp");
        fs::write(&tmp, raw)?;
        secure_file(&tmp);
        fs::rename(tmp, path)
    }

    pub fn remember_search(&mut self, query: &str) {
        let query = query.trim();
        if query.is_empty() {
            return;
        }
        self.search_history.retain(|item| item != query);
        self.search_history.insert(0, query.to_string());
        self.search_history.truncate(8);
    }
}

#[derive(Debug, Clone)]
pub struct Paths {
    config_dir: PathBuf,
    cache_dir: PathBuf,
}

impl Paths {
    pub fn discover() -> Self {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let config_dir = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"))
            .join("gitx-tui");
        let cache_dir = std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".cache"))
            .join("gitx-tui");
        Self {
            config_dir,
            cache_dir,
        }
    }

    #[cfg(test)]
    pub fn for_test(config_dir: PathBuf, cache_dir: PathBuf) -> Self {
        Self {
            config_dir,
            cache_dir,
        }
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    pub fn theme_file(&self, flavor: &str) -> PathBuf {
        self.config_dir
            .join("themes")
            .join(format!("{flavor}.toml"))
    }

    pub fn local_theme_file(flavor: &str) -> PathBuf {
        PathBuf::from("themes").join(format!("{flavor}.toml"))
    }
}

fn secure_dir(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o700));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

fn secure_file(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remembers_unique_search_terms() {
        let mut config = Config::default();
        config.remember_search("ratatui");
        config.remember_search("crossterm");
        config.remember_search("ratatui");
        assert_eq!(config.search_history[0], "ratatui");
        assert_eq!(config.search_history.len(), 2);
    }

    #[test]
    fn bundled_config_parses() {
        let config: Config = toml::from_str(DEFAULT_CONFIG).expect("bundled config");
        assert!(crate::theme::FLAVORS.contains(&config.theme.as_str()));
        assert_eq!(config.proxy, DEFAULT_PROXY);
    }

    #[test]
    fn old_config_gets_new_defaults() {
        let config: Config =
            toml::from_str("theme = \"latte\"\nproxy = \"https://v4.gh-proxy.org\"\n")
                .expect("old config");
        assert!(!config.show_file_tree);
        assert!(!config.show_readme);
        assert_eq!(config.download_dir, default_download_dir());
    }

    #[test]
    fn expands_home_path() {
        if let Some(home) = std::env::var_os("HOME") {
            assert_eq!(
                expand_tilde("~/Downloads/gitx-tui"),
                PathBuf::from(home).join("Downloads/gitx-tui")
            );
        }
    }

    #[test]
    fn load_creates_config_file() {
        let root = std::env::temp_dir().join(format!("gitx-tui-config-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let paths = Paths::for_test(root.join("config"), root.join("cache"));
        let config = Config::load(&paths);
        assert!(paths.config_file().exists());
        assert_eq!(config.theme, "mocha");
        let _ = fs::remove_dir_all(&root);
    }
}
