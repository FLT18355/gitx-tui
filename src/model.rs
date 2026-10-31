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

pub fn conclusion_label(raw: &str) -> &str {
    match raw {
        "success" => "成功",
        "failure" => "失败",
        "cancelled" => "已取消",
        "timed_out" => "超时",
        "startup_failure" => "启动失败",
        "skipped" => "已跳过",
        "neutral" => "中性",
        "action_required" => "需要处理",
        "stale" => "已过期",
        other => other,
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowStep {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub conclusion: Option<String>,
    #[serde(default)]
    pub number: u64,
}

impl WorkflowStep {
    pub fn status_label(&self) -> &str {
        match self.status.as_str() {
            "completed" => self
                .conclusion
                .as_deref()
                .map(conclusion_label)
                .unwrap_or("已完成"),
            "in_progress" => "进行中",
            "queued" => "排队中",
            "waiting" => "等待中",
            "requested" => "已请求",
            "pending" => "等待中",
            _ => &self.status,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowJob {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub conclusion: Option<String>,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub steps: Vec<WorkflowStep>,
}

impl WorkflowJob {
    pub fn status_label(&self) -> &str {
        match self.status.as_str() {
            "completed" => self
                .conclusion
                .as_deref()
                .map(conclusion_label)
                .unwrap_or("已完成"),
            "in_progress" => "进行中",
            "queued" => "排队中",
            "waiting" => "等待中",
            "requested" => "已请求",
            "pending" => "等待中",
            _ => &self.status,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowRun {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub display_title: String,
    #[serde(default)]
    pub event: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub conclusion: Option<String>,
    #[serde(default)]
    pub head_branch: Option<String>,
    #[serde(default)]
    pub run_number: u64,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub run_started_at: Option<String>,
    #[serde(default)]
    pub actor: Option<Owner>,
}

impl WorkflowRun {
    pub fn workflow_name(&self) -> &str {
        self.name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("未命名工作流")
    }

    pub fn title(&self) -> &str {
        if self.display_title.trim().is_empty() {
            self.workflow_name()
        } else {
            &self.display_title
        }
    }

    pub fn branch(&self) -> &str {
        self.head_branch.as_deref().unwrap_or("")
    }

    pub fn status_label(&self) -> &str {
        match self.status.as_str() {
            "completed" => self
                .conclusion
                .as_deref()
                .map(conclusion_label)
                .unwrap_or("已完成"),
            "in_progress" => "进行中",
            "queued" => "排队中",
            "waiting" => "等待中",
            "requested" => "已请求",
            "pending" => "等待中",
            _ => &self.status,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Workflow {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub html_url: String,
}

impl Workflow {
    pub fn state_label(&self) -> &str {
        match self.state.as_str() {
            "active" => "启用",
            "disabled_manually" => "已停用",
            "disabled_inactivity" => "不活跃停用",
            "deleted" => "已删除",
            _ => &self.state,
        }
    }

    pub fn file_name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowListResponse {
    #[serde(default)]
    pub total_count: u64,
    #[serde(default)]
    pub workflows: Vec<Workflow>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowRunsResponse {
    #[serde(default)]
    pub total_count: u64,
    #[serde(default)]
    pub workflow_runs: Vec<WorkflowRun>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowJobsResponse {
    #[serde(default)]
    pub total_count: u64,
    #[serde(default)]
    pub jobs: Vec<WorkflowJob>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotificationSubject {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub latest_comment_url: Option<String>,
    #[serde(rename = "type", default)]
    pub subject_type: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Notification {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub unread: bool,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub subject: NotificationSubject,
    #[serde(default)]
    pub repository: Repo,
}

impl Notification {
    pub fn subject_type_label(&self) -> &str {
        match self.subject.subject_type.as_str() {
            "Issue" => "议题",
            "PullRequest" => "合并请求",
            "Discussion" => "讨论",
            "Release" => "发行版",
            "Commit" => "提交",
            "CheckSuite" => "检查",
            "RepositoryVulnerabilityAlert" => "安全提醒",
            "" => "通知",
            other => other,
        }
    }

    pub fn reason_label(&self) -> &str {
        match self.reason.as_str() {
            "assign" => "指派给你",
            "author" => "你发起的",
            "comment" => "有评论",
            "ci_activity" => "CI 动静",
            "invitation" => "邀请",
            "manual" => "手动订阅",
            "mention" => "提到了你",
            "review_requested" => "请求你审查",
            "security_alert" => "安全提醒",
            "state_change" => "状态变化",
            "subscribed" => "你订阅了",
            "team_mention" => "提到你的团队",
            "" => "通知",
            other => other,
        }
    }

    pub fn repo_label(&self) -> String {
        let name = self.repository.display_full_name();
        if name.trim().is_empty() {
            "不明仓库".to_string()
        } else {
            name
        }
    }

    pub fn html_url(&self) -> String {
        let repo = if self.repository.html_url.trim().is_empty() {
            format!("https://github.com/{}", self.repo_label())
        } else {
            self.repository.html_url.trim_end_matches('/').to_string()
        };
        match self.subject.subject_type.as_str() {
            "Issue" | "PullRequest" | "Discussion" | "Commit" => {
                api_subject_to_web(self.subject.url.as_deref().unwrap_or("")).unwrap_or(repo)
            }
            "Release" => format!("{repo}/releases"),
            "RepositoryVulnerabilityAlert" => format!("{repo}/security/dependabot"),
            "CheckSuite" => format!("{repo}/actions"),
            _ => repo,
        }
    }
}

fn api_subject_to_web(url: &str) -> Option<String> {
    let path = url
        .trim()
        .strip_prefix("https://api.github.com/repos/")?
        .trim_start_matches('/');
    let mut parts = path.splitn(3, '/');
    let owner = parts.next()?.trim();
    let repo = parts.next()?.trim();
    let rest = parts.next()?.trim().trim_end_matches('/');
    if owner.is_empty() || repo.is_empty() || rest.is_empty() {
        return None;
    }
    let mapped = if let Some(number) = rest.strip_prefix("pulls/") {
        format!("pull/{number}")
    } else if let Some(sha) = rest.strip_prefix("commits/") {
        format!("commit/{sha}")
    } else {
        rest.to_string()
    };
    Some(format!("https://github.com/{owner}/{repo}/{mapped}"))
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IssueLabel {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PullRequestLink {
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub merged_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Issue {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub number: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub state_reason: Option<String>,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub user: Option<Owner>,
    #[serde(default)]
    pub labels: Vec<IssueLabel>,
    #[serde(default)]
    pub comments: u64,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub closed_at: Option<String>,
    #[serde(default)]
    pub pull_request: Option<PullRequestLink>,
}

impl Issue {
    pub fn is_pull_request(&self) -> bool {
        self.pull_request.is_some()
    }

    pub fn is_merged(&self) -> bool {
        self.pull_request
            .as_ref()
            .and_then(|pull| pull.merged_at.as_deref())
            .is_some_and(|merged| !merged.trim().is_empty())
    }

    pub fn kind_label(&self) -> &str {
        if self.is_pull_request() {
            "合并请求"
        } else {
            "议题"
        }
    }

    pub fn state_label(&self) -> &str {
        if self.is_merged() {
            return "已合并";
        }
        match self.state.as_str() {
            "open" => "打开",
            "closed" => "已关闭",
            other => other,
        }
    }

    pub fn author(&self) -> &str {
        self.user
            .as_ref()
            .map(|user| user.login.as_str())
            .unwrap_or("")
    }

    pub fn labels_label(&self) -> String {
        self.labels
            .iter()
            .map(|label| label.name.trim())
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>()
            .join(" · ")
    }

    pub fn body_text(&self) -> &str {
        self.body
            .as_deref()
            .map(str::trim)
            .filter(|body| !body.is_empty())
            .unwrap_or("没有写正文喵~")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueState {
    Open,
    Closed,
    All,
}

impl IssueState {
    pub fn api_value(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::All => "all",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Open => "打开喵~",
            Self::Closed => "已关闭喵~",
            Self::All => "全部喵~",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Open => Self::Closed,
            Self::Closed => Self::All,
            Self::All => Self::Open,
        }
    }
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

    #[test]
    fn parses_workflows() {
        let raw = r#"{
            "total_count": 1,
            "workflows": [{
                "id": 42,
                "name": "Release",
                "path": ".github/workflows/release.yml",
                "state": "active",
                "html_url": "https://github.com/me/demo/actions/workflows/release.yml"
            }]
        }"#;
        let response: WorkflowListResponse = serde_json::from_str(raw).unwrap();
        let workflow = &response.workflows[0];
        assert_eq!(workflow.id, 42);
        assert_eq!(workflow.file_name(), "release.yml");
        assert_eq!(workflow.state_label(), "启用");
    }

    #[test]
    fn parses_workflow_runs_and_jobs() {
        let raw = r#"{
            "total_count": 1,
            "workflow_runs": [{
                "id": 9001,
                "name": "CI",
                "display_title": "fix: 喵",
                "event": "push",
                "status": "completed",
                "conclusion": "success",
                "head_branch": "main",
                "run_number": 12,
                "html_url": "https://github.com/me/demo/actions/runs/9001",
                "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:02:00Z",
                "run_started_at": "2026-01-01T00:00:10Z",
                "actor": {"login": "me"}
            }]
        }"#;
        let response: WorkflowRunsResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(response.total_count, 1);
        let run = &response.workflow_runs[0];
        assert_eq!(run.title(), "fix: 喵");
        assert_eq!(run.branch(), "main");
        assert_eq!(run.status_label(), "成功");
        assert_eq!(run.conclusion.as_deref(), Some("success"));

        let jobs = r#"{
            "total_count": 1,
            "jobs": [{
                "id": 1,
                "name": "build",
                "status": "completed",
                "conclusion": "failure",
                "started_at": "2026-01-01T00:00:10Z",
                "completed_at": "2026-01-01T00:01:10Z",
                "html_url": "https://github.com/me/demo/actions/runs/9001/job/1",
                "steps": [{
                    "name": "checkout",
                    "status": "completed",
                    "conclusion": "success",
                    "number": 1
                }]
            }]
        }"#;
        let response: WorkflowJobsResponse = serde_json::from_str(jobs).unwrap();
        let job = &response.jobs[0];
        assert_eq!(job.status_label(), "失败");
        assert_eq!(job.steps[0].status_label(), "成功");
    }

    #[test]
    fn notification_links_to_github_web_urls() {
        let raw = r#"{
            "id": "11",
            "unread": true,
            "reason": "mention",
            "updated_at": "2026-10-30T10:00:00Z",
            "subject": {
                "title": "修一下导航喵~",
                "url": "https://api.github.com/repos/me/demo/pulls/7",
                "type": "PullRequest"
            },
            "repository": {
                "name": "demo",
                "full_name": "me/demo",
                "html_url": "https://github.com/me/demo",
                "private": false
            }
        }"#;
        let notification: Notification = serde_json::from_str(raw).unwrap();
        assert!(notification.unread);
        assert_eq!(notification.subject_type_label(), "合并请求");
        assert_eq!(notification.reason_label(), "提到了你");
        assert_eq!(notification.repo_label(), "me/demo");
        assert_eq!(notification.html_url(), "https://github.com/me/demo/pull/7");
    }

    #[test]
    fn issue_kind_and_state_labels() {
        let raw = r#"{
            "number": 3,
            "title": "喵",
            "state": "closed",
            "body": "正文",
            "user": {"login": "me"},
            "labels": [{"name": "bug", "color": "ff0000"}],
            "pull_request": {"html_url": "https://github.com/me/demo/pull/3", "merged_at": "2026-10-30T00:00:00Z"}
        }"#;
        let issue: Issue = serde_json::from_str(raw).unwrap();
        assert!(issue.is_pull_request());
        assert!(issue.is_merged());
        assert_eq!(issue.kind_label(), "合并请求");
        assert_eq!(issue.state_label(), "已合并");
        assert_eq!(issue.author(), "me");
        assert_eq!(issue.labels_label(), "bug");
        assert_eq!(issue.body_text(), "正文");
        assert_eq!(IssueState::Closed.next(), IssueState::All);
        assert_eq!(IssueState::All.next(), IssueState::Open);
    }
}
