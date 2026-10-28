use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Owner {
    #[serde(default)]
    pub login: String,
    #[serde(default)]
    pub html_url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct License {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub spdx_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Repo {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub full_name: String,
    #[serde(default)]
    pub owner: Owner,
    #[serde(default)]
    pub private: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub clone_url: String,
    #[serde(default)]
    pub ssh_url: String,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub stargazers_count: u64,
    #[serde(default)]
    pub forks_count: u64,
    #[serde(default)]
    pub watchers_count: u64,
    #[serde(default)]
    pub subscribers_count: u64,
    #[serde(default)]
    pub network_count: u64,
    #[serde(default)]
    pub open_issues_count: u64,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub pushed_at: Option<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub default_branch: String,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub fork: bool,
    #[serde(default)]
    pub has_issues: bool,
    #[serde(default)]
    pub has_discussions: bool,
    #[serde(default)]
    pub license: Option<License>,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub visibility: Option<String>,
}

impl Repo {
    pub fn display_full_name(&self) -> String {
        if self.full_name.is_empty() {
            format!("{}/{}", self.owner.login, self.name)
        } else {
            self.full_name.clone()
        }
    }

    pub fn owner_name(&self) -> &str {
        if self.owner.login.is_empty() {
            self.full_name.split('/').next().unwrap_or("")
        } else {
            self.owner.login.as_str()
        }
    }

    pub fn visibility(&self) -> &str {
        match self.visibility.as_deref() {
            Some("private") => "私有",
            Some("internal") => "内部",
            _ if self.private => "私有",
            _ => "公开",
        }
    }

    pub fn license_label(&self) -> String {
        self.license
            .as_ref()
            .and_then(|license| {
                license
                    .spdx_id
                    .clone()
                    .filter(|value| value != "NOASSERTION")
                    .or_else(|| (!license.name.is_empty()).then(|| license.name.clone()))
            })
            .unwrap_or_else(|| "未标注".to_string())
    }

    pub fn language_label(&self) -> &str {
        self.language.as_deref().unwrap_or("未标注")
    }

    pub fn description_text(&self) -> &str {
        self.description
            .as_deref()
            .unwrap_or("这个仓库还没有写简介。")
    }

    pub fn updated_at(&self) -> &str {
        if !self.updated_at.is_empty() {
            &self.updated_at
        } else if let Some(pushed) = self.pushed_at.as_deref() {
            pushed
        } else {
            &self.created_at
        }
    }

    pub fn cache_key(&self) -> String {
        format!("repo_{}", key_part(&self.display_full_name()))
    }

    pub fn releases_cache_key(&self) -> String {
        format!("releases_{}", key_part(&self.display_full_name()))
    }
}

pub fn key_part(full_name: &str) -> String {
    full_name
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | ' ' => '_',
            ch if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' => ch,
            _ => '_',
        })
        .collect::<String>()
        .to_ascii_lowercase()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct User {
    #[serde(default)]
    pub login: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub public_repos: u64,
    #[serde(default)]
    pub followers: u64,
    #[serde(default)]
    pub following: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReleaseAsset {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub download_count: u64,
    #[serde(default)]
    pub browser_download_url: String,
    #[serde(default)]
    pub content_type: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Release {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub tag_name: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub tarball_url: Option<String>,
    #[serde(default)]
    pub zipball_url: Option<String>,
    #[serde(default)]
    pub assets: Vec<ReleaseAsset>,
}

impl Release {
    pub fn title(&self) -> &str {
        self.name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(&self.tag_name)
    }

    pub fn date(&self) -> &str {
        self.published_at
            .as_deref()
            .filter(|date| !date.is_empty())
            .unwrap_or(&self.created_at)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchResponse {
    #[serde(default)]
    pub total_count: u64,
    #[serde(default)]
    pub incomplete_results: bool,
    #[serde(default)]
    pub items: Vec<Repo>,
}

#[derive(Debug, Clone)]
pub struct SearchOutcome {
    pub total: u64,
    pub items: Vec<Repo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchSort {
    BestMatch,
    Stars,
    Updated,
}

impl SearchSort {
    pub fn label(self) -> &'static str {
        match self {
            Self::BestMatch => "最佳匹配",
            Self::Stars => "星标优先",
            Self::Updated => "最近更新",
        }
    }

    pub fn api_params(self) -> &'static str {
        match self {
            Self::BestMatch => "",
            Self::Stars => "&sort=stars&order=desc",
            Self::Updated => "&sort=updated&order=desc",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::BestMatch => "best",
            Self::Stars => "stars",
            Self::Updated => "updated",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoSort {
    Updated,
    Stars,
    Name,
}

impl RepoSort {
    pub fn label(self) -> &'static str {
        match self {
            Self::Updated => "最近更新",
            Self::Stars => "星标",
            Self::Name => "名称",
        }
    }
}
