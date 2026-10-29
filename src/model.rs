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
            Some("private") => "私有喵~",
            Some("internal") => "内部喵~",
            _ if self.private => "私有喵~",
            _ => "公开喵~",
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
            .unwrap_or_else(|| "未标注喵~".to_string())
    }

    pub fn language_label(&self) -> &str {
        self.language.as_deref().unwrap_or("未标注喵~")
    }

    pub fn description_text(&self) -> &str {
        self.description
            .as_deref()
            .unwrap_or("这个仓库还没有写简介喵~")
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
pub struct UserPlan {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub space: u64,
    #[serde(default)]
    pub private_repos: u64,
    #[serde(default)]
    pub collaborators: u64,
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
    pub avatar_url: String,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub company: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub blog: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub twitter_username: Option<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub public_repos: u64,
    #[serde(default)]
    pub public_gists: u64,
    #[serde(default)]
    pub followers: u64,
    #[serde(default)]
    pub following: u64,
    #[serde(default)]
    pub plan: Option<UserPlan>,
}

impl User {
    pub fn display_name(&self) -> &str {
        self.name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or(&self.login)
    }

    pub fn joined_label(&self) -> String {
        if self.created_at.is_empty() {
            "时间不详喵~".to_string()
        } else {
            crate::format::relative_time(&self.created_at)
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RateLimitInfo {
    #[serde(default)]
    pub limit: u64,
    #[serde(default)]
    pub used: u64,
    #[serde(default)]
    pub remaining: u64,
    #[serde(default)]
    pub reset: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RateLimitResources {
    #[serde(default)]
    pub core: RateLimitInfo,
    #[serde(default)]
    pub search: RateLimitInfo,
    #[serde(default)]
    pub graphql: RateLimitInfo,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RateLimit {
    #[serde(default)]
    pub resources: RateLimitResources,
}

impl RateLimit {
    pub fn items(&self) -> [(&'static str, &RateLimitInfo); 3] {
        [
            ("核心接口喵~", &self.resources.core),
            ("搜索接口喵~", &self.resources.search),
            ("GraphQL", &self.resources.graphql),
        ]
    }
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
            Self::BestMatch => "最佳匹配喵~",
            Self::Stars => "星标优先喵~",
            Self::Updated => "最近更新喵~",
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
            Self::Updated => "最近更新喵~",
            Self::Stars => "星标喵~",
            Self::Name => "名称喵~",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TreeEntry {
    #[serde(default)]
    pub path: String,
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub sha: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TreeResponse {
    #[serde(default)]
    pub sha: String,
    #[serde(default)]
    pub truncated: bool,
    #[serde(default)]
    pub tree: Vec<TreeEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReadmeContent {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rate_limit_resources() {
        let raw = r#"{
            "resources": {
                "core": {"limit": 5000, "used": 12, "remaining": 4988, "reset": 1900000000},
                "search": {"limit": 30, "used": 1, "remaining": 29, "reset": 1900000100},
                "graphql": {"limit": 5000, "used": 0, "remaining": 5000, "reset": 1900000200}
            }
        }"#;
        let rate: RateLimit = serde_json::from_str(raw).unwrap();
        assert_eq!(rate.resources.core.remaining, 4988);
        assert_eq!(rate.resources.search.limit, 30);
        assert_eq!(rate.items()[2].0, "GraphQL");
    }

    #[test]
    fn keeps_extended_user_fields() {
        let raw = r#"{
            "login": "me",
            "name": "Meow",
            "bio": "hi",
            "public_repos": 7,
            "followers": 3,
            "following": 2,
            "created_at": "2020-01-02T03:04:05Z",
            "plan": {"name": "pro", "private_repos": 100}
        }"#;
        let user: User = serde_json::from_str(raw).unwrap();
        assert_eq!(user.display_name(), "Meow");
        assert_eq!(user.plan.as_ref().unwrap().name, "pro");
        assert!(!user.joined_label().is_empty());
    }
}
