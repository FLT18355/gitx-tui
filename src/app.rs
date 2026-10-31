use crate::api::ApiClient;
use crate::cache::{Cache, CacheStats, now_epoch, search_key};
use crate::config::{Config, Paths};
use crate::format::{self, base64};
use crate::icons;
use crate::model::{
    Issue, IssueState, Notification, RateLimit, ReadmeContent, Release, Repo, RepoSort,
    SearchResponse, SearchSort, StarSort, TreeResponse, User, Workflow, key_part,
};
use crate::theme::{FLAVORS, Theme};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use std::io::Write;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Repositories,
    Starred,
    Search,
    Account,
    Notifications,
    Settings,
    Repo,
    Actions,
    Issues,
    ReleaseDetail,
}

impl Page {
    pub fn nav_index(self) -> Option<usize> {
        match self {
            Self::Repositories => Some(0),
            Self::Starred => Some(1),
            Self::Search => Some(2),
            Self::Account => Some(3),
            Self::Notifications => Some(4),
            Self::Repo | Self::Actions | Self::Issues | Self::ReleaseDetail | Self::Settings => {
                None
            }
        }
    }

    pub fn from_nav(index: usize) -> Self {
        match index {
            1 => Self::Starred,
            2 => Self::Search,
            3 => Self::Account,
            4 => Self::Notifications,
            _ => Self::Repositories,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Repositories => "人家的仓库喵~",
            Self::Starred => "星星收藏喵~",
            Self::Search => "找仓库喵~",
            Self::Account => "账号喵~",
            Self::Notifications => "通知喵~",
            Self::Settings => "设置喵~",
            Self::Repo => "仓库详情喵~",
            Self::Actions => "工作流喵~",
            Self::Issues => "议题喵~",
            Self::ReleaseDetail => "发行版详情喵~",
        }
    }

    pub fn is_repo_page(self) -> bool {
        matches!(
            self,
            Self::Repo | Self::Actions | Self::Issues | Self::ReleaseDetail
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    Help,
    Languages,
    EditRepo,
    Cache,
    RunWorkflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoPane {
    Tree,
    Releases,
    Readme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueKind {
    All,
    Issues,
    PullRequests,
}

impl IssueKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "全部喵~",
            Self::Issues => "议题喵~",
            Self::PullRequests => "合并请求喵~",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Issues,
            Self::Issues => Self::PullRequests,
            Self::PullRequests => Self::All,
        }
    }

    pub fn matches(self, issue: &crate::model::Issue) -> bool {
        match self {
            Self::All => true,
            Self::Issues => !issue.is_pull_request(),
            Self::PullRequests => issue.is_pull_request(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Ok,
    Error,
}

#[derive(Debug, Clone)]
pub struct Status {
    pub text: String,
    pub kind: StatusKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtlKind {
    Repositories,
    Detail,
    Search,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Navigate(Page),
    SelectVisible(usize),
    Language(Option<String>),
    SetRepoSort(RepoSort),
    SetStarSort(StarSort),
    SetSearchSort(SearchSort),
    SearchFocus,
    ListFilterFocus,
    ReleaseFilterFocus,
    SearchHistory(String),
    SubmitSearch,
    OpenPage,
    Back,
    Refresh,
    OpenCurrent,
    CopyClone,
    SetTheme(String),
    StartProxyEdit,
    StartDownloadDirEdit,
    ToggleFileTree,
    ToggleReadme,
    Ttl(TtlKind, i32),
    ClearCache,
    SaveConfig,
    OpenDownloadDir,
    SetSettingsRow(usize),
    OpenLanguages,
    DownloadAsset { release: usize, asset: usize },
    DownloadSource,
    OpenReleasePage(usize),
    OpenReleaseDetail(usize),
    OpenTreePath { path: String, directory: bool },
    FocusRepoPane(RepoPane),
    OpenExternal(String),
    ToggleStar,
    CloseOverlay,
    PrevListPage,
    NextListPage,
    OpenRepoEdit,
    EditRepoRow(usize),
    SaveRepoEdit,
    RefreshRateLimit,
    OpenActions,
    SelectActionRun(usize),
    OpenCacheManager,
    ClearCacheGroup(usize),
    SelectCacheRow(usize),
    OpenCacheDir,
    OpenWorkflowRunner,
    SelectWorkflow(usize),
    EditWorkflowRef,
    EditWorkflowInputs,
    RunWorkflow,
    RefreshWorkflows,
    OpenIssues,
    OpenPullRequests,
    SelectIssue(usize),
    SetIssueKind(IssueKind),
    SetIssueState(IssueState),
    SelectNotification(usize),
    SetNotificationFilter(bool),
    MarkNotificationRead,
    MarkAllNotificationsRead,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitBox {
    pub rect: Rect,
    pub action: Action,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LayoutRects {
    pub list: Rect,
    pub detail: Rect,
    pub page_releases: Rect,
    pub page_tree: Rect,
    pub page_readme: Rect,
    pub settings: Rect,
    pub search: Rect,
    pub overlay: Rect,
}

impl LayoutRects {
    fn contains(&self, rect: Rect, x: u16, y: u16) -> bool {
        rect.width > 0
            && rect.height > 0
            && x >= rect.x
            && x < rect.x.saturating_add(rect.width)
            && y >= rect.y
            && y < rect.y.saturating_add(rect.height)
    }
}

#[derive(Debug, Clone)]
pub struct TextInput {
    pub value: String,
    pub cursor: usize,
}

impl TextInput {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.len();
        Self { value, cursor }
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    pub fn visible_window(&self, max_width: usize) -> (String, u16) {
        format::visible_text(&self.value, self.cursor, max_width)
    }

    pub fn insert(&mut self, ch: char) {
        self.value.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let previous = self.value[..self.cursor]
            .char_indices()
            .last()
            .map(|(index, _)| index)
            .unwrap_or(0);
        self.value.remove(previous);
        self.cursor = previous;
    }

    pub fn delete(&mut self) {
        if self.cursor >= self.value.len() {
            return;
        }
        self.value.remove(self.cursor);
    }

    pub fn left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.value[..self.cursor]
                .char_indices()
                .last()
                .map(|(index, _)| index)
                .unwrap_or(0);
        }
    }

    pub fn right(&mut self) {
        if let Some(ch) = self.value[self.cursor..].chars().next() {
            self.cursor += ch.len_utf8();
        }
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.value.len();
    }
}

struct PendingDetail {
    repo: Repo,
    at: Instant,
}

#[derive(Clone, Copy)]
pub struct CacheGroup {
    pub label: &'static str,
    pub prefixes: &'static [&'static str],
}

pub fn cache_groups() -> Vec<CacheGroup> {
    vec![
        CacheGroup {
            label: "仓库列表",
            prefixes: &["repos_"],
        },
        CacheGroup {
            label: "星星收藏",
            prefixes: &["starred_"],
        },
        CacheGroup {
            label: "搜索记录",
            prefixes: &["search_"],
        },
        CacheGroup {
            label: "仓库详情",
            prefixes: &["repo_", "releases_", "tree_", "readme_"],
        },
        CacheGroup {
            label: "工作流",
            prefixes: &["actions_"],
        },
        CacheGroup {
            label: "议题与合并请求",
            prefixes: &["issues_"],
        },
        CacheGroup {
            label: "通知",
            prefixes: &["notifications_"],
        },
        CacheGroup {
            label: "账号与额度",
            prefixes: &["user", "rate_limit"],
        },
        CacheGroup {
            label: "全部缓存",
            prefixes: &[""],
        },
    ]
}

#[derive(Clone)]
pub struct RepoEdit {
    pub full_name: String,
    pub description: TextInput,
    pub homepage: TextInput,
    pub topics: TextInput,
    pub private: bool,
    pub archived: bool,
    pub row: usize,
    pub editing: bool,
    pub saving: bool,
    pub error: Option<String>,
}

impl RepoEdit {
    pub fn from_repo(repo: &Repo) -> Self {
        Self {
            full_name: repo.display_full_name(),
            description: TextInput::new(repo.description.clone().unwrap_or_default()),
            homepage: TextInput::new(repo.homepage.clone().unwrap_or_default()),
            topics: TextInput::new(repo.topics.join(" ")),
            private: repo.private,
            archived: repo.archived,
            row: 0,
            editing: false,
            saving: false,
            error: None,
        }
    }

    pub fn topics_list(&self) -> Vec<String> {
        self.topics
            .value
            .split(|ch: char| ch.is_whitespace() || ch == ',')
            .map(str::trim)
            .filter(|topic| !topic.is_empty())
            .map(|topic| topic.trim_start_matches('#').to_ascii_lowercase())
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct DownloadJob {
    pub id: u64,
    pub label: String,
    pub received: u64,
    pub total: Option<u64>,
}

#[allow(clippy::large_enum_variant)]
enum Msg {
    User(Result<User, String>),
    Repos {
        login: String,
        result: Result<Vec<Repo>, String>,
    },
    Starred {
        login: String,
        result: Result<Vec<Repo>, String>,
    },
    Search {
        query: String,
        sort: SearchSort,
        result: Result<crate::model::SearchOutcome, String>,
    },
    Detail {
        name: String,
        result: Result<Repo, String>,
    },
    Releases {
        name: String,
        result: Result<Vec<Release>, String>,
    },
    Tree {
        name: String,
        branch: String,
        result: Result<TreeResponse, String>,
    },
    Readme {
        name: String,
        result: Result<ReadmeContent, String>,
    },
    StarToggled {
        name: String,
        starred: bool,
        result: Result<(), String>,
    },
    DownloadProgress {
        id: u64,
        received: u64,
        total: Option<u64>,
    },
    DownloadDone {
        id: u64,
        label: String,
        result: Result<std::path::PathBuf, String>,
    },
    RateLimit(Result<crate::model::RateLimit, String>),
    RepoUpdated(Result<Repo, String>),
    ActionRuns {
        name: String,
        result: Result<Vec<crate::model::WorkflowRun>, String>,
    },
    ActionJobs {
        name: String,
        run_id: u64,
        result: Result<Vec<crate::model::WorkflowJob>, String>,
    },
    Workflows {
        name: String,
        result: Result<Vec<Workflow>, String>,
    },
    WorkflowDispatched {
        name: String,
        workflow: String,
        git_ref: String,
        result: Result<(), String>,
    },
    Issues {
        name: String,
        state: IssueState,
        result: Result<Vec<Issue>, String>,
    },
    Notifications {
        all: bool,
        result: Result<Vec<Notification>, String>,
    },
    NotificationRead {
        id: String,
        result: Result<(), String>,
    },
    NotificationsRead {
        result: Result<(), String>,
    },
}

pub struct App {
    pub paths: Paths,
    pub config: Config,
    pub theme: Theme,
    pub client: ApiClient,
    pub cache: Cache,
    pub user: Option<User>,
    pub login: Option<String>,
    pub rate_limit: Option<RateLimit>,
    pub rate_loading: bool,
    pub rate_fetched_at: Option<i64>,
    pub action_repo: Option<String>,
    pub action_runs: Vec<crate::model::WorkflowRun>,
    pub action_runs_loading: bool,
    pub action_runs_fetched_at: Option<i64>,
    pub action_selected: usize,
    pub action_jobs: Vec<crate::model::WorkflowJob>,
    pub action_jobs_loading: Option<u64>,
    pub action_jobs_for: Option<u64>,
    pub action_jobs_fetched_at: Option<i64>,
    pub action_jobs_scroll: u16,
    pub action_jobs_scroll_max: u16,
    pub action_workflows: Vec<Workflow>,
    pub action_workflows_loading: bool,
    pub action_workflows_fetched_at: Option<i64>,
    pub workflow_selected: usize,
    pub workflow_ref: TextInput,
    pub workflow_ref_editing: bool,
    pub workflow_inputs: TextInput,
    pub workflow_inputs_editing: bool,
    pub workflow_running: bool,
    pub workflow_error: Option<String>,
    pub notifications: Vec<Notification>,
    pub notifications_loading: Option<bool>,
    pub notifications_fetched_at: Option<i64>,
    pub notifications_all: bool,
    pub notification_selected: usize,
    pub notification_marking: Option<String>,
    pub notifications_clearing: bool,
    pub issues: Vec<Issue>,
    pub issues_loading: Option<(String, IssueState)>,
    pub issues_fetched_at: Option<i64>,
    pub issue_repo: Option<Repo>,
    pub issue_selected: usize,
    pub issue_kind: IssueKind,
    pub issue_state: IssueState,
    pub cache_row: usize,
    pub page: Page,
    pub return_page: Page,
    pub overlay: Option<Overlay>,
    pub should_quit: bool,
    pub settings_only: bool,

    pub repos: Vec<Repo>,
    pub starred: Vec<Repo>,
    pub search_results: Vec<Repo>,
    pub search_total: Option<u64>,
    pub search_submitted: Option<String>,
    pub search_input: TextInput,
    pub search_focus: bool,
    pub repo_filter: TextInput,
    pub star_filter: TextInput,
    pub list_filter_focus: bool,
    pub release_filter: TextInput,
    pub release_filter_focus: bool,
    pub search_sort: SearchSort,
    pub repo_sort: RepoSort,
    pub star_sort: StarSort,
    pub language_filter: Option<String>,

    repo_selected: usize,
    star_selected: usize,
    search_selected: usize,
    pub list_offset: usize,
    pub viewport_cards: usize,

    pub detail: Option<Repo>,
    pub detail_for: Option<String>,
    pub detail_fetched_at: Option<i64>,
    pub releases: Vec<Release>,
    pub releases_fetched_at: Option<i64>,
    pub release_selected: usize,
    pub release_asset_selected: usize,
    pub release_scroll: u16,
    pub release_scroll_max: u16,
    pub detail_scroll: u16,
    pub detail_scroll_max: u16,
    pub page_repo: Option<Repo>,
    pub page_scroll: u16,
    pub page_scroll_max: u16,
    pub repo_pane: RepoPane,
    pub tree: Option<TreeResponse>,
    pub tree_for: Option<String>,
    pub tree_branch: String,
    pub tree_fetched_at: Option<i64>,
    pub tree_loading: Option<String>,
    pub tree_scroll: u16,
    pub tree_scroll_max: u16,
    pub readme: Option<ReadmeContent>,
    pub readme_for: Option<String>,
    pub readme_fetched_at: Option<i64>,
    pub readme_loading: Option<String>,
    pub readme_scroll: u16,
    pub readme_scroll_max: u16,

    pub repo_edit: Option<RepoEdit>,
    pub settings_row: usize,
    pub editing_proxy: bool,
    pub proxy_input: TextInput,
    pub editing_download_dir: bool,
    pub download_input: TextInput,
    pub config_dirty: bool,
    pub cache_stats: CacheStats,
    pub download_job: Option<DownloadJob>,
    pub star_loading: Option<String>,
    pub language_selected: usize,
    pub language_offset: usize,
    pub language_viewport: usize,

    pub user_loading: bool,
    pub repos_loading: bool,
    pub starred_loading: bool,
    pub search_loading: bool,
    pub detail_loading: Option<String>,
    pub releases_loading: Option<String>,

    pub status: Status,
    pub hitboxes: Vec<HitBox>,
    pub hover: Option<Action>,
    pub layout: LayoutRects,
    pub started_at: Instant,
    pub last_click: Option<(Instant, String)>,
    pub detail_scroll_reset: bool,

    download_seq: u64,
    fetched_repos_at: Option<i64>,
    fetched_starred_at: Option<i64>,
    pending_detail: Option<PendingDetail>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
}

impl App {
    pub fn new(paths: Paths, theme_override: Option<String>) -> Self {
        Self::build(paths, theme_override, false)
    }

    pub fn settings(paths: Paths, theme_override: Option<String>) -> Self {
        Self::build(paths, theme_override, true)
    }

    fn build(paths: Paths, theme_override: Option<String>, settings_only: bool) -> Self {
        let config = Config::load(&paths);
        let theme_name = theme_override
            .filter(|value| FLAVORS.contains(&value.as_str()))
            .unwrap_or_else(|| config.theme.clone());
        let theme = Theme::load(&paths, &theme_name);
        let client = if settings_only {
            ApiClient::without_auth(config.proxy.clone())
        } else {
            ApiClient::new(config.proxy.clone())
        };
        let cache = Cache::new(paths.cache_dir().to_path_buf());
        let cache_stats = cache.stats();
        let download_dir_initial = config.download_dir.clone();
        let (tx, rx) = mpsc::channel();
        let mut app = Self {
            paths,
            config,
            theme,
            client,
            cache,
            user: None,
            login: None,
            rate_limit: None,
            rate_loading: false,
            rate_fetched_at: None,
            action_repo: None,
            action_runs: Vec::new(),
            action_runs_loading: false,
            action_runs_fetched_at: None,
            action_selected: 0,
            action_jobs: Vec::new(),
            action_jobs_loading: None,
            action_jobs_for: None,
            action_jobs_fetched_at: None,
            action_jobs_scroll: 0,
            action_jobs_scroll_max: 0,
            action_workflows: Vec::new(),
            action_workflows_loading: false,
            action_workflows_fetched_at: None,
            workflow_selected: 0,
            workflow_ref: TextInput::new(""),
            workflow_ref_editing: false,
            workflow_inputs: TextInput::new("{}"),
            workflow_inputs_editing: false,
            workflow_running: false,
            workflow_error: None,
            notifications: Vec::new(),
            notifications_loading: None,
            notifications_fetched_at: None,
            notifications_all: false,
            notification_selected: 0,
            notification_marking: None,
            notifications_clearing: false,
            issues: Vec::new(),
            issues_loading: None,
            issues_fetched_at: None,
            issue_repo: None,
            issue_selected: 0,
            issue_kind: IssueKind::All,
            issue_state: IssueState::Open,
            cache_row: 0,
            page: if settings_only {
                Page::Settings
            } else {
                Page::Repositories
            },
            return_page: if settings_only {
                Page::Settings
            } else {
                Page::Repositories
            },
            overlay: None,
            should_quit: false,
            settings_only,
            repos: Vec::new(),
            starred: Vec::new(),
            search_results: Vec::new(),
            search_total: None,
            search_submitted: None,
            search_input: TextInput::new(""),
            search_focus: false,
            repo_filter: TextInput::new(""),
            star_filter: TextInput::new(""),
            list_filter_focus: false,
            release_filter: TextInput::new(""),
            release_filter_focus: false,
            search_sort: SearchSort::BestMatch,
            repo_sort: RepoSort::Updated,
            star_sort: StarSort::Starred,
            language_filter: None,
            repo_selected: 0,
            star_selected: 0,
            search_selected: 0,
            list_offset: 0,
            viewport_cards: 4,
            detail: None,
            detail_for: None,
            detail_fetched_at: None,
            releases: Vec::new(),
            releases_fetched_at: None,
            release_selected: 0,
            release_asset_selected: 0,
            release_scroll: 0,
            release_scroll_max: 0,
            detail_scroll: 0,
            detail_scroll_max: 0,
            page_repo: None,
            page_scroll: 0,
            page_scroll_max: 0,
            repo_pane: RepoPane::Releases,
            tree: None,
            tree_for: None,
            tree_branch: String::new(),
            tree_fetched_at: None,
            tree_loading: None,
            tree_scroll: 0,
            tree_scroll_max: 0,
            readme: None,
            readme_for: None,
            readme_fetched_at: None,
            readme_loading: None,
            readme_scroll: 0,
            readme_scroll_max: 0,
            repo_edit: None,
            settings_row: 0,
            editing_proxy: false,
            proxy_input: TextInput::new(""),
            editing_download_dir: false,
            download_input: TextInput::new(download_dir_initial),
            config_dirty: false,
            cache_stats,
            download_job: None,
            star_loading: None,
            language_selected: 0,
            language_offset: 0,
            language_viewport: 12,
            user_loading: false,
            repos_loading: false,
            starred_loading: false,
            search_loading: false,
            detail_loading: None,
            releases_loading: None,
            status: Status {
                text: if settings_only {
                    "全局设置喵~改完按 Ctrl+S 保存哦喵~".to_string()
                } else {
                    "准备就绪，人家先看看缓存里有什么喵~".to_string()
                },
                kind: StatusKind::Info,
            },
            hitboxes: Vec::new(),
            hover: None,
            layout: LayoutRects::default(),
            started_at: Instant::now(),
            last_click: None,
            detail_scroll_reset: false,
            download_seq: 0,
            fetched_repos_at: None,
            fetched_starred_at: None,
            pending_detail: None,
            tx,
            rx,
        };
        app.boot();
        app
    }

    fn boot(&mut self) {
        if self.settings_only {
            self.refresh_cache_stats();
            return;
        }
        if !self.client.token_present() {
            self.set_status(
                StatusKind::Error,
                "还没有登录 GitHub，只能看公开内容哦喵~先在终端跑一下 gh auth login 嘛喵~",
            );
        }

        if let Some(cached) = self.cache.load::<User>("user") {
            self.user = Some(cached.data.clone());
            self.login = Some(cached.data.login.clone());
            self.load_account_cache();
        }
        if let Some(cached) = self.cache.load::<RateLimit>("rate_limit") {
            self.rate_limit = Some(cached.data);
            self.rate_fetched_at = Some(cached.fetched_at);
        }

        if self.client.token_present() {
            self.fetch_user();
            self.fetch_rate_limit(false);
            if self.login.is_some() {
                self.fetch_repos_if_needed(false);
                self.fetch_starred_if_needed(false);
            }
        }

        self.schedule_selected_detail();
        self.refresh_cache_stats();
    }

    fn load_account_cache(&mut self) {
        let Some(login) = self.login.clone() else {
            return;
        };
        if let Some(cached) = self.cache.load::<Vec<Repo>>(&repos_cache_key(&login)) {
            self.repos = cached.data;
            self.fetched_repos_at = Some(cached.fetched_at);
        }
        if let Some(cached) = self.cache.load::<Vec<Repo>>(&starred_cache_key(&login)) {
            self.starred = cached.data;
            self.fetched_starred_at = Some(cached.fetched_at);
        }
        if self.client.token_present() && self.starred.iter().any(|repo| repo.starred_at.is_none())
        {
            self.fetched_starred_at = None;
        }
        self.clamp_selection();
        self.schedule_selected_detail();
    }

    pub fn drain_messages(&mut self) {
        while let Ok(message) = self.rx.try_recv() {
            self.handle_message(message);
        }
    }

    fn handle_message(&mut self, message: Msg) {
        match message {
            Msg::User(result) => {
                self.user_loading = false;
                match result {
                    Ok(user) => {
                        let login = user.login.clone();
                        let changed = self.login.as_deref() != Some(login.as_str());
                        self.user = Some(user);
                        self.login = Some(login.clone());
                        let _ = self.cache.save("user", self.user.as_ref().unwrap());
                        if changed {
                            self.repos.clear();
                            self.starred.clear();
                            self.repo_selected = 0;
                            self.star_selected = 0;
                            self.fetched_repos_at = None;
                            self.fetched_starred_at = None;
                            self.detail = None;
                            self.detail_for = None;
                            self.releases.clear();
                            self.load_account_cache();
                        }
                        self.set_status(
                            StatusKind::Ok,
                            &format!("已经连上 GitHub，当前账号是 {}喵~", login),
                        );
                        if self.repos.is_empty() || self.repos_stale() {
                            self.fetch_repos_if_needed(false);
                        }
                        if self.starred.is_empty() || self.starred_stale() {
                            self.fetch_starred_if_needed(false);
                        }
                    }
                    Err(error) => {
                        self.set_status(StatusKind::Error, &error);
                    }
                }
            }
            Msg::Repos { login, result } => {
                if self.login.as_deref() != Some(login.as_str()) {
                    return;
                }
                self.repos_loading = false;
                match result {
                    Ok(repos) => {
                        let selected_name =
                            self.selected_repo().map(|repo| repo.display_full_name());
                        self.repos = repos;
                        self.fetched_repos_at = Some(now_epoch());
                        let _ = self.cache.save(&repos_cache_key(&login), &self.repos);
                        self.restore_selection(selected_name);
                        self.clamp_selection();
                        self.schedule_selected_detail();
                        self.refresh_cache_stats();
                        self.set_status(
                            StatusKind::Ok,
                            &format!("仓库都搬过来啦，一共 {} 个呢喵~", self.repos.len()),
                        );
                    }
                    Err(error) => self.set_status(StatusKind::Error, &error),
                }
            }
            Msg::Starred { login, result } => {
                if self.login.as_deref() != Some(login.as_str()) {
                    return;
                }
                self.starred_loading = false;
                match result {
                    Ok(repos) => {
                        let selected_name =
                            self.selected_repo().map(|repo| repo.display_full_name());
                        self.starred = repos;
                        self.fetched_starred_at = Some(now_epoch());
                        let _ = self.cache.save(&starred_cache_key(&login), &self.starred);
                        self.restore_selection(selected_name);
                        self.clamp_selection();
                        self.schedule_selected_detail();
                        self.refresh_cache_stats();
                        self.set_status(
                            StatusKind::Ok,
                            &format!("星星都整理好啦，一共 {} 个哦喵~", self.starred.len()),
                        );
                    }
                    Err(error) => self.set_status(StatusKind::Error, &error),
                }
            }
            Msg::Search {
                query,
                sort,
                result,
            } => {
                if self.search_submitted.as_deref() != Some(query.as_str())
                    || self.search_sort != sort
                {
                    return;
                }
                self.search_loading = false;
                match result {
                    Ok(outcome) => {
                        self.search_results = outcome.items;
                        self.search_total = Some(outcome.total);
                        self.search_selected = 0;
                        self.list_offset = 0;
                        let key = search_key(&query, sort.key());
                        let response = SearchResponse {
                            total_count: outcome.total,
                            incomplete_results: false,
                            items: self.search_results.clone(),
                        };
                        let _ = self.cache.save(&key, &response);
                        self.refresh_cache_stats();
                        self.schedule_selected_detail();
                        if self.search_results.is_empty() {
                            self.set_status(
                                StatusKind::Info,
                                "没有找到符合条件的仓库呢，换个词试试嘛喵~",
                            );
                        } else {
                            self.set_status(
                                StatusKind::Ok,
                                &format!(
                                    "找到 {} 个仓库，人家先摆出前 100 个，可以分页翻喵~",
                                    outcome.total
                                ),
                            );
                        }
                    }
                    Err(error) => self.set_status(StatusKind::Error, &error),
                }
            }
            Msg::Detail { name, result } => {
                if self.detail_loading.as_deref() == Some(name.as_str()) {
                    self.detail_loading = None;
                }
                match result {
                    Ok(repo) => {
                        let key = repo.cache_key();
                        let _ = self.cache.save(&key, &repo);
                        self.refresh_cache_stats();
                        if self.matches_current_repo(&name) {
                            let extra_repo = repo.clone();
                            self.detail = Some(repo);
                            self.detail_for = Some(name);
                            self.detail_fetched_at = Some(now_epoch());
                            if self.page == Page::Repo {
                                self.schedule_repo_extras(&extra_repo, false);
                            }
                        }
                    }
                    Err(error) => {
                        if self.matches_current_repo(&name) {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
                }
            }
            Msg::Releases { name, result } => {
                if self.releases_loading.as_deref() == Some(name.as_str()) {
                    self.releases_loading = None;
                }
                match result {
                    Ok(releases) => {
                        let key = format!("releases_{}", key_part(&name));
                        let _ = self.cache.save(&key, &releases);
                        self.refresh_cache_stats();
                        if self.matches_current_repo(&name) {
                            self.releases = releases;
                            self.releases_fetched_at = Some(now_epoch());
                            self.clamp_release_selection();
                        }
                    }
                    Err(error) => {
                        if self.matches_current_repo(&name) {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
                }
            }
            Msg::Tree {
                name,
                branch,
                result,
            } => {
                if self.tree_loading.as_deref() == Some(name.as_str()) {
                    self.tree_loading = None;
                }
                match result {
                    Ok(tree) => {
                        let _ = self.cache.save(&tree_cache_key(&name, &branch), &tree);
                        self.refresh_cache_stats();
                        if self.matches_current_repo(&name) {
                            self.tree = Some(tree);
                            self.tree_for = Some(name);
                            self.tree_branch = branch;
                            self.tree_fetched_at = Some(now_epoch());
                        }
                    }
                    Err(error) => {
                        if self.matches_current_repo(&name) {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
                }
            }
            Msg::Readme { name, result } => {
                if self.readme_loading.as_deref() == Some(name.as_str()) {
                    self.readme_loading = None;
                }
                match result {
                    Ok(readme) => {
                        let _ = self.cache.save(&readme_cache_key(&name), &readme);
                        self.refresh_cache_stats();
                        if self.matches_current_repo(&name) {
                            self.readme = Some(readme);
                            self.readme_for = Some(name);
                            self.readme_fetched_at = Some(now_epoch());
                        }
                    }
                    Err(error) => {
                        if self.matches_current_repo(&name) {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
                }
            }
            Msg::StarToggled {
                name,
                starred,
                result,
            } => {
                if self.star_loading.as_deref() == Some(name.as_str()) {
                    self.star_loading = None;
                }
                match result {
                    Ok(()) => {
                        if starred {
                            if let Some(repo) = self
                                .page_repo
                                .clone()
                                .or_else(|| self.selected_repo())
                                .or_else(|| self.detail.clone())
                                && !self
                                    .starred
                                    .iter()
                                    .any(|item| item.display_full_name() == name)
                            {
                                self.starred.insert(0, repo);
                            }
                        } else {
                            self.starred.retain(|item| item.display_full_name() != name);
                        }
                        if let Some(login) = self.login.clone() {
                            let _ = self.cache.save(&starred_cache_key(&login), &self.starred);
                        }
                        self.clamp_selection();
                        self.refresh_cache_stats();
                        self.set_status(
                            StatusKind::Ok,
                            if starred {
                                "收藏好啦，这颗星星已经记在人家的小本本上啦喵~"
                            } else {
                                "取消收藏啦，星星还给 GitHub 了喵~"
                            },
                        );
                        self.schedule_selected_detail();
                    }
                    Err(error) => self.set_status(StatusKind::Error, &error),
                }
            }
            Msg::RateLimit(result) => {
                self.rate_loading = false;
                match result {
                    Ok(rate) => {
                        self.rate_limit = Some(rate.clone());
                        self.rate_fetched_at = Some(now_epoch());
                        let _ = self.cache.save("rate_limit", &rate);
                        self.refresh_cache_stats();
                    }
                    Err(error) => {
                        if self.page == Page::Account {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
                }
            }
            Msg::RepoUpdated(result) => {
                if let Some(edit) = self.repo_edit.as_mut() {
                    edit.saving = false;
                }
                match result {
                    Ok(repo) => {
                        self.apply_updated_repo(repo);
                        self.repo_edit = None;
                        self.overlay = None;
                        self.set_status(StatusKind::Ok, "仓库信息改好啦，GitHub 那边已经记上了喵~");
                    }
                    Err(error) => {
                        if let Some(edit) = self.repo_edit.as_mut() {
                            edit.error = Some(error.clone());
                        } else {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
                }
            }
            Msg::ActionRuns { name, result } => {
                self.action_runs_loading = false;
                if self.action_repo.as_deref() != Some(name.as_str()) {
                    return;
                }
                match result {
                    Ok(runs) => {
                        let _ = self.cache.save(&actions_cache_key(&name), &runs);
                        self.action_runs = runs;
                        self.action_runs_fetched_at = Some(now_epoch());
                        self.action_selected = self
                            .action_selected
                            .min(self.action_runs.len().saturating_sub(1));
                        self.refresh_cache_stats();
                        self.schedule_action_jobs(false);
                    }
                    Err(error) => {
                        if self.page == Page::Actions {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
                }
            }
            Msg::ActionJobs {
                name,
                run_id,
                result,
            } => {
                if self.action_jobs_loading == Some(run_id) {
                    self.action_jobs_loading = None;
                }
                if self.action_repo.as_deref() != Some(name.as_str()) {
                    return;
                }
                match result {
                    Ok(jobs) => {
                        let _ = self.cache.save(&action_jobs_cache_key(run_id), &jobs);
                        if self.selected_action_run().map(|run| run.id) == Some(run_id) {
                            self.action_jobs = jobs;
                            self.action_jobs_for = Some(run_id);
                            self.action_jobs_fetched_at = Some(now_epoch());
                            self.action_jobs_scroll = 0;
                            self.action_jobs_scroll_max = 0;
                        }
                        self.refresh_cache_stats();
                    }
                    Err(error) => {
                        if self.page == Page::Actions {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
                }
            }
            Msg::Workflows { name, result } => {
                self.action_workflows_loading = false;
                if self.action_repo.as_deref() != Some(name.as_str()) {
                    return;
                }
                match result {
                    Ok(workflows) => {
                        let _ = self.cache.save(&workflows_cache_key(&name), &workflows);
                        self.action_workflows = workflows;
                        self.action_workflows_fetched_at = Some(now_epoch());
                        self.workflow_selected = self
                            .workflow_selected
                            .min(self.action_workflows.len().saturating_sub(1));
                        self.workflow_error = None;
                        self.refresh_cache_stats();
                    }
                    Err(error) => {
                        self.workflow_error = Some(error.clone());
                        self.set_status(StatusKind::Error, &error);
                    }
                }
            }
            Msg::WorkflowDispatched {
                name,
                workflow,
                git_ref,
                result,
            } => {
                self.workflow_running = false;
                if self.action_repo.as_deref() != Some(name.as_str()) {
                    return;
                }
                match result {
                    Ok(()) => {
                        self.overlay = None;
                        self.workflow_ref_editing = false;
                        self.workflow_error = None;
                        self.set_status(
                            StatusKind::Ok,
                            &format!(
                                "{workflow} 已经在 {git_ref} 上出发啦喵~等几秒按 r 刷新记录喵~"
                            ),
                        );
                        self.fetch_action_runs(true);
                    }
                    Err(error) => {
                        self.workflow_error = Some(error.clone());
                        self.set_status(StatusKind::Error, &error);
                    }
                }
            }
            Msg::Issues {
                name,
                state,
                result,
            } => {
                let key = (name.clone(), state);
                if self.issues_loading.as_ref() == Some(&key) {
                    self.issues_loading = None;
                }
                let current = self
                    .issue_repo
                    .as_ref()
                    .map(|repo| repo.display_full_name());
                if current.as_deref() != Some(name.as_str()) || self.issue_state != state {
                    return;
                }
                match result {
                    Ok(issues) => {
                        let _ = self.cache.save(&issues_cache_key(&name, state), &issues);
                        self.issues = issues;
                        self.issues_fetched_at = Some(now_epoch());
                        self.clamp_issue_selection();
                        self.refresh_cache_stats();
                        if self.page == Page::Issues {
                            self.set_status(
                                StatusKind::Ok,
                                &format!(
                                    "议题与合并请求更新好啦，当前筛选下有 {} 条喵~",
                                    self.issue_indices().len()
                                ),
                            );
                        }
                    }
                    Err(error) => {
                        if self.page == Page::Issues {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
                }
            }
            Msg::Notifications { all, result } => {
                if self.notifications_loading == Some(all) {
                    self.notifications_loading = None;
                }
                if self.notifications_all != all {
                    return;
                }
                match result {
                    Ok(items) => {
                        let _ = self.cache.save(&notifications_cache_key(all), &items);
                        self.notifications = items;
                        self.notifications_fetched_at = Some(now_epoch());
                        self.clamp_notification_selection();
                        self.refresh_cache_stats();
                        if self.page == Page::Notifications {
                            self.set_status(
                                StatusKind::Ok,
                                &format!("通知都拿来啦，一共 {} 条喵~", self.notifications.len()),
                            );
                        }
                    }
                    Err(error) => {
                        if self.page == Page::Notifications {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
                }
            }
            Msg::NotificationRead { id, result } => {
                if self.notification_marking.as_deref() == Some(id.as_str()) {
                    self.notification_marking = None;
                }
                match result {
                    Ok(()) => {
                        if let Some(item) = self.notifications.iter_mut().find(|item| item.id == id)
                        {
                            item.unread = false;
                        }
                        if !self.notifications_all {
                            self.notifications.retain(|item| item.id != id);
                        }
                        self.clamp_notification_selection();
                        self.save_notifications_cache();
                        self.set_status(StatusKind::Ok, "这条通知已经读过啦喵~");
                    }
                    Err(error) => self.set_status(StatusKind::Error, &error),
                }
            }
            Msg::NotificationsRead { result } => {
                self.notifications_clearing = false;
                match result {
                    Ok(()) => {
                        for item in &mut self.notifications {
                            item.unread = false;
                        }
                        if !self.notifications_all {
                            self.notifications.clear();
                        }
                        self.clamp_notification_selection();
                        self.save_notifications_cache();
                        self.set_status(StatusKind::Ok, "所有通知都标成已读啦喵~");
                    }
                    Err(error) => self.set_status(StatusKind::Error, &error),
                }
            }
            Msg::DownloadProgress {
                id,
                received,
                total,
            } => {
                if let Some(job) = self.download_job.as_mut()
                    && job.id == id
                {
                    job.received = received;
                    job.total = total;
                }
            }
            Msg::DownloadDone { id, label, result } => {
                if self.download_job.as_ref().is_some_and(|job| job.id == id) {
                    self.download_job = None;
                }
                match result {
                    Ok(path) => self.set_status(
                        StatusKind::Ok,
                        &format!(
                            "{label} 下载好啦，人家放在 {} 啦喵~",
                            format::display_path(&path)
                        ),
                    ),
                    Err(error) => self.set_status(StatusKind::Error, &error),
                }
            }
        }
    }

    fn matches_current_repo(&self, name: &str) -> bool {
        self.page_repo
            .as_ref()
            .map(|repo| repo.display_full_name() == name)
            .unwrap_or(false)
            || self
                .selected_repo()
                .map(|repo| repo.display_full_name() == name)
                .unwrap_or(false)
    }

    fn apply_updated_repo(&mut self, repo: Repo) {
        let name = repo.display_full_name();
        let key = repo.cache_key();
        let _ = self.cache.save(&key, &repo);
        for list in [&mut self.repos, &mut self.starred, &mut self.search_results] {
            for item in list.iter_mut() {
                if item.display_full_name() == name {
                    *item = repo.clone();
                }
            }
        }
        if self
            .detail
            .as_ref()
            .is_some_and(|detail| detail.display_full_name() == name)
        {
            self.detail = Some(repo.clone());
            self.detail_fetched_at = Some(now_epoch());
        }
        if self
            .page_repo
            .as_ref()
            .is_some_and(|page_repo| page_repo.display_full_name() == name)
        {
            self.page_repo = Some(repo);
        }
        if let Some(login) = self.login.clone() {
            let _ = self.cache.save(&repos_cache_key(&login), &self.repos);
            let _ = self.cache.save(&starred_cache_key(&login), &self.starred);
        }
        self.refresh_cache_stats();
        self.clamp_selection();
        self.schedule_selected_detail();
    }

    fn fetch_rate_limit(&mut self, force: bool) {
        if self.rate_loading {
            return;
        }
        if !force
            && self
                .rate_fetched_at
                .map(|at| now_epoch() - at <= 60)
                .unwrap_or(false)
        {
            return;
        }
        self.rate_loading = true;
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Msg::RateLimit(client.rate_limit()));
        });
    }

    fn repo_by_name(&self, name: &str) -> Option<Repo> {
        let matches = |repo: &&Repo| repo.display_full_name() == name;
        self.page_repo
            .as_ref()
            .filter(|repo| repo.display_full_name() == name)
            .cloned()
            .or_else(|| {
                self.detail
                    .as_ref()
                    .filter(|repo| repo.display_full_name() == name)
                    .cloned()
            })
            .or_else(|| {
                self.repos
                    .iter()
                    .chain(self.starred.iter())
                    .chain(self.search_results.iter())
                    .find(matches)
                    .cloned()
            })
    }

    fn context_repo(&self) -> Option<Repo> {
        match self.page {
            Page::Repo | Page::ReleaseDetail => {
                self.page_repo.clone().or_else(|| self.detail.clone())
            }
            Page::Actions => self
                .action_repo
                .as_deref()
                .and_then(|name| self.repo_by_name(name))
                .or_else(|| self.page_repo.clone()),
            Page::Issues => self.issue_repo.clone(),
            Page::Notifications | Page::Settings => None,
            _ => self
                .detail
                .clone()
                .filter(|detail| {
                    self.selected_repo()
                        .map(|repo| repo.display_full_name() == detail.display_full_name())
                        .unwrap_or(false)
                })
                .or_else(|| self.selected_repo()),
        }
    }

    pub fn open_actions(&mut self) {
        let Some(repo) = self.context_repo() else {
            self.set_status(
                StatusKind::Info,
                "先挑一个仓库嘛，人家才知道看哪个工作流喵~",
            );
            return;
        };
        let name = repo.display_full_name();
        if self.action_repo.as_deref() != Some(name.as_str()) {
            self.action_runs.clear();
            self.action_runs_fetched_at = None;
            self.action_selected = 0;
            self.action_jobs.clear();
            self.action_jobs_for = None;
            self.action_jobs_fetched_at = None;
            self.action_jobs_scroll = 0;
            self.action_jobs_scroll_max = 0;
            self.action_workflows.clear();
            self.action_workflows_fetched_at = None;
            self.workflow_selected = 0;
            self.workflow_error = None;
        }
        self.action_repo = Some(name.clone());
        self.return_page = return_page_when_opening(self.page, self.return_page);
        self.page = Page::Actions;
        self.load_actions_cache();
        self.fetch_action_runs(false);
    }

    fn load_actions_cache(&mut self) {
        let Some(name) = self.action_repo.clone() else {
            return;
        };
        if let Some(cached) = self
            .cache
            .load::<Vec<crate::model::WorkflowRun>>(&actions_cache_key(&name))
        {
            self.action_runs = cached.data;
            self.action_runs_fetched_at = Some(cached.fetched_at);
        }
        self.action_selected = self
            .action_selected
            .min(self.action_runs.len().saturating_sub(1));
        self.schedule_action_jobs(false);
    }

    fn fetch_action_runs(&mut self, force: bool) {
        if self.action_runs_loading {
            return;
        }
        let Some(name) = self.action_repo.clone() else {
            return;
        };
        let fresh = self
            .action_runs_fetched_at
            .map(|at| now_epoch() - at <= self.config.detail_ttl_secs as i64)
            .unwrap_or(false);
        if !force && fresh {
            return;
        }
        let private = self
            .repo_by_name(&name)
            .map(|repo| repo.private)
            .unwrap_or(false);
        self.action_runs_loading = true;
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.action_runs(&name, private);
            let _ = tx.send(Msg::ActionRuns { name, result });
        });
    }

    pub fn selected_action_run(&self) -> Option<crate::model::WorkflowRun> {
        self.action_runs.get(self.action_selected).cloned()
    }

    fn schedule_action_jobs(&mut self, force: bool) {
        let Some(run) = self.selected_action_run() else {
            self.action_jobs.clear();
            self.action_jobs_for = None;
            self.action_jobs_fetched_at = None;
            return;
        };
        if self.action_jobs_for != Some(run.id) {
            self.action_jobs.clear();
            self.action_jobs_for = Some(run.id);
            self.action_jobs_fetched_at = None;
            self.action_jobs_scroll = 0;
            self.action_jobs_scroll_max = 0;
            if let Some(cached) = self
                .cache
                .load::<Vec<crate::model::WorkflowJob>>(&action_jobs_cache_key(run.id))
            {
                self.action_jobs = cached.data;
                self.action_jobs_fetched_at = Some(cached.fetched_at);
            }
        }
        let fresh = self
            .action_jobs_fetched_at
            .map(|at| now_epoch() - at <= self.config.detail_ttl_secs as i64)
            .unwrap_or(false);
        if force || !fresh {
            self.fetch_action_jobs(run.id);
        }
    }

    fn fetch_action_jobs(&mut self, run_id: u64) {
        let Some(name) = self.action_repo.clone() else {
            return;
        };
        if self.action_jobs_loading == Some(run_id) {
            return;
        }
        let private = self
            .repo_by_name(&name)
            .map(|repo| repo.private)
            .unwrap_or(false);
        self.action_jobs_loading = Some(run_id);
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.action_jobs(&name, run_id, private);
            let _ = tx.send(Msg::ActionJobs {
                name,
                run_id,
                result,
            });
        });
    }

    pub fn move_action_selection(&mut self, delta: i32) {
        if self.action_runs.is_empty() {
            return;
        }
        let next = (self.action_selected as i32 + delta).clamp(0, self.action_runs.len() as i32 - 1)
            as usize;
        if next != self.action_selected {
            self.action_selected = next;
            self.schedule_action_jobs(false);
        }
    }

    pub fn select_action_run(&mut self, index: usize) {
        if index < self.action_runs.len() {
            self.action_selected = index;
            self.schedule_action_jobs(false);
        }
    }

    pub fn open_action_run(&mut self, index: Option<usize>) {
        let run = index
            .and_then(|index| self.action_runs.get(index).cloned())
            .or_else(|| self.selected_action_run());
        if let Some(run) = run {
            self.open_url(&run.html_url);
        }
    }

    pub fn open_workflow_runner(&mut self) {
        if self.action_repo.is_none() {
            self.set_status(StatusKind::Info, "先打开一个仓库的工作流页嘛喵~");
            return;
        }
        if !self.client.token_present() {
            self.set_status(
                StatusKind::Error,
                "还没有登录 GitHub，跑不了工作流哦喵~先运行 gh auth login 吧喵~",
            );
            return;
        }
        let repo = self.page_repo.clone().or_else(|| self.selected_repo());
        let branch = repo
            .as_ref()
            .map(|repo| repo.default_branch.clone())
            .filter(|branch| !branch.trim().is_empty())
            .or_else(|| {
                self.selected_action_run().and_then(|run| {
                    let branch = run.branch().to_string();
                    (!branch.is_empty()).then_some(branch)
                })
            })
            .unwrap_or_else(|| "main".to_string());
        self.workflow_ref = TextInput::new(&branch);
        self.workflow_ref_editing = false;
        self.workflow_inputs = TextInput::new("{}");
        self.workflow_inputs_editing = false;
        self.workflow_running = false;
        self.workflow_error = None;
        self.overlay = Some(Overlay::RunWorkflow);
        self.load_workflows_cache();
        self.fetch_workflows(false);
    }

    fn load_workflows_cache(&mut self) {
        let Some(name) = self.action_repo.clone() else {
            return;
        };
        match self
            .cache
            .load::<Vec<Workflow>>(&workflows_cache_key(&name))
        {
            Some(cached) => {
                self.action_workflows = cached.data;
                self.action_workflows_fetched_at = Some(cached.fetched_at);
            }
            None => {
                self.action_workflows.clear();
                self.action_workflows_fetched_at = None;
            }
        }
        self.workflow_selected = self
            .workflow_selected
            .min(self.action_workflows.len().saturating_sub(1));
    }

    fn fetch_workflows(&mut self, force: bool) {
        if self.action_workflows_loading {
            return;
        }
        let Some(name) = self.action_repo.clone() else {
            return;
        };
        let fresh = self
            .action_workflows_fetched_at
            .map(|at| now_epoch() - at <= self.config.detail_ttl_secs as i64)
            .unwrap_or(false);
        if !force && fresh {
            return;
        }
        let private = self
            .repo_by_name(&name)
            .map(|repo| repo.private)
            .unwrap_or(false);
        self.action_workflows_loading = true;
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.workflows(&name, private);
            let _ = tx.send(Msg::Workflows { name, result });
        });
    }

    pub fn move_workflow_selection(&mut self, delta: i32) {
        if self.action_workflows.is_empty() {
            return;
        }
        self.workflow_selected = (self.workflow_selected as i32 + delta)
            .clamp(0, self.action_workflows.len() as i32 - 1)
            as usize;
    }

    pub fn selected_workflow(&self) -> Option<Workflow> {
        self.action_workflows.get(self.workflow_selected).cloned()
    }

    pub fn dispatch_selected_workflow(&mut self) {
        if self.workflow_running {
            return;
        }
        let Some(name) = self.action_repo.clone() else {
            self.set_status(StatusKind::Info, "先打开一个仓库的工作流页嘛喵~");
            return;
        };
        let Some(workflow) = self.selected_workflow() else {
            let error = if self.action_workflows_loading {
                "工作流列表还在路上喵~稍等一下再按 Enter 吧喵~".to_string()
            } else {
                "这里还没有能运行的工作流喵~先往 .github/workflows 放一个吧喵~".to_string()
            };
            self.workflow_error = Some(error.clone());
            self.set_status(StatusKind::Error, &error);
            return;
        };
        let git_ref = self.workflow_ref.value.trim().to_string();
        if git_ref.is_empty() {
            self.workflow_error = Some("分支或标签不能空着喵~".to_string());
            self.workflow_ref_editing = true;
            return;
        }
        let inputs = match parse_workflow_inputs(&self.workflow_inputs.value) {
            Ok(inputs) => inputs,
            Err(error) => {
                self.workflow_error = Some(error.clone());
                self.workflow_inputs_editing = true;
                return;
            }
        };
        let workflow_name = if workflow.name.trim().is_empty() {
            workflow.file_name().to_string()
        } else {
            workflow.name.clone()
        };
        self.workflow_running = true;
        self.workflow_error = None;
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.dispatch_workflow(&name, workflow.id, &git_ref, inputs);
            let _ = tx.send(Msg::WorkflowDispatched {
                name,
                workflow: workflow_name,
                git_ref,
                result,
            });
        });
    }

    fn handle_workflow_key(&mut self, key: KeyEvent) {
        if self.workflow_running {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
                self.overlay = None;
            }
            return;
        }
        if self.workflow_ref_editing {
            match key.code {
                KeyCode::Enter | KeyCode::Char('\r') | KeyCode::Char('\n') => {
                    self.workflow_ref_editing = false;
                }
                KeyCode::Tab => {
                    self.workflow_ref_editing = false;
                    self.workflow_inputs_editing = true;
                }
                KeyCode::Esc => self.workflow_ref_editing = false,
                KeyCode::Backspace => self.workflow_ref.backspace(),
                KeyCode::Delete => self.workflow_ref.delete(),
                KeyCode::Left => self.workflow_ref.left(),
                KeyCode::Right => self.workflow_ref.right(),
                KeyCode::Home => self.workflow_ref.home(),
                KeyCode::End => self.workflow_ref.end(),
                KeyCode::Char(ch) => self.workflow_ref.insert(ch),
                _ => {}
            }
            return;
        }
        if self.workflow_inputs_editing {
            match key.code {
                KeyCode::Enter | KeyCode::Char('\r') | KeyCode::Char('\n') => {
                    self.workflow_inputs_editing = false;
                }
                KeyCode::Tab => {
                    self.workflow_inputs_editing = false;
                    self.workflow_ref_editing = true;
                }
                KeyCode::Esc => self.workflow_inputs_editing = false,
                KeyCode::Backspace => self.workflow_inputs.backspace(),
                KeyCode::Delete => self.workflow_inputs.delete(),
                KeyCode::Left => self.workflow_inputs.left(),
                KeyCode::Right => self.workflow_inputs.right(),
                KeyCode::Home => self.workflow_inputs.home(),
                KeyCode::End => self.workflow_inputs.end(),
                KeyCode::Char(ch) => self.workflow_inputs.insert(ch),
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.overlay = None;
                self.workflow_error = None;
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_workflow_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_workflow_selection(1),
            KeyCode::Home | KeyCode::Char('g') => self.workflow_selected = 0,
            KeyCode::End | KeyCode::Char('G') => {
                self.workflow_selected = self.action_workflows.len().saturating_sub(1);
            }
            KeyCode::Tab | KeyCode::Char('i') => {
                self.workflow_ref_editing = true;
                self.workflow_inputs_editing = false;
            }
            KeyCode::Enter | KeyCode::Char('\r') | KeyCode::Char('\n') => {
                self.dispatch_selected_workflow();
            }
            KeyCode::Char('r') => self.fetch_workflows(true),
            KeyCode::Char('o') => {
                if let Some(workflow) = self.selected_workflow() {
                    self.open_external_url(&workflow.html_url);
                }
            }
            _ => {}
        }
    }

    pub fn open_cache_manager(&mut self) {
        self.cache_row = 0;
        self.overlay = Some(Overlay::Cache);
        self.refresh_cache_stats();
    }

    pub fn move_cache_row(&mut self, delta: i32) {
        let len = cache_groups().len();
        self.cache_row = (self.cache_row as i32 + delta).clamp(0, len as i32 - 1) as usize;
    }

    pub fn clear_cache_group(&mut self, index: usize) {
        let groups = cache_groups();
        let Some(group) = groups.get(index) else {
            return;
        };
        let result = if group.prefixes.contains(&"") {
            self.cache.clear().map(|()| CacheStats {
                files: self.cache_stats.files,
                bytes: self.cache_stats.bytes,
            })
        } else {
            self.cache.clear_prefixes(group.prefixes)
        };
        match result {
            Ok(removed) => {
                self.refresh_cache_stats();
                self.set_status(
                    StatusKind::Ok,
                    &format!(
                        "{}清掉啦，删了 {} 个文件、{} 喵~",
                        group.label,
                        removed.files,
                        format::format_size(removed.bytes)
                    ),
                );
            }
            Err(error) => self.set_status(StatusKind::Error, &format!("缓存没清掉呢：{error}喵~")),
        }
    }

    pub fn cache_group_stats(&self, index: usize) -> CacheStats {
        cache_groups()
            .get(index)
            .map(|group| self.cache.stats_prefixes(group.prefixes))
            .unwrap_or_default()
    }

    pub fn open_cache_dir(&mut self) {
        let dir = self.cache.dir().display().to_string();
        match open_external(&dir) {
            Ok(()) => self.set_status(StatusKind::Ok, "缓存目录已经打开啦喵~"),
            Err(error) => self.set_status(StatusKind::Error, &error),
        }
    }

    fn load_notifications_cache(&mut self) {
        let all = self.notifications_all;
        match self
            .cache
            .load::<Vec<Notification>>(&notifications_cache_key(all))
        {
            Some(cached) => {
                self.notifications = cached.data;
                self.notifications_fetched_at = Some(cached.fetched_at);
            }
            None => {
                self.notifications.clear();
                self.notifications_fetched_at = None;
            }
        }
        self.clamp_notification_selection();
    }

    fn save_notifications_cache(&mut self) {
        let all = self.notifications_all;
        let _ = self
            .cache
            .save(&notifications_cache_key(all), &self.notifications);
        self.refresh_cache_stats();
    }

    fn fetch_notifications(&mut self, force: bool) {
        let all = self.notifications_all;
        if self.notifications_loading == Some(all) {
            return;
        }
        if !self.client.token_present() {
            if self.page == Page::Notifications {
                self.set_status(
                    StatusKind::Error,
                    "还没登录 GitHub，看不到通知喵~先运行 gh auth login 吧喵~",
                );
            }
            return;
        }
        let fresh = self
            .notifications_fetched_at
            .map(|at| now_epoch() - at <= self.config.detail_ttl_secs as i64)
            .unwrap_or(false);
        if !force && fresh {
            return;
        }
        self.notifications_loading = Some(all);
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.notifications(all);
            let _ = tx.send(Msg::Notifications { all, result });
        });
    }

    pub fn set_notification_filter(&mut self, all: bool) {
        if self.notifications_all == all {
            return;
        }
        self.notifications_all = all;
        self.notifications.clear();
        self.notifications_fetched_at = None;
        self.notification_selected = 0;
        self.load_notifications_cache();
        self.fetch_notifications(false);
        self.set_status(
            StatusKind::Info,
            if all {
                "现在看全部通知喵~"
            } else {
                "现在只看未读通知喵~"
            },
        );
    }

    pub fn toggle_notification_filter(&mut self) {
        self.set_notification_filter(!self.notifications_all);
    }

    pub fn move_notification_selection(&mut self, delta: i32) {
        if self.notifications.is_empty() {
            return;
        }
        self.notification_selected = (self.notification_selected as i32 + delta)
            .clamp(0, self.notifications.len() as i32 - 1)
            as usize;
    }

    pub fn select_notification(&mut self, index: usize) {
        if index < self.notifications.len() {
            self.notification_selected = index;
        }
    }

    fn clamp_notification_selection(&mut self) {
        if self.notifications.is_empty() {
            self.notification_selected = 0;
        } else {
            self.notification_selected =
                self.notification_selected.min(self.notifications.len() - 1);
        }
    }

    pub fn selected_notification(&self) -> Option<Notification> {
        self.notifications.get(self.notification_selected).cloned()
    }

    pub fn open_selected_notification(&mut self) {
        let Some(item) = self.selected_notification() else {
            self.set_status(StatusKind::Info, "先挑一条通知嘛喵~");
            return;
        };
        let url = item.html_url();
        self.open_url(&url);
    }

    pub fn mark_selected_notification_read(&mut self) {
        let Some(item) = self.selected_notification() else {
            self.set_status(StatusKind::Info, "先挑一条通知嘛喵~");
            return;
        };
        if self.notification_marking.is_some() {
            self.set_status(StatusKind::Info, "人家还在改上一条通知喵~等一下下嘛喵~");
            return;
        }
        if !self.client.token_present() {
            self.set_status(StatusKind::Error, "还没登录 GitHub，改不了通知喵~");
            return;
        }
        if !item.unread {
            self.set_status(StatusKind::Info, "这条通知已经读过啦喵~");
            return;
        }
        let id = item.id;
        self.notification_marking = Some(id.clone());
        self.set_status(StatusKind::Info, "人家正在把通知标成已读喵~...");
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.mark_notification_read(&id);
            let _ = tx.send(Msg::NotificationRead { id, result });
        });
    }

    pub fn mark_all_notifications_read(&mut self) {
        if self.notifications_clearing {
            return;
        }
        if !self.client.token_present() {
            self.set_status(StatusKind::Error, "还没登录 GitHub，改不了通知喵~");
            return;
        }
        if self.notifications.is_empty() {
            self.set_status(StatusKind::Info, "这里没有要处理的通知喵~");
            return;
        }
        self.notifications_clearing = true;
        self.set_status(StatusKind::Info, "人家正在把通知全都标成已读喵~...");
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.mark_all_notifications_read();
            let _ = tx.send(Msg::NotificationsRead { result });
        });
    }

    pub fn open_issues(&mut self, kind: IssueKind) {
        let Some(repo) = self.context_repo() else {
            self.set_status(StatusKind::Info, "先挑一个仓库嘛，人家才知道看哪个议题喵~");
            return;
        };
        let name = repo.display_full_name();
        let changed = self
            .issue_repo
            .as_ref()
            .map(|current| current.display_full_name() != name)
            .unwrap_or(true);
        if changed {
            self.issues.clear();
            self.issues_fetched_at = None;
            self.issue_selected = 0;
        }
        self.issue_repo = Some(repo);
        self.issue_kind = kind;
        self.return_page = return_page_when_opening(self.page, self.return_page);
        self.page = Page::Issues;
        self.load_issues_cache();
        self.fetch_issues(false);
    }

    fn load_issues_cache(&mut self) {
        let Some(repo) = self.issue_repo.clone() else {
            return;
        };
        let name = repo.display_full_name();
        let state = self.issue_state;
        match self
            .cache
            .load::<Vec<Issue>>(&issues_cache_key(&name, state))
        {
            Some(cached) => {
                self.issues = cached.data;
                self.issues_fetched_at = Some(cached.fetched_at);
            }
            None => {
                self.issues.clear();
                self.issues_fetched_at = None;
            }
        }
        self.clamp_issue_selection();
    }

    fn fetch_issues(&mut self, force: bool) {
        let Some(repo) = self.issue_repo.clone() else {
            return;
        };
        let name = repo.display_full_name();
        let state = self.issue_state;
        let key = (name.clone(), state);
        if self.issues_loading.as_ref() == Some(&key) {
            return;
        }
        let fresh = self
            .issues_fetched_at
            .map(|at| now_epoch() - at <= self.config.detail_ttl_secs as i64)
            .unwrap_or(false);
        if !force && fresh {
            return;
        }
        self.issues_loading = Some(key);
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.issues(&name, state, repo.private);
            let _ = tx.send(Msg::Issues {
                name,
                state,
                result,
            });
        });
    }

    pub fn issue_indices(&self) -> Vec<usize> {
        self.issues
            .iter()
            .enumerate()
            .filter(|(_, issue)| self.issue_kind.matches(issue))
            .map(|(index, _)| index)
            .collect()
    }

    pub fn selected_issue(&self) -> Option<Issue> {
        self.issue_indices()
            .get(self.issue_selected)
            .and_then(|index| self.issues.get(*index))
            .cloned()
    }

    pub fn issue_repo_label(&self) -> String {
        self.issue_repo
            .as_ref()
            .map(|repo| repo.display_full_name())
            .unwrap_or_else(|| "还没有选仓库".to_string())
    }

    fn clamp_issue_selection(&mut self) {
        let len = self.issue_indices().len();
        if len == 0 {
            self.issue_selected = 0;
        } else {
            self.issue_selected = self.issue_selected.min(len - 1);
        }
    }

    pub fn move_issue_selection(&mut self, delta: i32) {
        let len = self.issue_indices().len();
        if len == 0 {
            return;
        }
        self.issue_selected =
            (self.issue_selected as i32 + delta).clamp(0, len as i32 - 1) as usize;
    }

    pub fn select_issue(&mut self, index: usize) {
        let len = self.issue_indices().len();
        if len == 0 {
            return;
        }
        self.issue_selected = index.min(len - 1);
    }

    pub fn open_selected_issue(&mut self) {
        let Some(issue) = self.selected_issue() else {
            self.set_status(StatusKind::Info, "先挑一个议题嘛喵~");
            return;
        };
        self.open_url(&issue.html_url);
    }

    pub fn set_issue_kind(&mut self, kind: IssueKind) {
        if self.issue_kind == kind {
            return;
        }
        self.issue_kind = kind;
        self.issue_selected = 0;
        self.clamp_issue_selection();
    }

    pub fn cycle_issue_kind(&mut self) {
        self.set_issue_kind(self.issue_kind.next());
        self.set_status(
            StatusKind::Info,
            &format!("现在只看{}", self.issue_kind.label()),
        );
    }

    pub fn set_issue_state(&mut self, state: IssueState) {
        if self.issue_state == state {
            return;
        }
        self.issue_state = state;
        self.issue_selected = 0;
        self.issues.clear();
        self.issues_fetched_at = None;
        self.load_issues_cache();
        self.fetch_issues(false);
    }

    pub fn cycle_issue_state(&mut self) {
        self.set_issue_state(self.issue_state.next());
        self.set_status(
            StatusKind::Info,
            &format!("状态筛成{}", self.issue_state.label()),
        );
    }

    pub fn open_account(&mut self) {
        self.navigate(Page::Account);
        self.fetch_rate_limit(false);
    }

    pub fn is_own_repo(&self, repo: &Repo) -> bool {
        self.login
            .as_deref()
            .map(|login| login.eq_ignore_ascii_case(repo.owner_name()))
            .unwrap_or(false)
    }

    pub fn open_repo_edit(&mut self) {
        let Some(repo) = self.effective_repo() else {
            self.set_status(StatusKind::Info, "先挑一个仓库嘛，人家才知道要改哪个喵~");
            return;
        };
        if !self.is_own_repo(&repo) {
            self.set_status(StatusKind::Info, "这个仓库不是你的哦，人家只能帮你看看喵~");
            return;
        }
        if !self.client.token_present() {
            self.set_status(StatusKind::Error, "还没登录呢，改不了仓库喵~");
            return;
        }
        self.repo_edit = Some(RepoEdit::from_repo(&repo));
        self.overlay = Some(Overlay::EditRepo);
    }

    pub fn move_repo_edit_row(&mut self, delta: i32) {
        let Some(edit) = self.repo_edit.as_mut() else {
            return;
        };
        let next = (edit.row as i32 + delta).clamp(0, 4) as usize;
        edit.row = next;
    }

    pub fn toggle_repo_visibility(&mut self) {
        if let Some(edit) = self.repo_edit.as_mut() {
            edit.private = !edit.private;
        }
    }

    pub fn toggle_repo_archived(&mut self) {
        if let Some(edit) = self.repo_edit.as_mut() {
            edit.archived = !edit.archived;
        }
    }

    pub fn save_repo_edit(&mut self) {
        let Some(edit) = self.repo_edit.as_mut() else {
            return;
        };
        if edit.saving {
            return;
        }
        edit.editing = false;
        edit.error = None;
        edit.saving = true;
        let full_name = edit.full_name.clone();
        let description = edit.description.value.trim().to_string();
        let homepage = edit.homepage.value.trim().to_string();
        let topics = edit.topics_list();
        let private = edit.private;
        let archived = edit.archived;
        let patch = serde_json::json!({
            "description": if description.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(description) },
            "homepage": if homepage.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(homepage) },
            "topics": topics,
            "private": private,
            "archived": archived,
        });
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.update_repository(&full_name, &patch);
            let _ = tx.send(Msg::RepoUpdated(result));
        });
    }

    fn handle_repo_edit_key(&mut self, key: KeyEvent) {
        let Some(edit) = self.repo_edit.as_mut() else {
            self.overlay = None;
            return;
        };
        if edit.saving {
            return;
        }
        if edit.editing {
            match key.code {
                KeyCode::Enter | KeyCode::Char('\r') | KeyCode::Char('\n') => {
                    edit.editing = false;
                    if edit.row < 2 {
                        edit.row += 1;
                    }
                }
                KeyCode::Esc => edit.editing = false,
                KeyCode::Backspace => match edit.row {
                    0 => edit.description.backspace(),
                    1 => edit.homepage.backspace(),
                    2 => edit.topics.backspace(),
                    _ => {}
                },
                KeyCode::Delete => match edit.row {
                    0 => edit.description.delete(),
                    1 => edit.homepage.delete(),
                    2 => edit.topics.delete(),
                    _ => {}
                },
                KeyCode::Left => match edit.row {
                    0 => edit.description.left(),
                    1 => edit.homepage.left(),
                    2 => edit.topics.left(),
                    _ => {}
                },
                KeyCode::Right => match edit.row {
                    0 => edit.description.right(),
                    1 => edit.homepage.right(),
                    2 => edit.topics.right(),
                    _ => {}
                },
                KeyCode::Home => match edit.row {
                    0 => edit.description.home(),
                    1 => edit.homepage.home(),
                    2 => edit.topics.home(),
                    _ => {}
                },
                KeyCode::End => match edit.row {
                    0 => edit.description.end(),
                    1 => edit.homepage.end(),
                    2 => edit.topics.end(),
                    _ => {}
                },
                KeyCode::Char(ch) => match edit.row {
                    0 => edit.description.insert(ch),
                    1 => edit.homepage.insert(ch),
                    2 => edit.topics.insert(ch),
                    _ => {}
                },
                _ => {}
            }
            return;
        }
        let row = edit.row;
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.overlay = None;
                self.repo_edit = None;
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_repo_edit_row(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_repo_edit_row(1),
            KeyCode::Char('g') | KeyCode::Home => {
                if let Some(edit) = self.repo_edit.as_mut() {
                    edit.row = 0;
                }
            }
            KeyCode::Char('G') | KeyCode::End => {
                if let Some(edit) = self.repo_edit.as_mut() {
                    edit.row = 4;
                }
            }
            KeyCode::Enter | KeyCode::Char('\r') | KeyCode::Char('\n') => match row {
                0..=2 => {
                    if let Some(edit) = self.repo_edit.as_mut() {
                        edit.editing = true;
                    }
                }
                3 => self.toggle_repo_visibility(),
                4 => self.toggle_repo_archived(),
                _ => {}
            },
            KeyCode::Left | KeyCode::Right => {
                if row == 3 {
                    self.toggle_repo_visibility();
                } else if row == 4 {
                    self.toggle_repo_archived();
                }
            }
            KeyCode::Char('s') => self.save_repo_edit(),
            _ => {}
        }
    }

    fn handle_cache_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.overlay = None,
            KeyCode::Up | KeyCode::Char('k') => self.move_cache_row(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_cache_row(1),
            KeyCode::Home | KeyCode::Char('g') => self.cache_row = 0,
            KeyCode::End | KeyCode::Char('G') => {
                self.cache_row = cache_groups().len().saturating_sub(1);
            }
            KeyCode::Enter => self.clear_cache_group(self.cache_row),
            KeyCode::Char('o') => self.open_cache_dir(),
            _ => {}
        }
    }

    pub fn tick(&mut self) {
        if let Some(pending) = self.pending_detail.take() {
            if pending.at.elapsed() >= Duration::from_millis(220) {
                self.fetch_detail_if_needed(pending.repo, false);
            } else {
                self.pending_detail = Some(pending);
            }
        }
    }

    pub fn busy_text(&self) -> Option<&'static str> {
        if self.user_loading {
            return Some("人家正在确认身份喵~...");
        }
        if self.page == Page::Account && self.rate_loading {
            return Some("人家正在数 API 额度喵~...");
        }
        if self.page == Page::Notifications && self.notifications_loading.is_some() {
            return Some("人家正在收通知喵~...");
        }
        if self.notifications_clearing {
            return Some("人家正在把通知全都标成已读喵~...");
        }
        if self.notification_marking.is_some() {
            return Some("人家正在把通知标成已读喵~...");
        }
        if self.page == Page::Issues && self.issues_loading.is_some() {
            return Some("人家正在翻议题和合并请求喵~...");
        }
        if self.page == Page::Actions && self.action_runs_loading {
            return Some("人家正在翻工作流记录喵~...");
        }
        if self.page == Page::Actions && self.action_jobs_loading.is_some() {
            return Some("人家正在读任务详情喵~...");
        }
        if self.workflow_running {
            return Some("人家正在把工作流交给 GitHub 喵~...");
        }
        if self.action_workflows_loading && self.overlay == Some(Overlay::RunWorkflow) {
            return Some("人家正在取工作流列表喵~...");
        }
        if self.repo_edit.as_ref().is_some_and(|edit| edit.saving) {
            return Some("人家正在把改动递给 GitHub 喵~...");
        }
        if self.repos_loading && self.page == Page::Repositories {
            return Some("人家正在一页一页搬仓库列表喵~...");
        }
        if self.starred_loading && self.page == Page::Starred {
            return Some("人家正在把全部收藏都整理出来喵~...");
        }
        if self.search_loading && self.page == Page::Search {
            return Some("人家正在翻找仓库喵~...");
        }
        if self.star_loading.is_some() {
            return Some("人家正在改星星喵~...");
        }
        if self.page == Page::Repo && self.tree_loading.is_some() {
            return Some("人家正在翻仓库的文件树喵~...");
        }
        if self.page == Page::Repo && self.readme_loading.is_some() {
            return Some("人家正在读 README 喵~...");
        }
        if matches!(self.page, Page::Repo | Page::ReleaseDetail) && self.releases_loading.is_some()
        {
            return Some("人家正在取发行版喵~...");
        }
        if self.detail_loading.is_some()
            && matches!(
                self.page,
                Page::Repositories | Page::Starred | Page::Search | Page::Repo
            )
        {
            return Some("人家正在读仓库详情喵~...");
        }
        None
    }

    pub fn download_summary(&self) -> Option<String> {
        let job = self.download_job.as_ref()?;
        let progress = job
            .total
            .filter(|total| *total > 0)
            .map(|total| (job.received.saturating_mul(100) / total).min(100));
        let size = match job.total {
            Some(total) => format!(
                "{} / {}",
                format::format_size(job.received),
                format::format_size(total)
            ),
            None => format::format_size(job.received),
        };
        Some(format!(
            "{} 下载 {} · {} · {}喵~",
            icons::DOWNLOAD,
            format::truncate(&job.label, 36),
            size,
            progress
                .map(|value| format!("{value}%"))
                .unwrap_or_else(|| "进行中喵~".to_string())
        ))
    }

    pub fn download_path_label(&self) -> String {
        format::display_path(&self.config.expanded_download_dir())
    }

    pub fn spinner(&self) -> &'static str {
        format::spinner(self.started_at.elapsed().as_millis() as u64 / 120)
    }

    pub fn set_status(&mut self, kind: StatusKind, text: &str) {
        self.status = Status {
            text: text.to_string(),
            kind,
        };
    }

    pub fn refresh_cache_stats(&mut self) {
        self.cache_stats = self.cache.stats();
    }

    pub fn current_list_page(&self) -> Page {
        if self.page.is_repo_page() {
            self.return_page
        } else {
            self.page
        }
    }

    fn raw_list(&self, page: Page) -> &[Repo] {
        match page {
            Page::Starred => &self.starred,
            Page::Search => &self.search_results,
            _ => &self.repos,
        }
    }

    pub fn visible_indices(&self) -> Vec<usize> {
        self.visible_indices_for(self.current_list_page())
    }

    fn filter_for(&self, page: Page) -> &TextInput {
        if page == Page::Starred {
            &self.star_filter
        } else {
            &self.repo_filter
        }
    }

    fn filter_for_mut(&mut self, page: Page) -> &mut TextInput {
        if page == Page::Starred {
            &mut self.star_filter
        } else {
            &mut self.repo_filter
        }
    }

    pub fn current_filter(&self) -> &TextInput {
        self.filter_for(self.page)
    }

    pub fn list_filter_active(&self) -> bool {
        matches!(self.page, Page::Repositories | Page::Starred)
            && !self.current_filter().value.trim().is_empty()
    }

    pub fn focus_list_filter(&mut self) {
        if !matches!(self.page, Page::Repositories | Page::Starred) {
            return;
        }
        self.search_focus = false;
        self.list_filter_focus = true;
    }

    pub fn focus_release_filter(&mut self) {
        if self.page != Page::Repo || self.repo_pane != RepoPane::Releases {
            return;
        }
        self.search_focus = false;
        self.list_filter_focus = false;
        self.release_filter_focus = true;
    }

    fn release_matches_filter(release: &Release, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let haystack = format!("{} {}", release.tag_name, release.title()).to_lowercase();
        haystack.contains(query)
    }

    pub fn visible_release_indices(&self) -> Vec<usize> {
        let query = self.release_filter.value.trim().to_lowercase();
        self.releases
            .iter()
            .enumerate()
            .filter(|(_, release)| Self::release_matches_filter(release, &query))
            .map(|(index, _)| index)
            .collect()
    }

    fn repo_matches_filter(repo: &Repo, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let haystack = format!(
            "{} {} {} {}",
            repo.display_full_name(),
            repo.description_text(),
            repo.language_label(),
            repo.topics.join(" ")
        )
        .to_lowercase();
        haystack.contains(query)
    }

    fn visible_indices_for(&self, page: Page) -> Vec<usize> {
        let raw = self.raw_list(page);
        let language = if page == Page::Repositories {
            self.language_filter.as_deref()
        } else {
            None
        };
        let query = if matches!(page, Page::Repositories | Page::Starred) {
            self.filter_for(page).value.trim().to_lowercase()
        } else {
            String::new()
        };
        let mut indices: Vec<usize> = raw
            .iter()
            .enumerate()
            .filter(|(_, repo)| {
                language
                    .map(|language| repo.language_label().eq_ignore_ascii_case(language))
                    .unwrap_or(true)
                    && Self::repo_matches_filter(repo, &query)
            })
            .map(|(index, _)| index)
            .collect();
        match page {
            Page::Repositories => sort_indices(raw, &mut indices, self.repo_sort),
            Page::Starred => sort_star_indices(raw, &mut indices, self.star_sort),
            _ => {}
        }
        indices
    }

    pub fn selected_repo(&self) -> Option<Repo> {
        let page = self.current_list_page();
        let indices = self.visible_indices_for(page);
        let selected = self.selection_for(page);
        indices
            .get(selected)
            .and_then(|index| self.raw_list(page).get(*index))
            .cloned()
    }

    pub fn raw_repo(&self, raw_index: usize) -> Repo {
        self.raw_list(self.current_list_page())
            .get(raw_index)
            .cloned()
            .unwrap_or_default()
    }

    fn selection_for(&self, page: Page) -> usize {
        match page {
            Page::Starred => self.star_selected,
            Page::Search => self.search_selected,
            _ => self.repo_selected,
        }
    }

    fn set_selection_for(&mut self, page: Page, value: usize) {
        match page {
            Page::Starred => self.star_selected = value,
            Page::Search => self.search_selected = value,
            _ => self.repo_selected = value,
        }
    }

    fn set_selected_visible(&mut self, index: usize) {
        let page = self.current_list_page();
        let len = self.visible_indices_for(page).len();
        let index = if len == 0 { 0 } else { index.min(len - 1) };
        self.set_selection_for(page, index);
        self.follow_selection();
        self.schedule_selected_detail();
    }

    pub fn select_visible(&mut self, index: usize) {
        self.set_selected_visible(index);
    }

    pub fn move_selection(&mut self, delta: i32) {
        let page = self.current_list_page();
        let len = self.visible_indices_for(page).len();
        if len == 0 {
            return;
        }
        let current = self.selection_for(page) as i32;
        let next = (current + delta).clamp(0, len as i32 - 1) as usize;
        self.set_selected_visible(next);
    }

    pub fn follow_selection(&mut self) {
        let len = self.visible_indices().len();
        if len == 0 {
            self.list_offset = 0;
            return;
        }
        let selected = self.selection_for(self.current_list_page());
        let page_size = self.page_size();
        self.list_offset = page_start(selected, page_size);
        let max_offset = ((len - 1) / page_size) * page_size;
        self.list_offset = self.list_offset.min(max_offset);
    }

    pub fn page_size(&self) -> usize {
        self.viewport_cards.max(1)
    }

    pub fn page_count_for(&self, page: Page) -> usize {
        let len = self.visible_indices_for(page).len();
        page_count(len, self.page_size())
    }

    pub fn current_page_for(&self, page: Page) -> usize {
        page_index(self.selection_for(page), self.page_size()).min(self.page_count_for(page) - 1)
    }

    pub fn page_label(&self) -> String {
        let page = self.current_list_page();
        let len = self.visible_indices_for(page).len();
        format!(
            "第 {}/{} 页 · {} 个喵~",
            self.current_page_for(page) + 1,
            self.page_count_for(page),
            len
        )
    }

    fn is_list_page(page: Page) -> bool {
        matches!(page, Page::Repositories | Page::Starred | Page::Search)
    }

    pub fn next_list_page(&mut self) {
        let page = self.current_list_page();
        if !Self::is_list_page(page) {
            return;
        }
        let current = self.current_page_for(page);
        if current + 1 >= self.page_count_for(page) {
            self.set_status(StatusKind::Info, "已经是最后一页啦喵~");
            return;
        }
        self.set_list_page(current + 1);
    }

    pub fn prev_list_page(&mut self) {
        let page = self.current_list_page();
        if !Self::is_list_page(page) {
            return;
        }
        let current = self.current_page_for(page);
        if current == 0 {
            self.set_status(StatusKind::Info, "已经是第一页啦喵~");
            return;
        }
        self.set_list_page(current - 1);
    }

    pub fn first_list_page(&mut self) {
        if Self::is_list_page(self.current_list_page()) {
            self.set_list_page(0);
        }
    }

    pub fn last_list_page(&mut self) {
        let page = self.current_list_page();
        if !Self::is_list_page(page) {
            return;
        }
        let last = self.page_count_for(page).saturating_sub(1);
        self.set_list_page(last);
    }

    fn set_list_page(&mut self, target: usize) {
        let page = self.current_list_page();
        let len = self.visible_indices_for(page).len();
        if len == 0 {
            return;
        }
        let start = target.min(self.page_count_for(page) - 1) * self.page_size();
        self.set_selection_for(page, start.min(len - 1));
        self.follow_selection();
        self.schedule_selected_detail();
    }

    pub fn selected_index(&self) -> usize {
        self.selection_for(self.current_list_page())
    }

    fn clamp_selection(&mut self) {
        for page in [Page::Repositories, Page::Starred, Page::Search] {
            let len = self.visible_indices_for(page).len();
            let selected = self.selection_for(page);
            if len == 0 {
                self.set_selection_for(page, 0);
            } else if selected >= len {
                self.set_selection_for(page, len - 1);
            }
        }
        self.follow_selection();
    }

    fn restore_selection(&mut self, previous: Option<String>) {
        let Some(previous) = previous else {
            return;
        };
        let page = self.current_list_page();
        let indices = self.visible_indices_for(page);
        if let Some(position) = indices.iter().position(|index| {
            self.raw_list(page)
                .get(*index)
                .map(|repo| repo.display_full_name() == previous)
                .unwrap_or(false)
        }) {
            self.set_selection_for(page, position);
        }
    }

    pub fn language_counts(&self) -> Vec<(String, usize)> {
        let mut counts: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for repo in &self.repos {
            let language = repo.language_label().to_string();
            *counts.entry(language).or_insert(0) += 1;
        }
        let mut counts: Vec<_> = counts.into_iter().collect();
        counts.sort_by(|left, right| {
            right.1.cmp(&left.1).then_with(|| {
                left.0
                    .to_ascii_lowercase()
                    .cmp(&right.0.to_ascii_lowercase())
            })
        });
        counts
    }

    pub fn language_choices(&self) -> Vec<(Option<String>, String, usize)> {
        let mut choices = vec![(None, "全部语言喵~".to_string(), self.repos.len())];
        for (language, count) in self.language_counts() {
            choices.push((Some(language.clone()), language, count));
        }
        choices
    }

    pub fn open_languages(&mut self) {
        self.overlay = Some(Overlay::Languages);
        let choices = self.language_choices();
        self.language_selected = choices
            .iter()
            .position(|(value, _, _)| value.as_deref() == self.language_filter.as_deref())
            .unwrap_or(0);
        let viewport = self.language_viewport.max(1);
        if self.language_selected >= viewport {
            self.language_offset = self.language_selected + 1 - viewport;
        } else {
            self.language_offset = 0;
        }
    }

    pub fn move_language_selection(&mut self, delta: i32) {
        let len = self.language_choices().len();
        if len == 0 {
            return;
        }
        let next = (self.language_selected as i32 + delta).clamp(0, len as i32 - 1) as usize;
        self.language_selected = next;
        let viewport = self.language_viewport.max(1);
        if next < self.language_offset {
            self.language_offset = next;
        } else if next >= self.language_offset + viewport {
            self.language_offset = next + 1 - viewport;
        }
    }

    pub fn choose_language_selection(&mut self) {
        let choices = self.language_choices();
        let Some((value, _, _)) = choices.get(self.language_selected) else {
            return;
        };
        self.set_language(value.clone());
    }

    pub fn repo_stats(&self) -> (usize, usize, usize, u64) {
        let public = self.repos.iter().filter(|repo| !repo.private).count();
        let private = self.repos.len().saturating_sub(public);
        let languages = self
            .repos
            .iter()
            .filter(|repo| repo.language.is_some())
            .map(|repo| repo.language_label().to_string())
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        let stars = self.repos.iter().map(|repo| repo.stargazers_count).sum();
        (public, private, languages, stars)
    }

    pub fn starred_stats(&self) -> (usize, usize, u64) {
        let languages = self
            .starred
            .iter()
            .filter(|repo| repo.language.is_some())
            .map(|repo| repo.language_label().to_string())
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        let stars = self.starred.iter().map(|repo| repo.stargazers_count).sum();
        (self.starred.len(), languages, stars)
    }

    pub fn search_depth_label(&self) -> String {
        let Some(query) = self.search_submitted.as_deref() else {
            return "人家还没开始找呢喵~".to_string();
        };
        let total = self
            .search_total
            .map(format::human_count)
            .unwrap_or_else(|| "?".to_string());
        format!("「{query}」一共找到 {total} 个呢喵~")
    }

    pub fn auth_label(&self) -> String {
        if !self.client.token_present() {
            "未登录喵~".to_string()
        } else {
            self.login
                .clone()
                .or_else(|| Some("读取中喵~".to_string()))
                .unwrap_or_else(|| "已登录喵~".to_string())
        }
    }

    fn repos_stale(&self) -> bool {
        if self.repos.len() == 100 {
            return true;
        }
        self.fetched_repos_at
            .map(|at| now_epoch() - at > self.config.repos_ttl_secs as i64)
            .unwrap_or(true)
    }

    fn starred_stale(&self) -> bool {
        if self.starred.len() == 100 {
            return true;
        }
        self.fetched_starred_at
            .map(|at| now_epoch() - at > self.config.repos_ttl_secs as i64)
            .unwrap_or(true)
    }

    fn fetch_user(&mut self) {
        if self.user_loading || !self.client.token_present() {
            return;
        }
        self.user_loading = true;
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Msg::User(client.user()));
        });
    }

    fn fetch_repos_if_needed(&mut self, force: bool) {
        if self.repos_loading {
            return;
        }
        let Some(login) = self.login.clone() else {
            self.fetch_user();
            return;
        };
        if !force && !self.repos_stale() {
            return;
        }
        if !self.client.token_present() {
            self.set_status(StatusKind::Error, "还没登录呢，人家读不到仓库列表哦喵~");
            return;
        }
        self.repos_loading = true;
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Repos {
                login,
                result: client.repositories(),
            });
        });
    }

    fn fetch_starred_if_needed(&mut self, force: bool) {
        if self.starred_loading {
            return;
        }
        let Some(login) = self.login.clone() else {
            self.fetch_user();
            return;
        };
        if !force && !self.starred_stale() {
            return;
        }
        if !self.client.token_present() {
            self.set_status(StatusKind::Error, "还没登录呢，人家读不到收藏哦喵~");
            return;
        }
        self.starred_loading = true;
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Starred {
                login,
                result: client.starred(),
            });
        });
    }

    fn schedule_selected_detail(&mut self) {
        let Some(repo) = self.selected_repo() else {
            if self.page != Page::Repo {
                self.detail = None;
                self.detail_for = None;
                self.detail_fetched_at = None;
                self.releases.clear();
                self.releases_fetched_at = None;
            }
            return;
        };
        let name = repo.display_full_name();
        if self.detail_for.as_deref() != Some(name.as_str()) {
            self.load_detail_cache(&repo);
            self.detail_scroll = 0;
            self.detail_scroll_max = 0;
        }
        self.pending_detail = Some(PendingDetail {
            repo,
            at: Instant::now(),
        });
    }

    fn load_detail_cache(&mut self, repo: &Repo) {
        let name = repo.display_full_name();
        self.detail_for = Some(name.clone());
        self.detail = None;
        self.detail_fetched_at = None;
        self.releases.clear();
        self.releases_fetched_at = None;
        if let Some(cached) = self.cache.load::<Repo>(&repo.cache_key()) {
            self.detail = Some(cached.data);
            self.detail_fetched_at = Some(cached.fetched_at);
        }
        if let Some(cached) = self.cache.load::<Vec<Release>>(&repo.releases_cache_key()) {
            self.releases = cached.data;
            self.releases_fetched_at = Some(cached.fetched_at);
        }
    }

    fn fetch_detail_if_needed(&mut self, repo: Repo, force: bool) {
        let name = repo.display_full_name();
        if self.detail_for.as_deref() != Some(name.as_str()) {
            self.load_detail_cache(&repo);
        }
        let detail_fresh = self
            .detail_fetched_at
            .map(|at| now_epoch() - at <= self.config.detail_ttl_secs as i64)
            .unwrap_or(false);
        let releases_fresh = self
            .releases_fetched_at
            .map(|at| now_epoch() - at <= self.config.detail_ttl_secs as i64)
            .unwrap_or(false);
        if force || !detail_fresh {
            self.fetch_detail(&repo);
        }
        if force || !releases_fresh {
            self.fetch_releases(&repo);
        }
    }

    fn fetch_detail(&mut self, repo: &Repo) {
        let name = repo.display_full_name();
        if self.detail_loading.as_deref() == Some(name.as_str()) {
            return;
        }
        self.detail_loading = Some(name.clone());
        let tx = self.tx.clone();
        let client = self.client.clone();
        let private = repo.private;
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Detail {
                name: name.clone(),
                result: client.repository(&name, private),
            });
        });
    }

    fn fetch_releases(&mut self, repo: &Repo) {
        let name = repo.display_full_name();
        if self.releases_loading.as_deref() == Some(name.as_str()) {
            return;
        }
        self.releases_loading = Some(name.clone());
        let tx = self.tx.clone();
        let client = self.client.clone();
        let private = repo.private;
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Releases {
                name: name.clone(),
                result: client.releases(&name, private),
            });
        });
    }

    fn schedule_repo_extras(&mut self, repo: &Repo, force: bool) {
        let name = repo.display_full_name();
        let branch = repo.default_branch.clone();
        if self.config.show_file_tree {
            if self.tree_for.as_deref() != Some(name.as_str()) || self.tree_branch != branch {
                self.load_tree_cache(repo);
            }
            let fresh = self
                .tree_fetched_at
                .map(|at| now_epoch() - at <= self.config.detail_ttl_secs as i64)
                .unwrap_or(false);
            if force || !fresh {
                self.fetch_tree(repo);
            }
        }
        if self.config.show_readme {
            if self.readme_for.as_deref() != Some(name.as_str()) {
                self.load_readme_cache(repo);
            }
            let fresh = self
                .readme_fetched_at
                .map(|at| now_epoch() - at <= self.config.detail_ttl_secs as i64)
                .unwrap_or(false);
            if force || !fresh {
                self.fetch_readme(repo);
            }
        }
        if !self.config.show_file_tree && self.repo_pane == RepoPane::Tree {
            self.repo_pane = RepoPane::Releases;
        }
        if !self.config.show_readme && self.repo_pane == RepoPane::Readme {
            self.repo_pane = RepoPane::Releases;
        }
    }

    fn load_tree_cache(&mut self, repo: &Repo) {
        let name = repo.display_full_name();
        let branch = repo.default_branch.clone();
        self.tree = None;
        self.tree_fetched_at = None;
        self.tree_for = Some(name.clone());
        self.tree_branch = branch.clone();
        self.tree_scroll = 0;
        self.tree_scroll_max = 0;
        if let Some(cached) = self
            .cache
            .load::<TreeResponse>(&tree_cache_key(&name, &branch))
        {
            self.tree = Some(cached.data);
            self.tree_fetched_at = Some(cached.fetched_at);
        }
    }

    fn load_readme_cache(&mut self, repo: &Repo) {
        let name = repo.display_full_name();
        self.readme = None;
        self.readme_fetched_at = None;
        self.readme_for = Some(name.clone());
        self.readme_scroll = 0;
        self.readme_scroll_max = 0;
        if let Some(cached) = self.cache.load::<ReadmeContent>(&readme_cache_key(&name)) {
            self.readme = Some(cached.data);
            self.readme_fetched_at = Some(cached.fetched_at);
        }
    }

    fn fetch_tree(&mut self, repo: &Repo) {
        let name = repo.display_full_name();
        if self.tree_loading.as_deref() == Some(name.as_str()) {
            return;
        }
        self.tree_loading = Some(name.clone());
        let tx = self.tx.clone();
        let client = self.client.clone();
        let branch = repo.default_branch.clone();
        let private = repo.private;
        std::thread::spawn(move || {
            let result = client.file_tree(&name, &branch, private);
            let _ = tx.send(Msg::Tree {
                name,
                branch,
                result,
            });
        });
    }

    fn fetch_readme(&mut self, repo: &Repo) {
        let name = repo.display_full_name();
        if self.readme_loading.as_deref() == Some(name.as_str()) {
            return;
        }
        self.readme_loading = Some(name.clone());
        let tx = self.tx.clone();
        let client = self.client.clone();
        let private = repo.private;
        std::thread::spawn(move || {
            let result = client.readme(&name, private);
            let _ = tx.send(Msg::Readme { name, result });
        });
    }

    pub fn refresh_current(&mut self) {
        match self.page {
            Page::Repositories => self.fetch_repos_if_needed(true),
            Page::Starred => self.fetch_starred_if_needed(true),
            Page::Search => {
                if self.search_submitted.is_some() {
                    self.perform_search(true);
                }
            }
            Page::Repo => {
                if let Some(repo) = self.page_repo.clone().or_else(|| self.selected_repo()) {
                    self.fetch_detail_if_needed(repo.clone(), true);
                    self.schedule_repo_extras(&repo, true);
                }
            }
            Page::Account => {
                self.fetch_user();
                self.fetch_rate_limit(true);
            }
            Page::Actions => {
                self.fetch_action_runs(true);
                self.schedule_action_jobs(true);
            }
            Page::Issues => self.fetch_issues(true),
            Page::ReleaseDetail => {
                if let Some(repo) = self.page_repo.clone() {
                    self.schedule_repo_extras(&repo, true);
                }
            }
            Page::Notifications => self.fetch_notifications(true),
            Page::Settings => self.refresh_cache_stats(),
        }
    }

    pub fn navigate(&mut self, page: Page) {
        if self.settings_only && page != Page::Settings {
            return;
        }
        if page == Page::Repo || page == self.page {
            if page == Page::Search {
                self.search_focus = true;
            }
            return;
        }
        if self.editing_download_dir {
            self.commit_download_dir_edit();
        }
        if self.editing_proxy {
            self.commit_proxy_edit();
        }
        if self.page == Page::Settings && self.config_dirty {
            self.save_config(false);
        }
        self.page = page;
        self.return_page = page;
        self.page_repo = None;
        self.issue_repo = None;
        self.search_focus = page == Page::Search;
        self.list_filter_focus = false;
        self.release_filter_focus = false;
        self.list_offset = 0;
        self.pending_detail = None;
        if page == Page::Settings {
            self.refresh_cache_stats();
            self.editing_proxy = false;
            self.editing_download_dir = false;
        }
        if matches!(page, Page::Repositories | Page::Starred) {
            self.schedule_selected_detail();
            self.follow_selection();
        }
        if page == Page::Search {
            self.schedule_selected_detail();
        }
        if page == Page::Account {
            self.fetch_rate_limit(false);
        }
        if page == Page::Notifications {
            self.load_notifications_cache();
            self.fetch_notifications(false);
        }
    }

    pub fn open_repo_page(&mut self) {
        let Some(repo) = self
            .page_repo
            .clone()
            .filter(|_| self.page == Page::Repo)
            .or_else(|| self.selected_repo())
        else {
            self.set_status(StatusKind::Info, "先挑一个仓库嘛，人家才知道你想看哪个喵~");
            return;
        };
        self.return_page = return_page_when_opening(self.page, self.return_page);
        self.issue_repo = None;
        self.release_filter = TextInput::new("");
        self.release_filter_focus = false;
        self.page_repo = Some(repo.clone());
        self.page = Page::Repo;
        self.page_scroll = 0;
        self.search_focus = false;
        self.detail_scroll_reset = true;
        self.detail_for = None;
        self.detail = None;
        self.releases.clear();
        self.release_selected = 0;
        self.release_asset_selected = 0;
        self.release_scroll = 0;
        self.release_scroll_max = 0;
        self.detail_fetched_at = None;
        self.releases_fetched_at = None;
        self.load_detail_cache(&repo);
        self.repo_pane = RepoPane::Releases;
        self.tree = None;
        self.readme = None;
        self.tree_for = None;
        self.readme_for = None;
        self.tree_fetched_at = None;
        self.readme_fetched_at = None;
        self.schedule_repo_extras(&repo, false);
        self.pending_detail = Some(PendingDetail {
            repo,
            at: Instant::now(),
        });
    }

    pub fn go_back(&mut self) {
        let Some(target) = back_target(self.page, self.return_page, self.page_repo.is_some())
        else {
            return;
        };
        self.search_focus = false;
        if target == Page::Repo {
            self.page = Page::Repo;
            return;
        }
        self.page = target;
        self.page_repo = None;
        self.issue_repo = None;
        self.schedule_selected_detail();
    }

    pub fn perform_search(&mut self, force: bool) {
        let query = self.search_input.value.trim().to_string();
        if query.is_empty() {
            self.set_status(StatusKind::Info, "搜索框空空的，先告诉人家想找什么嘛喵~");
            self.search_focus = true;
            return;
        }
        self.search_focus = false;
        self.search_submitted = Some(query.clone());
        self.config.remember_search(&query);
        let _ = self.config.save(&self.paths);
        self.search_selected = 0;
        self.list_offset = 0;
        self.search_loading = true;
        let key = search_key(&query, self.search_sort.key());
        if let Some(cached) = self.cache.load::<SearchResponse>(&key) {
            let fresh = cached.is_fresh(self.config.search_ttl_secs);
            self.search_results = cached.data.items;
            self.search_total = Some(cached.data.total_count);
            if fresh && !force {
                self.search_loading = false;
                self.schedule_selected_detail();
                self.refresh_cache_stats();
                self.set_status(StatusKind::Info, "这组结果刚从缓存里拿出来啦喵~");
                return;
            }
        }
        if force {
            self.search_results.clear();
            self.search_total = None;
        }
        let tx = self.tx.clone();
        let client = self.client.clone();
        let sort = self.search_sort;
        let sent_query = query.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Search {
                query: sent_query.clone(),
                sort,
                result: client.search(&sent_query, sort),
            });
        });
        self.schedule_selected_detail();
    }

    pub fn submit_search(&mut self) {
        self.perform_search(false);
    }

    pub fn focus_search(&mut self) {
        self.list_filter_focus = false;
        self.release_filter_focus = false;
        if self.page != Page::Search {
            self.navigate(Page::Search);
        }
        self.search_focus = true;
    }

    fn blur_list_filter(&mut self) {
        self.list_filter_focus = false;
    }

    pub fn select_search_history(&mut self, query: String) {
        self.search_input = TextInput::new(query);
        self.perform_search(false);
    }

    pub fn set_repo_sort(&mut self, sort: RepoSort) {
        self.blur_list_filter();
        self.repo_sort = sort;
        self.repo_selected = 0;
        self.list_offset = 0;
        self.schedule_selected_detail();
    }

    pub fn set_star_sort(&mut self, sort: StarSort) {
        self.blur_list_filter();
        self.star_sort = sort;
        self.star_selected = 0;
        self.list_offset = 0;
        self.schedule_selected_detail();
    }

    pub fn set_search_sort(&mut self, sort: SearchSort) {
        if self.search_sort == sort {
            return;
        }
        self.search_sort = sort;
        if self.search_submitted.is_some() {
            self.perform_search(false);
        }
    }

    pub fn set_language(&mut self, language: Option<String>) {
        self.language_filter = language;
        self.repo_selected = 0;
        self.list_offset = 0;
        self.overlay = None;
        self.schedule_selected_detail();
    }

    pub fn open_profile(&mut self) {
        let Some(user) = self.user.as_ref() else {
            self.set_status(StatusKind::Info, "还没有账号信息呢，先登录一下嘛喵~");
            return;
        };
        let url = user.html_url.clone();
        self.open_url(&url);
    }

    pub fn open_current(&mut self) {
        let Some(repo) = self.effective_repo() else {
            self.set_status(StatusKind::Info, "还没有选中仓库呢喵~");
            return;
        };
        self.open_url(&repo.html_url);
    }

    pub fn copy_clone(&mut self) {
        let Some(repo) = self.effective_repo() else {
            self.set_status(StatusKind::Info, "还没有选中仓库呢喵~");
            return;
        };
        let url = if !repo.ssh_url.is_empty() {
            repo.ssh_url.clone()
        } else if !repo.clone_url.is_empty() {
            repo.clone_url.clone()
        } else {
            format!("https://github.com/{}.git", repo.display_full_name())
        };
        self.copy_text(&url, "克隆地址喵~");
    }

    fn effective_repo(&self) -> Option<Repo> {
        self.context_repo()
    }

    fn open_url(&mut self, url: &str) {
        if url.trim().is_empty() {
            self.set_status(StatusKind::Error, "这个地址是空的，打不开呢喵~");
            return;
        }
        let result = open_external(url);
        match result {
            Ok(()) => self.set_status(StatusKind::Ok, "已经交给浏览器打开啦喵~"),
            Err(error) => self.set_status(StatusKind::Error, &error),
        }
    }

    fn copy_text(&mut self, text: &str, label: &str) {
        if text.trim().is_empty() {
            self.set_status(StatusKind::Error, "没有可以复制的内容哦喵~");
            return;
        }
        let encoded = base64(text);
        let sequence = if std::env::var_os("TMUX").is_some() {
            format!("\x1bPtmux;\x1b\x1b]52;c;{encoded}\x07\x1b\\")
        } else {
            format!("\x1b]52;c;{encoded}\x07")
        };
        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(sequence.as_bytes());
        let _ = stdout.flush();
        self.set_status(
            StatusKind::Ok,
            &format!("{label}已经放进剪贴板啦（终端要支持 OSC 52 才行）喵~"),
        );
    }

    pub fn download_asset(&mut self, release_index: usize, asset_index: usize) {
        let Some(asset) = self
            .releases
            .get(release_index)
            .and_then(|release| release.assets.get(asset_index))
            .cloned()
        else {
            return;
        };
        let Some(repo) = self.page_repo.clone().or_else(|| self.selected_repo()) else {
            return;
        };
        self.start_download(
            asset.name.clone(),
            asset.browser_download_url,
            repo.private,
            asset.name,
        );
    }

    pub fn open_release_page(&mut self, index: usize) {
        let Some(url) = self
            .releases
            .get(index)
            .map(|release| release.html_url.clone())
        else {
            return;
        };
        self.open_url(&url);
    }

    pub fn selected_release(&self) -> Option<Release> {
        self.releases.get(self.release_selected).cloned()
    }

    pub fn open_release_detail(&mut self, index: usize) {
        if index >= self.releases.len() {
            return;
        }
        self.release_selected = index;
        self.release_asset_selected = 0;
        self.release_scroll = 0;
        self.release_scroll_max = 0;
        self.search_focus = false;
        self.list_filter_focus = false;
        self.return_page = return_page_when_opening(self.page, self.return_page);
        self.page = Page::ReleaseDetail;
    }

    fn clamp_release_selection(&mut self) {
        if self.releases.is_empty() {
            self.release_selected = 0;
            self.release_asset_selected = 0;
            return;
        }
        self.release_selected = self.release_selected.min(self.releases.len() - 1);
        let assets = self.releases[self.release_selected].assets.len();
        if assets == 0 {
            self.release_asset_selected = 0;
        } else {
            self.release_asset_selected = self.release_asset_selected.min(assets - 1);
        }
    }

    pub fn move_release_asset_selection(&mut self, delta: i32) {
        let Some(release) = self.selected_release() else {
            return;
        };
        if release.assets.is_empty() {
            return;
        }
        self.release_asset_selected = (self.release_asset_selected as i32 + delta)
            .clamp(0, release.assets.len() as i32 - 1)
            as usize;
    }

    pub fn select_release_asset(&mut self, index: usize) {
        let Some(release) = self.selected_release() else {
            return;
        };
        if release.assets.is_empty() {
            return;
        }
        self.release_asset_selected = index.min(release.assets.len() - 1);
    }

    pub fn download_selected_release_asset(&mut self) {
        let selected = self.release_asset_selected;
        self.download_asset(self.release_selected, selected);
    }

    pub fn open_selected_release_page(&mut self) {
        self.open_release_page(self.release_selected);
    }

    pub fn download_source_zip(&mut self) {
        let Some(repo) = self.effective_repo() else {
            return;
        };
        let branch = if repo.default_branch.is_empty() {
            "HEAD".to_string()
        } else {
            repo.default_branch.clone()
        };
        let safe_repo = repo.display_full_name().replace('/', "-");
        let url = format!(
            "https://github.com/{}/archive/refs/heads/{branch}.zip",
            repo.display_full_name()
        );
        self.start_download(
            format!("{safe_repo}-{branch}.zip"),
            url,
            repo.private,
            format!("{safe_repo}-{branch}.zip"),
        );
    }

    fn start_download(&mut self, label: String, url: String, private: bool, filename: String) {
        if self.download_job.is_some() {
            self.set_status(
                StatusKind::Info,
                "上一份还在下载呢，等人家下完再点下一份嘛喵~",
            );
            return;
        }
        let directory = self.config.expanded_download_dir();
        if let Err(error) = std::fs::create_dir_all(&directory) {
            self.set_status(
                StatusKind::Error,
                &format!("下载目录建不出来：{error}喵~到设置里换一个路径嘛喵~"),
            );
            return;
        }
        let filename = safe_download_name(&filename);
        let destination = unique_download_path(&directory, &filename);
        self.download_seq = self.download_seq.wrapping_add(1);
        let id = self.download_seq;
        self.download_job = Some(DownloadJob {
            id,
            label: label.clone(),
            received: 0,
            total: None,
        });
        self.set_status(
            StatusKind::Info,
            &format!(
                "开始下载 {label}，人家放到 {} 里啦喵~",
                format::display_path(&directory)
            ),
        );
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let progress_tx = tx.clone();
            let result =
                client.download_asset(&url, private, &destination, move |received, total| {
                    let _ = progress_tx.send(Msg::DownloadProgress {
                        id,
                        received,
                        total,
                    });
                });
            let result = result.map(|_| destination);
            let _ = tx.send(Msg::DownloadDone { id, label, result });
        });
    }

    pub fn open_download_dir(&mut self) {
        let directory = self.config.expanded_download_dir();
        if let Err(error) = std::fs::create_dir_all(&directory) {
            self.set_status(StatusKind::Error, &format!("下载目录建不出来：{error}喵~"));
            return;
        }
        match open_external(&directory.display().to_string()) {
            Ok(()) => self.set_status(StatusKind::Ok, "下载目录已经打开啦喵~"),
            Err(error) => self.set_status(StatusKind::Error, &error),
        }
    }

    pub fn is_current_starred(&self) -> bool {
        let Some(repo) = self.effective_repo() else {
            return false;
        };
        self.starred
            .iter()
            .any(|item| item.display_full_name() == repo.display_full_name())
    }

    pub fn toggle_current_star(&mut self) {
        let Some(repo) = self.effective_repo() else {
            self.set_status(StatusKind::Info, "先选一个仓库，人家才改得了收藏嘛喵~");
            return;
        };
        if !self.client.token_present() {
            self.set_status(StatusKind::Error, "还没登录呢，人家改不了收藏哦喵~");
            return;
        }
        let name = repo.display_full_name();
        if self.star_loading.is_some() {
            self.set_status(StatusKind::Info, "星星还在路上呢，等人家一下下嘛喵~");
            return;
        }
        let starred = !self.is_current_starred();
        self.star_loading = Some(name.clone());
        self.set_status(
            StatusKind::Info,
            if starred {
                "人家正在把星星点上去喵~..."
            } else {
                "人家正在把星星摘下来喵~..."
            },
        );
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.set_starred(&name, starred);
            let _ = tx.send(Msg::StarToggled {
                name,
                starred,
                result,
            });
        });
    }

    pub fn open_external_url(&mut self, url: &str) {
        self.open_url(url);
    }

    pub fn open_tree_path(&mut self, path: &str, directory: bool) {
        let Some(repo) = self.page_repo.clone().or_else(|| self.selected_repo()) else {
            return;
        };
        let branch = if repo.default_branch.is_empty() {
            "main"
        } else {
            &repo.default_branch
        };
        let kind = if directory { "tree" } else { "blob" };
        let url = format!(
            "{}/{kind}/{branch}/{}",
            repo.html_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        self.open_url(&url);
    }

    pub fn scroll(&mut self, x: u16, y: u16, delta: i32) {
        if self.overlay == Some(Overlay::Languages) {
            let choices = self.language_choices().len();
            let viewport = self.language_viewport.max(1);
            let max = choices.saturating_sub(viewport);
            let next = self.language_offset as i32 + delta;
            self.language_offset = next.clamp(0, max as i32) as usize;
            return;
        }
        if self.overlay.is_some() {
            return;
        }
        let layout = self.layout;
        if self.page == Page::Repo {
            if layout.contains(layout.page_tree, x, y) {
                self.repo_pane = RepoPane::Tree;
                let max = self.tree_scroll_max as i32;
                self.tree_scroll = (self.tree_scroll as i32 + 3 * delta).clamp(0, max) as u16;
                return;
            }
            if layout.contains(layout.page_readme, x, y) {
                self.repo_pane = RepoPane::Readme;
                let max = self.readme_scroll_max as i32;
                self.readme_scroll = (self.readme_scroll as i32 + 3 * delta).clamp(0, max) as u16;
                return;
            }
            if layout.contains(layout.page_releases, x, y) {
                self.repo_pane = RepoPane::Releases;
                let max = self.page_scroll_max as i32;
                self.page_scroll = (self.page_scroll as i32 + 3 * delta).clamp(0, max) as u16;
                return;
            }
        }
        if self.page == Page::Settings {
            if delta > 0 {
                self.settings_row = (self.settings_row + 1).min(9);
            } else {
                self.settings_row = self.settings_row.saturating_sub(1);
            }
            return;
        }
        if self.page == Page::ReleaseDetail {
            if layout.contains(layout.detail, x, y) {
                let step = 3_i32 * delta;
                let max = self.release_scroll_max as i32;
                self.release_scroll = (self.release_scroll as i32 + step).clamp(0, max) as u16;
            } else if layout.contains(layout.page_releases, x, y) && delta != 0 {
                self.move_release_asset_selection(delta.signum());
            }
            return;
        }
        if self.page == Page::Notifications {
            if layout.contains(layout.list, x, y) && delta != 0 {
                self.move_notification_selection(delta.signum());
            }
            return;
        }
        if self.page == Page::Issues {
            if layout.contains(layout.list, x, y) && delta != 0 {
                self.move_issue_selection(delta.signum());
            }
            return;
        }
        if layout.contains(layout.list, x, y) {
            if delta != 0 {
                self.move_selection(delta.signum());
            }
        } else if layout.contains(layout.detail, x, y) {
            let step = 3_i32 * delta;
            let max = self.detail_scroll_max as i32;
            self.detail_scroll = (self.detail_scroll as i32 + step).clamp(0, max) as u16;
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != crossterm::event::KeyEventKind::Press
            && key.kind != crossterm::event::KeyEventKind::Repeat
        {
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
        {
            self.quit();
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('s') | KeyCode::Char('S'))
        {
            if self.overlay == Some(Overlay::EditRepo) {
                self.save_repo_edit();
            } else {
                if self.editing_proxy {
                    self.commit_proxy_edit();
                }
                if self.editing_download_dir {
                    self.commit_download_dir_edit();
                }
                self.save_config(true);
            }
            return;
        }
        if self.overlay == Some(Overlay::EditRepo) {
            self.handle_repo_edit_key(key);
            return;
        }
        if self.overlay == Some(Overlay::Cache) {
            self.handle_cache_key(key);
            return;
        }
        if self.overlay == Some(Overlay::RunWorkflow) {
            self.handle_workflow_key(key);
            return;
        }
        if self.overlay == Some(Overlay::Languages) {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => self.overlay = None,
                KeyCode::Up | KeyCode::Char('k') => self.move_language_selection(-1),
                KeyCode::Down | KeyCode::Char('j') => self.move_language_selection(1),
                KeyCode::Char('g') | KeyCode::Home => {
                    self.language_selected = 0;
                    self.language_offset = 0;
                }
                KeyCode::Char('G') | KeyCode::End => {
                    let len = self.language_choices().len();
                    if len > 0 {
                        self.language_selected = len - 1;
                        let viewport = self.language_viewport.max(1);
                        self.language_offset = len.saturating_sub(viewport);
                    }
                }
                KeyCode::Enter => self.choose_language_selection(),
                _ => {}
            }
            return;
        }
        if self.overlay.is_some() {
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')
            ) {
                self.overlay = None;
            }
            return;
        }

        if self.editing_proxy {
            match key.code {
                KeyCode::Enter => self.commit_proxy_edit(),
                KeyCode::Esc => {
                    self.editing_proxy = false;
                    self.proxy_input = TextInput::new(self.config.proxy.clone());
                }
                KeyCode::Backspace => self.proxy_input.backspace(),
                KeyCode::Delete => self.proxy_input.delete(),
                KeyCode::Left => self.proxy_input.left(),
                KeyCode::Right => self.proxy_input.right(),
                KeyCode::Home => self.proxy_input.home(),
                KeyCode::End => self.proxy_input.end(),
                KeyCode::Char(ch) => self.proxy_input.insert(ch),
                _ => {}
            }
            return;
        }

        if self.editing_download_dir {
            match key.code {
                KeyCode::Enter => self.commit_download_dir_edit(),
                KeyCode::Esc => {
                    self.editing_download_dir = false;
                    self.download_input = TextInput::new(self.config.download_dir.clone());
                }
                KeyCode::Backspace => self.download_input.backspace(),
                KeyCode::Delete => self.download_input.delete(),
                KeyCode::Left => self.download_input.left(),
                KeyCode::Right => self.download_input.right(),
                KeyCode::Home => self.download_input.home(),
                KeyCode::End => self.download_input.end(),
                KeyCode::Char(ch) => self.download_input.insert(ch),
                _ => {}
            }
            return;
        }

        if self.search_focus {
            match key.code {
                KeyCode::Enter => self.submit_search(),
                KeyCode::Esc => self.search_focus = false,
                KeyCode::Backspace => self.search_input.backspace(),
                KeyCode::Delete => self.search_input.delete(),
                KeyCode::Left => self.search_input.left(),
                KeyCode::Right => self.search_input.right(),
                KeyCode::Home => self.search_input.home(),
                KeyCode::End => self.search_input.end(),
                KeyCode::Tab => {
                    self.search_focus = false;
                    self.navigate_next_page();
                }
                KeyCode::Char(ch) => self.search_input.insert(ch),
                _ => {}
            }
            return;
        }

        if self.list_filter_focus {
            match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    self.list_filter_focus = false;
                }
                KeyCode::Tab => {
                    self.list_filter_focus = false;
                    self.navigate_next_page();
                }
                KeyCode::Backspace => self.filter_for_mut(self.page).backspace(),
                KeyCode::Delete => self.filter_for_mut(self.page).delete(),
                KeyCode::Left => self.filter_for_mut(self.page).left(),
                KeyCode::Right => self.filter_for_mut(self.page).right(),
                KeyCode::Home => self.filter_for_mut(self.page).home(),
                KeyCode::End => self.filter_for_mut(self.page).end(),
                KeyCode::Char(ch) => self.filter_for_mut(self.page).insert(ch),
                _ => {}
            }
            self.clamp_selection();
            return;
        }

        if self.release_filter_focus {
            match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    self.release_filter_focus = false;
                }
                KeyCode::Tab => {
                    self.release_filter_focus = false;
                    self.cycle_repo_pane();
                }
                KeyCode::Backspace => self.release_filter.backspace(),
                KeyCode::Delete => self.release_filter.delete(),
                KeyCode::Left => self.release_filter.left(),
                KeyCode::Right => self.release_filter.right(),
                KeyCode::Home => self.release_filter.home(),
                KeyCode::End => self.release_filter.end(),
                KeyCode::Char(ch) => self.release_filter.insert(ch),
                _ => {}
            }
            self.page_scroll = 0;
            return;
        }

        if self.settings_only {
            match key.code {
                KeyCode::Esc => {
                    self.quit();
                    return;
                }
                KeyCode::Tab => {
                    self.settings_row = (self.settings_row + 1).min(9);
                    return;
                }
                KeyCode::BackTab => {
                    self.settings_row = self.settings_row.saturating_sub(1);
                    return;
                }
                KeyCode::Char('h') => {
                    self.adjust_settings(-1);
                    return;
                }
                KeyCode::Char('l') => {
                    self.adjust_settings(1);
                    return;
                }
                KeyCode::Char('?') => {
                    self.set_status(
                        StatusKind::Info,
                        "j/k 选  h/l 调  Tab 换行  Enter 操作  Ctrl+S 保存  Esc 退出喵~",
                    );
                    return;
                }
                KeyCode::Char('1'..='5')
                | KeyCode::Char('/')
                | KeyCode::Char('s')
                | KeyCode::Char('a')
                | KeyCode::Char('R')
                | KeyCode::Char('x')
                | KeyCode::Char('y')
                | KeyCode::Char('f')
                | KeyCode::Char('e')
                | KeyCode::Char('o')
                | KeyCode::Char('t') => return,
                _ => {}
            }
        }

        if self.page == Page::Repo && key.code == KeyCode::Tab {
            self.cycle_repo_pane();
            return;
        }
        if self.page == Page::Issues {
            match key.code {
                KeyCode::Char('v') => {
                    self.cycle_issue_kind();
                    return;
                }
                KeyCode::Char('s') => {
                    self.cycle_issue_state();
                    return;
                }
                _ => {}
            }
        }
        if self.page == Page::Notifications {
            match key.code {
                KeyCode::Char('u') => {
                    self.toggle_notification_filter();
                    return;
                }
                KeyCode::Char('m') => {
                    self.mark_selected_notification_read();
                    return;
                }
                KeyCode::Char('M') => {
                    self.mark_all_notifications_read();
                    return;
                }
                _ => {}
            }
        }
        match key.code {
            KeyCode::Char('q') => self.quit(),
            KeyCode::Char('?') => self.overlay = Some(Overlay::Help),
            KeyCode::Char('/') if matches!(self.page, Page::Repositories | Page::Starred) => {
                self.focus_list_filter()
            }
            KeyCode::Char('/')
                if self.page == Page::Repo && self.repo_pane == RepoPane::Releases =>
            {
                self.focus_release_filter()
            }
            KeyCode::Char('/') | KeyCode::Char('s') => self.focus_search(),
            KeyCode::Char('1') => self.navigate(Page::Repositories),
            KeyCode::Char('2') => self.navigate(Page::Starred),
            KeyCode::Char('3') => self.navigate(Page::Search),
            KeyCode::Char('4') => self.open_account(),
            KeyCode::Char('5') => self.navigate(Page::Notifications),
            KeyCode::Tab => self.navigate_next_page(),
            KeyCode::Esc => match self.page {
                Page::Repo | Page::Actions | Page::Issues | Page::ReleaseDetail => self.go_back(),
                Page::Settings | Page::Account | Page::Notifications => {
                    self.navigate(Page::Repositories)
                }
                _ => {}
            },
            KeyCode::Char('r') => self.refresh_current(),
            KeyCode::Char('o') => match self.page {
                Page::Actions => self.open_action_run(None),
                Page::Issues => self.open_selected_issue(),
                Page::ReleaseDetail => self.open_selected_release_page(),
                Page::Notifications => self.open_selected_notification(),
                _ => self.open_current(),
            },
            KeyCode::Char('a') => self.open_actions(),
            KeyCode::Char('i') => self.open_issues(IssueKind::All),
            KeyCode::Char('R') | KeyCode::Char('x') if self.page == Page::Actions => {
                self.open_workflow_runner()
            }
            KeyCode::Char('c') => self.open_cache_manager(),
            KeyCode::Char('y') => self.copy_clone(),
            KeyCode::Char('f') => self.toggle_current_star(),
            KeyCode::Char('e') => self.open_repo_edit(),
            KeyCode::Char('[') => self.prev_list_page(),
            KeyCode::Char(']') => self.next_list_page(),
            KeyCode::Char('l') => {
                if self.page == Page::Repositories {
                    self.open_languages();
                }
            }
            KeyCode::Char('t') => self.cycle_theme(1),
            KeyCode::Enter => match self.page {
                Page::Settings => self.activate_settings_row(),
                Page::Repo => self.open_current(),
                Page::Account => self.open_profile(),
                Page::Actions => self.open_action_run(None),
                Page::Issues => self.open_selected_issue(),
                Page::ReleaseDetail => self.download_selected_release_asset(),
                Page::Notifications => self.open_selected_notification(),
                _ => self.open_repo_page(),
            },
            KeyCode::Up | KeyCode::Char('k') => self.key_up(),
            KeyCode::Down | KeyCode::Char('j') => self.key_down(),
            KeyCode::Char('g') | KeyCode::Home => self.key_home(),
            KeyCode::Char('G') | KeyCode::End => self.key_end(),
            KeyCode::PageUp => self.key_page_up(),
            KeyCode::PageDown => self.key_page_down(),
            KeyCode::Left => self.key_left(),
            KeyCode::Right => self.key_right(),
            _ => {}
        }
    }

    fn navigate_next_page(&mut self) {
        let current = self
            .page
            .nav_index()
            .unwrap_or_else(|| self.return_page.nav_index().unwrap_or(0));
        let next = (current + 1) % 5;
        self.navigate(Page::from_nav(next));
    }

    fn key_up(&mut self) {
        match self.page {
            Page::Repo => self.scroll_repo_pane(-1),
            Page::Actions => self.move_action_selection(-1),
            Page::Issues => self.move_issue_selection(-1),
            Page::ReleaseDetail => self.move_release_asset_selection(-1),
            Page::Notifications => self.move_notification_selection(-1),
            Page::Settings => self.settings_row = self.settings_row.saturating_sub(1),
            _ => self.move_selection(-1),
        }
    }

    fn key_down(&mut self) {
        match self.page {
            Page::Repo => self.scroll_repo_pane(1),
            Page::Actions => self.move_action_selection(1),
            Page::Issues => self.move_issue_selection(1),
            Page::ReleaseDetail => self.move_release_asset_selection(1),
            Page::Notifications => self.move_notification_selection(1),
            Page::Settings => self.settings_row = (self.settings_row + 1).min(9),
            _ => self.move_selection(1),
        }
    }

    fn key_home(&mut self) {
        match self.page {
            Page::Repo => self.set_repo_pane_scroll(0),
            Page::Settings => self.settings_row = 0,
            Page::Actions => self.select_action_run(0),
            Page::Issues => self.select_issue(0),
            Page::ReleaseDetail => self.select_release_asset(0),
            Page::Notifications => self.notification_selected = 0,
            Page::Account => {}
            _ => self.first_list_page(),
        }
    }

    fn key_end(&mut self) {
        match self.page {
            Page::Repo => self.set_repo_pane_scroll(u16::MAX),
            Page::Settings => self.settings_row = 9,
            Page::Actions => {
                self.select_action_run(self.action_runs.len().saturating_sub(1));
            }
            Page::Issues => {
                let last = self.issue_indices().len().saturating_sub(1);
                self.select_issue(last);
            }
            Page::ReleaseDetail => {
                let last = self
                    .selected_release()
                    .map(|release| release.assets.len().saturating_sub(1))
                    .unwrap_or(0);
                self.select_release_asset(last);
            }
            Page::Notifications => {
                self.notification_selected = self.notifications.len().saturating_sub(1);
            }
            Page::Account => {}
            _ => self.last_list_page(),
        }
    }

    fn key_page_up(&mut self) {
        match self.page {
            Page::Repo => self.scroll_repo_pane(-10),
            Page::Actions => {
                self.action_jobs_scroll = self.action_jobs_scroll.saturating_sub(10);
            }
            Page::Issues => self.move_issue_selection(-5),
            Page::ReleaseDetail => {
                self.release_scroll = self.release_scroll.saturating_sub(10);
            }
            Page::Notifications => self.move_notification_selection(-5),
            Page::Settings => self.settings_row = self.settings_row.saturating_sub(3),
            Page::Account => {}
            _ => self.prev_list_page(),
        }
    }

    fn key_page_down(&mut self) {
        match self.page {
            Page::Repo => self.scroll_repo_pane(10),
            Page::Actions => {
                self.action_jobs_scroll =
                    (self.action_jobs_scroll + 10).min(self.action_jobs_scroll_max);
            }
            Page::Issues => self.move_issue_selection(5),
            Page::ReleaseDetail => {
                self.release_scroll = (self.release_scroll + 10).min(self.release_scroll_max);
            }
            Page::Notifications => self.move_notification_selection(5),
            Page::Settings => self.settings_row = (self.settings_row + 3).min(9),
            Page::Account => {}
            _ => self.next_list_page(),
        }
    }

    pub fn cycle_repo_pane(&mut self) {
        self.release_filter_focus = false;
        let order = [RepoPane::Tree, RepoPane::Releases, RepoPane::Readme];
        let enabled = |pane: RepoPane| match pane {
            RepoPane::Tree => self.config.show_file_tree,
            RepoPane::Readme => self.config.show_readme,
            RepoPane::Releases => true,
        };
        let start = order
            .iter()
            .position(|pane| *pane == self.repo_pane)
            .unwrap_or(1);
        for step in 1..=order.len() {
            let pane = order[(start + step) % order.len()];
            if enabled(pane) {
                self.repo_pane = pane;
                return;
            }
        }
    }

    pub fn set_repo_pane(&mut self, pane: RepoPane) {
        self.release_filter_focus = false;
        let available = match pane {
            RepoPane::Tree => self.config.show_file_tree,
            RepoPane::Readme => self.config.show_readme,
            RepoPane::Releases => true,
        };
        if available {
            self.repo_pane = pane;
        }
    }

    fn scroll_repo_pane(&mut self, delta: i32) {
        match self.repo_pane {
            RepoPane::Tree => {
                let max = self.tree_scroll_max as i32;
                self.tree_scroll = (self.tree_scroll as i32 + delta).clamp(0, max) as u16;
            }
            RepoPane::Readme => {
                let max = self.readme_scroll_max as i32;
                self.readme_scroll = (self.readme_scroll as i32 + delta).clamp(0, max) as u16;
            }
            RepoPane::Releases => {
                let max = self.page_scroll_max as i32;
                self.page_scroll = (self.page_scroll as i32 + delta).clamp(0, max) as u16;
            }
        }
    }

    fn set_repo_pane_scroll(&mut self, value: u16) {
        match self.repo_pane {
            RepoPane::Tree => {
                self.tree_scroll = if value == u16::MAX {
                    self.tree_scroll_max
                } else {
                    value
                };
            }
            RepoPane::Readme => {
                self.readme_scroll = if value == u16::MAX {
                    self.readme_scroll_max
                } else {
                    value
                };
            }
            RepoPane::Releases => {
                self.page_scroll = if value == u16::MAX {
                    self.page_scroll_max
                } else {
                    value
                };
            }
        }
    }

    fn key_left(&mut self) {
        if self.page == Page::Settings {
            self.adjust_settings(-1);
        }
    }

    fn key_right(&mut self) {
        if self.page == Page::Settings {
            self.adjust_settings(1);
        }
    }

    fn adjust_settings(&mut self, direction: i32) {
        match self.settings_row {
            0 => self.cycle_theme(direction),
            1 => self.toggle_file_tree(),
            2 => self.toggle_readme(),
            5 => self.adjust_ttl(TtlKind::Repositories, direction),
            6 => self.adjust_ttl(TtlKind::Detail, direction),
            7 => self.adjust_ttl(TtlKind::Search, direction),
            _ => {}
        }
    }

    fn activate_settings_row(&mut self) {
        match self.settings_row {
            0 => self.cycle_theme(1),
            1 => self.toggle_file_tree(),
            2 => self.toggle_readme(),
            3 => self.start_download_dir_edit(),
            4 => self.start_proxy_edit(),
            5 => self.adjust_ttl(TtlKind::Repositories, 1),
            6 => self.adjust_ttl(TtlKind::Detail, 1),
            7 => self.adjust_ttl(TtlKind::Search, 1),
            8 => self.clear_cache(),
            9 => self.save_config(true),
            _ => {}
        }
    }

    pub fn cycle_theme(&mut self, direction: i32) {
        let index = FLAVORS
            .iter()
            .position(|flavor| *flavor == self.theme.flavor)
            .unwrap_or(3) as i32;
        let next = (index + direction).rem_euclid(FLAVORS.len() as i32) as usize;
        self.set_theme(FLAVORS[next].to_string());
    }

    pub fn set_theme(&mut self, flavor: String) {
        if !FLAVORS.contains(&flavor.as_str()) {
            return;
        }
        self.theme = Theme::load(&self.paths, &flavor);
        self.config.theme = flavor.clone();
        self.config_dirty = true;
        self.set_status(
            StatusKind::Info,
            &format!(
                "主题换成 {} 啦，记得到设置页保存哦喵~",
                Theme::label(&flavor)
            ),
        );
    }

    pub fn start_proxy_edit(&mut self) {
        self.editing_proxy = true;
        self.proxy_input = TextInput::new(self.config.proxy.clone());
        self.settings_row = 4;
    }

    pub fn start_download_dir_edit(&mut self) {
        self.editing_download_dir = true;
        self.download_input = TextInput::new(self.config.download_dir.clone());
        self.settings_row = 3;
    }

    fn commit_download_dir_edit(&mut self) {
        let value = self.download_input.value.trim().to_string();
        if value.is_empty() {
            self.set_status(StatusKind::Error, "下载目录不能空着呀，给人家一个路径嘛喵~");
            return;
        }
        let expanded = crate::config::expand_tilde(&value);
        if !expanded.is_absolute() {
            self.set_status(
                StatusKind::Error,
                "下载目录最好写绝对路径，或者用 ~/ 开头的路径哦喵~",
            );
            return;
        }
        self.config.download_dir = value;
        self.editing_download_dir = false;
        self.config_dirty = true;
        if let Err(error) = std::fs::create_dir_all(self.config.expanded_download_dir()) {
            self.set_status(
                StatusKind::Error,
                &format!("下载目录建不出来：{error}，换一个路径试试嘛喵~"),
            );
            return;
        }
        self.set_status(
            StatusKind::Info,
            &format!(
                "下载目录改成 {} 啦，记得保存哦喵~",
                self.download_path_label()
            ),
        );
    }

    pub fn toggle_file_tree(&mut self) {
        self.settings_row = 1;
        self.config.show_file_tree = !self.config.show_file_tree;
        self.config_dirty = true;
        if !self.config.show_file_tree && self.repo_pane == RepoPane::Tree {
            self.repo_pane = RepoPane::Releases;
        }
        if self.config.show_file_tree
            && self.page == Page::Repo
            && let Some(repo) = self.page_repo.clone()
        {
            self.schedule_repo_extras(&repo, false);
        }
        self.set_status(
            StatusKind::Info,
            if self.config.show_file_tree {
                "文件树预览打开啦，到仓库页就能看到呢喵~"
            } else {
                "文件树预览关掉啦喵~"
            },
        );
    }

    pub fn toggle_readme(&mut self) {
        self.settings_row = 2;
        self.config.show_readme = !self.config.show_readme;
        self.config_dirty = true;
        if !self.config.show_readme && self.repo_pane == RepoPane::Readme {
            self.repo_pane = RepoPane::Releases;
        }
        if self.config.show_readme
            && self.page == Page::Repo
            && let Some(repo) = self.page_repo.clone()
        {
            self.schedule_repo_extras(&repo, false);
        }
        self.set_status(
            StatusKind::Info,
            if self.config.show_readme {
                "README 预览打开啦，会放在仓库页最下面哦喵~"
            } else {
                "README 预览关掉啦喵~"
            },
        );
    }

    fn commit_proxy_edit(&mut self) {
        let value = self
            .proxy_input
            .value
            .trim()
            .trim_end_matches('/')
            .to_string();
        if !(value.starts_with("https://") || value.starts_with("http://")) {
            self.set_status(
                StatusKind::Error,
                "代理地址要以 http:// 或 https:// 开头哦喵~",
            );
            return;
        }
        self.config.proxy = value.clone();
        self.client.set_proxy(&value);
        self.editing_proxy = false;
        self.config_dirty = true;
        self.set_status(
            StatusKind::Info,
            "代理地址改好啦，保存之后才会写进配置文件哦喵~",
        );
    }

    pub fn adjust_ttl(&mut self, kind: TtlKind, direction: i32) {
        let current = match kind {
            TtlKind::Repositories => self.config.repos_ttl_secs,
            TtlKind::Detail => self.config.detail_ttl_secs,
            TtlKind::Search => self.config.search_ttl_secs,
        };
        let next = cycle_ttl(current, direction);
        match kind {
            TtlKind::Repositories => self.config.repos_ttl_secs = next,
            TtlKind::Detail => {
                self.config.detail_ttl_secs = next;
            }
            TtlKind::Search => self.config.search_ttl_secs = next,
        }
        self.config_dirty = true;
        self.set_status(
            StatusKind::Info,
            &format!("缓存时间调成 {} 了，记得保存哦喵~", format::ttl_label(next)),
        );
    }

    pub fn save_config(&mut self, announce: bool) {
        if let Err(error) = self.config.save(&self.paths) {
            self.set_status(
                StatusKind::Error,
                &format!("配置文件没写进去呢：{error}喵~"),
            );
            return;
        }
        self.client.set_proxy(&self.config.proxy);
        let _ = std::fs::create_dir_all(self.config.expanded_download_dir());
        self.config_dirty = false;
        if announce {
            self.set_status(
                StatusKind::Ok,
                &format!(
                    "配置写好啦：{}喵~",
                    format::display_path(&self.paths.config_file())
                ),
            );
        }
    }

    pub fn clear_cache(&mut self) {
        match self.cache.clear() {
            Ok(()) => {
                self.refresh_cache_stats();
                self.set_status(StatusKind::Ok, "缓存清干净啦，下次人家会重新取喵~");
            }
            Err(error) => self.set_status(StatusKind::Error, &format!("缓存没清掉呢：{error}喵~")),
        }
    }

    pub fn handle_settings_row(&mut self, row: usize) {
        self.settings_row = row.min(9);
    }

    pub fn handle_paste(&mut self, text: &str) {
        if self.overlay == Some(Overlay::EditRepo) {
            if let Some(edit) = self.repo_edit.as_mut()
                && edit.editing
            {
                for ch in text.chars().filter(|ch| *ch != '\n' && *ch != '\r') {
                    match edit.row {
                        0 => edit.description.insert(ch),
                        1 => edit.homepage.insert(ch),
                        2 => edit.topics.insert(ch),
                        _ => {}
                    }
                }
            }
            return;
        }
        if self.overlay == Some(Overlay::RunWorkflow) {
            if self.workflow_ref_editing {
                for ch in text.chars().filter(|ch| *ch != '\n' && *ch != '\r') {
                    self.workflow_ref.insert(ch);
                }
                return;
            }
            if self.workflow_inputs_editing {
                for ch in text.chars().filter(|ch| *ch != '\n' && *ch != '\r') {
                    self.workflow_inputs.insert(ch);
                }
                return;
            }
        }
        if self.overlay.is_some() {
            return;
        }
        if self.editing_download_dir {
            for ch in text.chars().filter(|ch| *ch != '\n' && *ch != '\r') {
                self.download_input.insert(ch);
            }
        } else if self.editing_proxy {
            for ch in text.chars().filter(|ch| *ch != '\n' && *ch != '\r') {
                self.proxy_input.insert(ch);
            }
        } else if self.release_filter_focus {
            for ch in text.chars().filter(|ch| *ch != '\n' && *ch != '\r') {
                self.release_filter.insert(ch);
            }
            self.page_scroll = 0;
        } else if self.list_filter_focus {
            let page = self.page;
            for ch in text.chars().filter(|ch| *ch != '\n' && *ch != '\r') {
                self.filter_for_mut(page).insert(ch);
            }
            self.clamp_selection();
        } else if self.search_focus {
            for ch in text.chars().filter(|ch| *ch != '\n' && *ch != '\r') {
                self.search_input.insert(ch);
            }
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::Moved => {
                if self.overlay == Some(Overlay::Help) {
                    self.hover = None;
                    return;
                }
                self.hover = self
                    .hit_at(mouse.column, mouse.row)
                    .map(|hit| hit.action.clone());
            }
            MouseEventKind::ScrollUp => self.scroll(mouse.column, mouse.row, -1),
            MouseEventKind::ScrollDown => self.scroll(mouse.column, mouse.row, 1),
            MouseEventKind::Down(MouseButton::Left) => {
                if self.overlay == Some(Overlay::Help) {
                    self.overlay = None;
                    return;
                }
                if self.overlay == Some(Overlay::Languages) {
                    if let Some(hit) = self.hit_at(mouse.column, mouse.row).cloned() {
                        self.handle_action(hit.action);
                    } else if !self
                        .layout
                        .contains(self.layout.overlay, mouse.column, mouse.row)
                    {
                        self.overlay = None;
                    }
                    return;
                }
                if let Some(hit) = self.hit_at(mouse.column, mouse.row).cloned() {
                    self.handle_action(hit.action);
                }
            }
            _ => {}
        }
    }

    fn hit_at(&self, x: u16, y: u16) -> Option<&HitBox> {
        self.hitboxes.iter().rev().find(|hit| {
            x >= hit.rect.x
                && x < hit.rect.x.saturating_add(hit.rect.width)
                && y >= hit.rect.y
                && y < hit.rect.y.saturating_add(hit.rect.height)
        })
    }

    pub fn handle_action(&mut self, action: Action) {
        match action {
            Action::Navigate(page) => self.navigate(page),
            Action::SelectVisible(index) => {
                let key = format!("{}:{}", self.page.title(), index);
                let now = Instant::now();
                let double = self
                    .last_click
                    .as_ref()
                    .map(|(at, last)| at.elapsed() < Duration::from_millis(420) && last == &key)
                    .unwrap_or(false);
                self.last_click = Some((now, key));
                self.search_focus = false;
                self.list_filter_focus = false;
                self.select_visible(index);
                if double {
                    self.open_repo_page();
                }
            }
            Action::Language(language) => self.set_language(language),
            Action::SetRepoSort(sort) => self.set_repo_sort(sort),
            Action::SetStarSort(sort) => self.set_star_sort(sort),
            Action::SetSearchSort(sort) => self.set_search_sort(sort),
            Action::SearchFocus => self.focus_search(),
            Action::ListFilterFocus => self.focus_list_filter(),
            Action::ReleaseFilterFocus => self.focus_release_filter(),
            Action::SearchHistory(query) => self.select_search_history(query),
            Action::SubmitSearch => self.submit_search(),
            Action::OpenPage => self.open_repo_page(),
            Action::Back => self.go_back(),
            Action::Refresh => self.refresh_current(),
            Action::OpenCurrent => self.open_current(),
            Action::CopyClone => self.copy_clone(),
            Action::SetTheme(flavor) => self.set_theme(flavor),
            Action::StartProxyEdit => self.start_proxy_edit(),
            Action::StartDownloadDirEdit => self.start_download_dir_edit(),
            Action::ToggleFileTree => self.toggle_file_tree(),
            Action::ToggleReadme => self.toggle_readme(),
            Action::Ttl(kind, direction) => self.adjust_ttl(kind, direction),
            Action::ClearCache => self.clear_cache(),
            Action::SaveConfig => self.save_config(true),
            Action::OpenDownloadDir => self.open_download_dir(),
            Action::SetSettingsRow(row) => self.handle_settings_row(row),
            Action::OpenLanguages => self.open_languages(),
            Action::DownloadAsset { release, asset } => self.download_asset(release, asset),
            Action::DownloadSource => self.download_source_zip(),
            Action::OpenReleasePage(index) => self.open_release_page(index),
            Action::OpenReleaseDetail(index) => self.open_release_detail(index),
            Action::OpenTreePath { path, directory } => self.open_tree_path(&path, directory),
            Action::FocusRepoPane(pane) => self.set_repo_pane(pane),
            Action::OpenExternal(url) => self.open_external_url(&url),
            Action::ToggleStar => self.toggle_current_star(),
            Action::CloseOverlay => {
                self.overlay = None;
                self.repo_edit = None;
            }
            Action::PrevListPage => self.prev_list_page(),
            Action::NextListPage => self.next_list_page(),
            Action::OpenRepoEdit => self.open_repo_edit(),
            Action::EditRepoRow(row) => {
                if let Some(edit) = self.repo_edit.as_mut() {
                    edit.row = row.min(4);
                    edit.editing = row <= 2;
                    if row == 3 {
                        edit.private = !edit.private;
                    } else if row == 4 {
                        edit.archived = !edit.archived;
                    }
                }
            }
            Action::SaveRepoEdit => self.save_repo_edit(),
            Action::RefreshRateLimit => self.fetch_rate_limit(true),
            Action::OpenActions => self.open_actions(),
            Action::SelectActionRun(index) => self.select_action_run(index),
            Action::OpenIssues => self.open_issues(IssueKind::Issues),
            Action::OpenPullRequests => self.open_issues(IssueKind::PullRequests),
            Action::SelectIssue(index) => {
                let key = format!("{}:{}", self.page.title(), index);
                let now = Instant::now();
                let double = self
                    .last_click
                    .as_ref()
                    .map(|(at, last)| at.elapsed() < Duration::from_millis(420) && last == &key)
                    .unwrap_or(false);
                self.last_click = Some((now, key));
                self.select_issue(index);
                if double {
                    self.open_selected_issue();
                }
            }
            Action::SetIssueKind(kind) => self.set_issue_kind(kind),
            Action::SetIssueState(state) => self.set_issue_state(state),
            Action::SelectNotification(index) => {
                let key = format!("{}:{}", self.page.title(), index);
                let now = Instant::now();
                let double = self
                    .last_click
                    .as_ref()
                    .map(|(at, last)| at.elapsed() < Duration::from_millis(420) && last == &key)
                    .unwrap_or(false);
                self.last_click = Some((now, key));
                self.select_notification(index);
                if double {
                    self.open_selected_notification();
                }
            }
            Action::SetNotificationFilter(all) => self.set_notification_filter(all),
            Action::MarkNotificationRead => self.mark_selected_notification_read(),
            Action::MarkAllNotificationsRead => self.mark_all_notifications_read(),
            Action::OpenCacheManager => self.open_cache_manager(),
            Action::ClearCacheGroup(index) => self.clear_cache_group(index),
            Action::SelectCacheRow(index) => {
                let last = cache_groups().len().saturating_sub(1);
                self.cache_row = index.min(last);
            }
            Action::OpenCacheDir => self.open_cache_dir(),
            Action::OpenWorkflowRunner => self.open_workflow_runner(),
            Action::SelectWorkflow(index) => {
                let last = self.action_workflows.len().saturating_sub(1);
                self.workflow_selected = index.min(last);
            }
            Action::EditWorkflowRef => {
                self.workflow_ref_editing = true;
                self.workflow_inputs_editing = false;
            }
            Action::EditWorkflowInputs => {
                self.workflow_inputs_editing = true;
                self.workflow_ref_editing = false;
            }
            Action::RunWorkflow => self.dispatch_selected_workflow(),
            Action::RefreshWorkflows => self.fetch_workflows(true),
        }
    }

    pub fn quit(&mut self) {
        if self.editing_download_dir {
            self.commit_download_dir_edit();
        }
        if self.editing_proxy {
            self.commit_proxy_edit();
        }
        if self.config_dirty {
            self.save_config(false);
        }
        self.should_quit = true;
    }

    pub fn config_file_label(&self) -> String {
        format::display_path(&self.paths.config_file())
    }
}

fn return_page_when_opening(current: Page, return_page: Page) -> Page {
    if current.is_repo_page() {
        return_page
    } else {
        current
    }
}

fn back_target(page: Page, return_page: Page, has_repo: bool) -> Option<Page> {
    if !page.is_repo_page() {
        return None;
    }
    if page != Page::Repo && has_repo {
        return Some(Page::Repo);
    }
    if return_page.is_repo_page() {
        Some(Page::Repositories)
    } else {
        Some(return_page)
    }
}

fn tree_cache_key(name: &str, branch: &str) -> String {
    format!("tree_{}_{}", key_part(name), key_part(branch))
}

fn readme_cache_key(name: &str) -> String {
    format!("readme_{}", key_part(name))
}

fn actions_cache_key(name: &str) -> String {
    format!("actions_{}", key_part(name))
}

fn action_jobs_cache_key(run_id: u64) -> String {
    format!("actions_jobs_{run_id}")
}

fn workflows_cache_key(name: &str) -> String {
    format!("actions_workflows_{}", key_part(name))
}

fn issues_cache_key(name: &str, state: IssueState) -> String {
    format!("issues_{}_{}", key_part(name), state.api_value())
}

fn notifications_cache_key(all: bool) -> String {
    format!("notifications_{}", if all { "all" } else { "unread" })
}

fn parse_workflow_inputs(raw: &str) -> Result<Option<serde_json::Value>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "{}" {
        return Ok(None);
    }
    let value: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|err| format!("inputs 要写成 JSON 对象喵~（{err}）"))?;
    match value {
        serde_json::Value::Object(map) if !map.is_empty() => {
            Ok(Some(serde_json::Value::Object(map)))
        }
        serde_json::Value::Object(_) => Ok(None),
        _ => Err(r#"inputs 要写成 JSON 对象，比如 {"env":"prod"}喵~"#.to_string()),
    }
}

fn safe_download_name(name: &str) -> String {
    let base = std::path::Path::new(name)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("download");
    let mut out: String = base
        .chars()
        .filter(|ch| {
            !ch.is_control() && !matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
        .take(180)
        .collect();
    out = out.trim().trim_matches('.').to_string();
    if out.is_empty() {
        "download".to_string()
    } else {
        out
    }
}

fn unique_download_path(directory: &std::path::Path, filename: &str) -> std::path::PathBuf {
    let path = directory.join(filename);
    if !path.exists() {
        return path;
    }
    let path_buf = std::path::Path::new(filename);
    let stem = path_buf
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(filename);
    let extension = path_buf
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{value}"))
        .unwrap_or_default();
    for index in 1..=10_000 {
        let candidate = directory.join(format!("{stem} ({index}){extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    directory.join(format!("{stem}-{}{extension}", std::process::id()))
}

fn sort_star_indices(raw: &[Repo], indices: &mut [usize], sort: StarSort) {
    match sort {
        StarSort::Starred => indices.sort_by(|left, right| {
            let a = raw[*left].starred_at.as_deref();
            let b = raw[*right].starred_at.as_deref();
            match (a, b) {
                (Some(a), Some(b)) => b.cmp(a),
                (None, None) => std::cmp::Ordering::Equal,
                (None, Some(_)) => std::cmp::Ordering::Less,
                (Some(_), None) => std::cmp::Ordering::Greater,
            }
        }),
        StarSort::Updated => sort_indices(raw, indices, RepoSort::Updated),
        StarSort::Stars => sort_indices(raw, indices, RepoSort::Stars),
        StarSort::Name => sort_indices(raw, indices, RepoSort::Name),
    }
}

fn sort_indices(raw: &[Repo], indices: &mut [usize], sort: RepoSort) {
    indices.sort_by(|left, right| {
        let a = &raw[*left];
        let b = &raw[*right];
        match sort {
            RepoSort::Updated => b.updated_at().cmp(a.updated_at()),
            RepoSort::Stars => b.stargazers_count.cmp(&a.stargazers_count),
            RepoSort::Name => a
                .display_full_name()
                .to_ascii_lowercase()
                .cmp(&b.display_full_name().to_ascii_lowercase()),
        }
    });
}

fn repos_cache_key(login: &str) -> String {
    format!("repos_{}", key_part(login))
}

fn starred_cache_key(login: &str) -> String {
    format!("starred_{}", key_part(login))
}

fn cycle_ttl(current: u64, direction: i32) -> u64 {
    const STEPS: [u64; 8] = [60, 300, 600, 900, 1800, 3600, 21_600, 86_400];
    let position = STEPS
        .iter()
        .position(|value| *value >= current)
        .unwrap_or(STEPS.len() - 1) as i32;
    let next = (position + direction).clamp(0, STEPS.len() as i32 - 1) as usize;
    STEPS[next]
}

fn page_count(len: usize, size: usize) -> usize {
    len.div_ceil(size.max(1)).max(1)
}

fn page_index(selected: usize, size: usize) -> usize {
    selected / size.max(1)
}

fn page_start(selected: usize, size: usize) -> usize {
    page_index(selected, size) * size.max(1)
}

fn open_external(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = std::process::Command::new("open");
        command.arg(url);
        command
    };
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = std::process::Command::new("cmd");
        command.args(["/C", "start", "", url]);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(url);
        command
    };
    command
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("没能调起浏览器：{error}喵~"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ttl_cycle_clamps() {
        assert_eq!(cycle_ttl(60, -1), 60);
        assert_eq!(cycle_ttl(60, 1), 300);
        assert_eq!(cycle_ttl(300, 1), 600);
        assert_eq!(cycle_ttl(86_400, 1), 86_400);
    }

    #[test]
    fn download_names_are_safe() {
        assert_eq!(safe_download_name("../../evil.tar.gz"), "evil.tar.gz");
        assert_eq!(safe_download_name("a/b/c.zip"), "c.zip");
        assert_eq!(safe_download_name(""), "download");
        assert!(!safe_download_name("a:b?.zip").contains(':'));
    }

    #[test]
    fn unique_download_path_keeps_existing_files() {
        let dir = std::env::temp_dir().join(format!("gitx-download-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let first = unique_download_path(&dir, "app.zip");
        assert_eq!(first.file_name().unwrap(), "app.zip");
        std::fs::write(&first, b"x").unwrap();
        let second = unique_download_path(&dir, "app.zip");
        assert_eq!(second.file_name().unwrap(), "app (1).zip");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn pagination_math_is_stable() {
        assert_eq!(page_count(0, 4), 1);
        assert_eq!(page_count(8, 4), 2);
        assert_eq!(page_count(9, 4), 3);
        assert_eq!(page_index(0, 4), 0);
        assert_eq!(page_index(5, 4), 1);
        assert_eq!(page_start(5, 4), 4);
        assert_eq!(page_start(9, 4), 8);
    }

    #[test]
    fn repo_edit_normalizes_topics() {
        let repo = Repo {
            name: "demo".to_string(),
            owner: crate::model::Owner {
                login: "me".to_string(),
                ..Default::default()
            },
            topics: vec!["Rust".to_string(), "CLI".to_string()],
            ..Default::default()
        };
        let mut edit = RepoEdit::from_repo(&repo);
        edit.topics = TextInput::new("#Rust, cli  TUI");
        assert_eq!(edit.topics_list(), vec!["rust", "cli", "tui"]);
    }

    #[test]
    fn workflow_inputs_must_be_json_object() {
        assert!(parse_workflow_inputs("").unwrap().is_none());
        assert!(parse_workflow_inputs("{}").unwrap().is_none());
        let parsed = parse_workflow_inputs(r#"{"env":"prod","dry_run":"true"}"#)
            .unwrap()
            .unwrap();
        assert_eq!(parsed["env"], "prod");
        assert!(parse_workflow_inputs("[1,2]").is_err());
        assert!(parse_workflow_inputs("env=prod").is_err());
    }

    #[test]
    fn release_filter_matches_tag_and_title() {
        let release = Release {
            tag_name: "v0.1.4".to_string(),
            name: Some("发行版喵".to_string()),
            body: Some("compare v0.1.3...v0.1.4".to_string()),
            ..Default::default()
        };
        assert!(App::release_matches_filter(&release, "0.1.4"));
        assert!(App::release_matches_filter(&release, "发行版"));
        assert!(!App::release_matches_filter(&release, "compare"));
    }

    #[test]
    fn star_sort_orders_by_starred_time() {
        let repos = vec![
            Repo {
                full_name: "old/repo".to_string(),
                starred_at: Some("2024-01-01T00:00:00Z".to_string()),
                ..Default::default()
            },
            Repo {
                full_name: "new/repo".to_string(),
                starred_at: Some("2026-01-01T00:00:00Z".to_string()),
                ..Default::default()
            },
            Repo {
                full_name: "just/starred".to_string(),
                ..Default::default()
            },
        ];
        let mut indices = vec![0, 1, 2];
        sort_star_indices(&repos, &mut indices, StarSort::Starred);
        assert_eq!(indices, vec![2, 1, 0]);
    }

    #[test]
    fn list_filter_matches_repo_fields() {
        let repo = Repo {
            full_name: "me/gitx-tui".to_string(),
            description: Some("终端 GitHub 工作台".to_string()),
            language: Some("Rust".to_string()),
            topics: vec!["tui".to_string(), "git".to_string()],
            ..Default::default()
        };
        assert!(App::repo_matches_filter(&repo, "gitx"));
        assert!(App::repo_matches_filter(&repo, "rust"));
        assert!(App::repo_matches_filter(&repo, "终端"));
        assert!(App::repo_matches_filter(&repo, "tui"));
        assert!(!App::repo_matches_filter(&repo, "python"));
    }

    #[test]
    fn repo_back_navigation_keeps_list_page() {
        let mut return_page = return_page_when_opening(Page::Repositories, Page::Repositories);
        assert_eq!(return_page, Page::Repositories);

        return_page = return_page_when_opening(Page::Repo, return_page);
        assert_eq!(return_page, Page::Repositories);

        return_page = return_page_when_opening(Page::Repo, return_page);
        assert_eq!(return_page, Page::Repositories);

        assert_eq!(
            back_target(Page::Actions, return_page, true),
            Some(Page::Repo)
        );
        assert_eq!(
            back_target(Page::Repo, return_page, true),
            Some(Page::Repositories)
        );
    }

    #[test]
    fn subpage_without_repo_returns_to_origin() {
        assert_eq!(
            return_page_when_opening(Page::Starred, Page::Repositories),
            Page::Starred
        );
        assert_eq!(
            back_target(Page::Issues, Page::Starred, false),
            Some(Page::Starred)
        );
        assert_eq!(back_target(Page::Search, Page::Repositories, false), None);
    }

    #[test]
    fn text_input_edits_unicode() {
        let mut input = TextInput::new("仓库");
        input.insert('!');
        assert_eq!(input.value, "仓库!");
        input.backspace();
        assert_eq!(input.value, "仓库");
        input.home();
        input.delete();
        assert_eq!(input.value, "库");
        input.insert('A');
        assert_eq!(input.value, "A库");
        assert_eq!(input.cursor, 1);
    }
}
