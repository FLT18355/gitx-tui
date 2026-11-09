use crate::api::ApiClient;
use crate::cache::{Cache, CacheStats, now_epoch, search_key};
use crate::config::{Config, Paths};
use crate::format::{self, base64};
use crate::icons;
use crate::model::{
    Branch, Commit, CommitDetail, CompareResult, FileContent, Gist, Issue, IssueComment,
    IssueState, LanguageStat, Notification, Org, PullFile, RateLimit, ReadmeContent, Release, Repo,
    RepoSort, SearchResponse, SearchSort, StarSort, Tag, TreeResponse, TrendingPeriod, User,
    Workflow, WorkflowRun, key_part,
};
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use std::collections::HashSet;
use std::io::Write;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Repositories,
    Starred,
    Search,
    Trending,
    Account,
    Notifications,
    Settings,
    Insights,
    Repo,
    Profile,
    Actions,
    Issues,
    Commits,
    ReleaseDetail,
    Tags,
    IssueDetail,
    Gists,
    Compare,
    Files,
    Network,
}

impl Page {
    pub fn nav_index(self) -> Option<usize> {
        match self {
            Self::Home => Some(0),
            Self::Repositories => Some(1),
            Self::Starred => Some(2),
            Self::Search => Some(3),
            Self::Account => Some(4),
            Self::Notifications => Some(5),
            Self::Trending => Some(6),
            Self::Gists => Some(7),
            Self::Repo
            | Self::Profile
            | Self::Actions
            | Self::Issues
            | Self::Commits
            | Self::ReleaseDetail
            | Self::Tags
            | Self::IssueDetail
            | Self::Compare
            | Self::Files
            | Self::Network
            | Self::Settings
            | Self::Insights => None,
        }
    }

    pub fn from_nav(index: usize) -> Self {
        match index {
            1 => Self::Repositories,
            2 => Self::Starred,
            3 => Self::Search,
            4 => Self::Account,
            5 => Self::Notifications,
            6 => Self::Trending,
            7 => Self::Gists,
            _ => Self::Home,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Home => "首页喵~",
            Self::Repositories => "人家的仓库喵~",
            Self::Starred => "星星收藏喵~",
            Self::Search => "找仓库喵~",
            Self::Trending => "热门仓库喵~",
            Self::Account => "账号喵~",
            Self::Notifications => "通知喵~",
            Self::Settings => "设置喵~",
            Self::Insights => "数据洞察喵~",
            Self::Repo => "仓库详情喵~",
            Self::Profile => "用户主页喵~",
            Self::Actions => "工作流喵~",
            Self::Issues => "议题喵~",
            Self::Commits => "提交喵~",
            Self::ReleaseDetail => "发行版详情喵~",
            Self::Tags => "标签喵~",
            Self::IssueDetail => "议题详情喵~",
            Self::Gists => "Gist 喵~",
            Self::Compare => "分支对比喵~",
            Self::Files => "文件预览喵~",
            Self::Network => "社交网络喵~",
        }
    }

    pub fn is_repo_page(self) -> bool {
        matches!(
            self,
            Self::Repo
                | Self::Actions
                | Self::Issues
                | Self::Commits
                | Self::ReleaseDetail
                | Self::Tags
                | Self::IssueDetail
                | Self::Compare
                | Self::Files
        )
    }

    pub const DEFAULT_PAGES: [Page; 9] = [
        Self::Home,
        Self::Repositories,
        Self::Starred,
        Self::Search,
        Self::Trending,
        Self::Account,
        Self::Notifications,
        Self::Gists,
        Self::Insights,
    ];

    pub fn from_default_name(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "repositories" | "repos" => Self::Repositories,
            "starred" => Self::Starred,
            "search" => Self::Search,
            "trending" => Self::Trending,
            "account" => Self::Account,
            "notifications" => Self::Notifications,
            "gists" => Self::Gists,
            "insights" => Self::Insights,
            _ => Self::Home,
        }
    }

    pub fn default_name(self) -> &'static str {
        match self {
            Self::Repositories => "repositories",
            Self::Starred => "starred",
            Self::Search => "search",
            Self::Trending => "trending",
            Self::Account => "account",
            Self::Notifications => "notifications",
            Self::Gists => "gists",
            Self::Insights => "insights",
            _ => "home",
        }
    }

    pub fn default_label(self) -> &'static str {
        match self {
            Self::Repositories => "仓库喵~",
            Self::Starred => "收藏喵~",
            Self::Search => "搜索喵~",
            Self::Trending => "热门喵~",
            Self::Account => "账号喵~",
            Self::Notifications => "通知喵~",
            Self::Gists => "Gist 喵~",
            Self::Insights => "洞察喵~",
            _ => "首页喵~",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    Help,
    Languages,
    RepoLanguages,
    EditRepo,
    Cache,
    RunWorkflow,
    Branches,
    ForkRepo,
    DeleteRepo,
    NewIssue,
    Palette,
    Themes,
    CancelRun,
    NewGist,
    DeleteGist,
    NewPull,
    MergePr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GistField {
    Description,
    Filename,
    Content,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareSide {
    Base,
    Head,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareView {
    Commits,
    Files,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkTab {
    Followers,
    Following,
    Orgs,
}

impl NetworkTab {
    pub fn label(self) -> &'static str {
        match self {
            Self::Followers => "粉丝喵~",
            Self::Following => "关注喵~",
            Self::Orgs => "组织喵~",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Followers => Self::Following,
            Self::Following => Self::Orgs,
            Self::Orgs => Self::Followers,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullField {
    Title,
    Body,
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
    SetTrendingPeriod(TrendingPeriod),
    ToggleReadmeFullscreen,
    SearchFocus,
    ListFilterFocus,
    ReleaseFilterFocus,
    SearchHistory(String),
    SubmitSearch,
    OpenPage,
    Back,
    Refresh,
    OpenCurrent,
    RepoActionsLeft,
    RepoActionsRight,
    CopyClone,
    SetTheme(String),
    CycleTheme,
    OpenThemes,
    OpenPalette,
    OpenHelp,
    RandomRepo,
    TogglePin,
    OpenPin(usize),
    NavLeft,
    NavRight,
    StartProxyEdit,
    StartDownloadDirEdit,
    ToggleFileTree,
    ToggleReadme,
    Ttl(TtlKind, i32),
    ClearCache,
    SaveConfig,
    OpenDownloadDir,
    SetSettingsRow(usize),
    CommitLimit(i32),
    OpenLanguages,
    DownloadAsset { release: usize, asset: usize },
    DownloadSource,
    OpenReleasePage(usize),
    OpenReleaseDetail(usize),
    OpenTreePath { path: String, directory: bool },
    FocusRepoPane(RepoPane),
    OpenExternal(String),
    OpenProfile(String),
    ForkRepo,
    ConfirmFork,
    DeleteRepo,
    ConfirmDeleteRepo,
    ToggleStar,
    CloseOverlay,
    PrevListPage,
    NextListPage,
    OpenRepoEdit,
    EditRepoRow(usize),
    SaveRepoEdit,
    RefreshRateLimit,
    OpenActions,
    OpenBranches,
    SelectBranch(String),
    OpenCommits,
    SelectActionRun(usize),
    OpenCacheManager,
    ClearCacheGroup(usize),
    SelectCacheRow(usize),
    OpenCacheDir,
    OpenRepoLanguages,
    OpenTags,
    OpenTagPage,
    SelectTag(usize),
    CopyTagName,
    DownloadTagZip,
    DownloadTagTarball,
    CopySsh,
    CopyFullName,
    CopyAssetUrl { asset: usize },
    FocusIssueFilter,
    FocusCommitFilter,
    RerunActionRun,
    CancelActionRun,
    ConfirmCancelRun,
    StartNewIssue,
    FocusNewIssue(bool),
    SubmitNewIssue,
    OpenWorkflowRunner,
    SelectWorkflow(usize),
    EditWorkflowRef,
    EditWorkflowInputs,
    RunWorkflow,
    RefreshWorkflows,
    OpenIssues,
    OpenPullRequests,
    SelectIssue(usize),
    SelectCommit(usize),
    SetIssueKind(IssueKind),
    SetIssueState(IssueState),
    SelectNotification(usize),
    SetNotificationFilter(bool),
    MarkNotificationRead,
    MarkAllNotificationsRead,
    ToggleIssueState,
    FocusIssueComment,
    SubmitIssueComment,
    MergePull,
    ConfirmMergePull,
    OpenNetwork,
    SelectGist(usize),
    SelectGistFile(usize),
    StartNewGist,
    DeleteGist,
    ConfirmDeleteGist,
    ToggleGistPublic,
    FocusGistField(GistField),
    SubmitNewGist,
    CopyGistUrl,
    RefreshGists,
    SelectCompareRow(usize),
    PickCompareBranch(CompareSide),
    RefreshCompare,
    ToggleCompareView,
    StartNewPull,
    ConfirmNewPull,
    TogglePullDraft,
    FocusPullField(PullField),
    SelectNetwork(usize),
    SetNetworkTab(NetworkTab),
    ToggleFollow,
    ToggleFileWrap,
    CopyFilePath,
    DownloadFile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitBox {
    pub rect: Rect,
    pub action: Action,
}

#[derive(Debug, Clone)]
pub struct PaletteItem {
    pub label: String,
    pub hint: String,
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

struct PendingCommitDetail {
    repo: String,
    sha: String,
    private: bool,
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
            prefixes: &[
                "repo_",
                "releases_",
                "tags_",
                "tree_",
                "readme_",
                "languages_",
                "commit_",
            ],
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
    pub name: TextInput,
    pub description: TextInput,
    pub homepage: TextInput,
    pub topics: TextInput,
    pub private: bool,
    pub archived: bool,
    pub has_issues: bool,
    pub has_discussions: bool,
    pub has_wiki: bool,
    pub row: usize,
    pub editing: bool,
    pub saving: bool,
    pub error: Option<String>,
}

impl RepoEdit {
    pub fn from_repo(repo: &Repo) -> Self {
        Self {
            full_name: repo.display_full_name(),
            name: TextInput::new(repo.name.clone()),
            description: TextInput::new(repo.description.clone().unwrap_or_default()),
            homepage: TextInput::new(repo.homepage.clone().unwrap_or_default()),
            topics: TextInput::new(repo.topics.join(" ")),
            private: repo.private,
            archived: repo.archived,
            has_issues: repo.has_issues,
            has_discussions: repo.has_discussions,
            has_wiki: repo.has_wiki,
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

fn repo_edit_input(edit: &mut RepoEdit, row: usize) -> Option<&mut TextInput> {
    match row {
        0 => Some(&mut edit.name),
        1 => Some(&mut edit.description),
        2 => Some(&mut edit.homepage),
        3 => Some(&mut edit.topics),
        _ => None,
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
    Trending {
        period: TrendingPeriod,
        result: Result<crate::model::SearchOutcome, String>,
    },
    Commits {
        name: String,
        branch: String,
        result: Result<Vec<Commit>, String>,
    },
    Branches {
        name: String,
        result: Result<Vec<Branch>, String>,
    },
    Detail {
        name: String,
        result: Result<Repo, String>,
    },
    Releases {
        name: String,
        result: Result<Vec<Release>, String>,
    },
    Tags {
        name: String,
        result: Result<Vec<Tag>, String>,
    },
    Tree {
        name: String,
        branch: String,
        result: Result<TreeResponse, String>,
    },
    Readme {
        name: String,
        branch: String,
        result: Result<ReadmeContent, String>,
    },
    StarToggled {
        name: String,
        starred: bool,
        result: Result<(), String>,
    },
    ProfileUser {
        login: String,
        result: Result<User, String>,
    },
    ProfileRepos {
        login: String,
        result: Result<Vec<Repo>, String>,
    },
    RepoForked {
        name: String,
        result: Result<Repo, String>,
    },
    RepoDeleted {
        name: String,
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
    ActionRunRerun {
        name: String,
        result: Result<(), String>,
    },
    ActionRunCancelled {
        name: String,
        result: Result<(), String>,
    },
    Issues {
        name: String,
        state: IssueState,
        result: Result<Vec<Issue>, String>,
    },
    IssueCreated {
        name: String,
        result: Result<Issue, String>,
    },
    RepoLanguages {
        name: String,
        result: Result<Vec<LanguageStat>, String>,
    },
    CommitDetail {
        repo: String,
        sha: String,
        result: Result<CommitDetail, String>,
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
    PinnedRepo {
        name: String,
        result: Result<Repo, String>,
    },
    IssueDetail {
        name: String,
        number: u64,
        result: Result<Issue, String>,
    },
    IssueComments {
        name: String,
        number: u64,
        result: Result<Vec<crate::model::IssueComment>, String>,
    },
    IssueCommentCreated {
        name: String,
        number: u64,
        result: Result<crate::model::IssueComment, String>,
    },
    IssueStateChanged {
        name: String,
        number: u64,
        result: Result<Issue, String>,
    },
    PullFiles {
        name: String,
        number: u64,
        result: Result<Vec<crate::model::PullFile>, String>,
    },
    PullMerged {
        name: String,
        number: u64,
        result: Result<String, String>,
    },
    Gists {
        result: Result<Vec<crate::model::Gist>, String>,
    },
    GistDetail {
        id: String,
        result: Result<crate::model::Gist, String>,
    },
    GistCreated {
        result: Result<crate::model::Gist, String>,
    },
    GistDeleted {
        id: String,
        result: Result<(), String>,
    },
    Compare {
        name: String,
        base: String,
        head: String,
        result: Result<crate::model::CompareResult, String>,
    },
    PullCreated {
        name: String,
        result: Result<crate::model::CreatedPull, String>,
    },
    FileContent {
        name: String,
        path: String,
        branch: String,
        result: Result<crate::model::FileContent, String>,
    },
    NetworkUsers {
        login: String,
        tab: NetworkTab,
        result: Result<Vec<User>, String>,
    },
    NetworkUserDetails {
        login: String,
        tab: NetworkTab,
        result: Result<Vec<User>, String>,
    },
    NetworkOrgs {
        login: String,
        result: Result<Vec<crate::model::Org>, String>,
    },
    ProfileFollowChecked {
        login: String,
        result: Result<bool, String>,
    },
    ProfileFollowToggled {
        login: String,
        following: bool,
        result: Result<(), String>,
    },
    FollowChecked {
        login: String,
        result: Result<bool, String>,
    },
    FollowToggled {
        login: String,
        following: bool,
        result: Result<(), String>,
    },
}

pub struct App {
    pub paths: Paths,
    pub config: Config,
    pub theme: Theme,
    pub themes: Vec<String>,
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
    pub new_issue_title: TextInput,
    pub new_issue_body: TextInput,
    pub new_issue_body_focus: bool,
    pub new_issue_saving: bool,
    pub new_issue_error: Option<String>,
    pub issue_open: Option<Issue>,
    pub issue_open_number: Option<u64>,
    pub issue_comments: Vec<IssueComment>,
    pub issue_comments_loading: Option<u64>,
    pub issue_detail_loading: Option<u64>,
    pub issue_scroll: u16,
    pub issue_scroll_max: u16,
    pub issue_comment_input: TextInput,
    pub issue_comment_focus: bool,
    pub issue_comment_saving: bool,
    pub issue_action_loading: bool,
    pub issue_pr_files: Vec<PullFile>,
    pub issue_pr_files_loading: Option<u64>,
    pub issue_pr_files_scroll: u16,
    pub gists: Vec<Gist>,
    pub gists_loading: bool,
    pub gists_fetched_at: Option<i64>,
    pub gist_selected: usize,
    pub gist_error: Option<String>,
    pub gist_detail: Option<Gist>,
    pub gist_detail_for: Option<String>,
    pub gist_detail_loading: Option<String>,
    pub gist_file_selected: usize,
    pub gist_scroll: u16,
    pub gist_scroll_max: u16,
    pub gist_desc_input: TextInput,
    pub gist_filename_input: TextInput,
    pub gist_content_input: TextInput,
    pub gist_field: GistField,
    pub gist_public: bool,
    pub gist_saving: bool,
    pub gist_create_error: Option<String>,
    pub gist_delete: Option<Gist>,
    pub gist_deleting: bool,
    pub compare_repo: Option<Repo>,
    pub compare_base: String,
    pub compare_head: String,
    pub compare_focus: CompareSide,
    pub compare_result: Option<CompareResult>,
    pub compare_loading: bool,
    pub compare_error: Option<String>,
    pub compare_view: CompareView,
    pub compare_selected: usize,
    pub compare_scroll: u16,
    pub compare_scroll_max: u16,
    pub compare_fetched_for: Option<(String, String, String)>,
    pub branch_pick_side: Option<CompareSide>,
    pub pull_created: Option<String>,
    pub pull_title: TextInput,
    pub pull_body: TextInput,
    pub pull_draft: bool,
    pub pull_field: PullField,
    pub pull_saving: bool,
    pub pull_error: Option<String>,
    pub file_repo: Option<Repo>,
    pub file_path: String,
    pub file_branch: String,
    pub file_view: Option<FileContent>,
    pub file_loading: Option<String>,
    pub file_error: Option<String>,
    pub file_wrap: bool,
    pub file_scroll: u16,
    pub file_scroll_max: u16,
    pub file_h_scroll: u16,
    pub file_h_max: u16,
    pub network_login: Option<String>,
    pub network_tab: NetworkTab,
    pub network_users: Vec<User>,
    pub network_orgs: Vec<Org>,
    pub network_loading: Option<NetworkTab>,
    pub network_selected: usize,
    pub network_following: Option<bool>,
    pub network_follow_loading: Option<String>,
    pub network_known_following: HashSet<String>,
    pub network_details: HashSet<String>,
    pub profile_login: Option<String>,
    pub profile_user: Option<User>,
    pub profile_following: Option<bool>,
    pub profile_follow_loading: Option<String>,
    pub profile_user_loading: bool,
    pub profile_repos: Vec<Repo>,
    pub profile_repos_loading: bool,
    pub profile_error: Option<String>,
    profile_selected: usize,
    profile_return_page: Page,
    pub fork_repo: Option<Repo>,
    pub fork_loading: Option<String>,
    pub delete_repo: Option<Repo>,
    pub delete_loading: Option<String>,
    pub cache_row: usize,
    pub page: Page,
    pub return_page: Page,
    pub overlay: Option<Overlay>,
    pub palette_input: TextInput,
    pub palette_selected: usize,
    pub theme_selected: usize,
    pub home_selected: usize,
    pub nav_offset: usize,
    pub pin_cache: Vec<Repo>,
    pub pin_loading: Vec<String>,
    pub should_quit: bool,
    pub settings_only: bool,

    pub repos: Vec<Repo>,
    pub starred: Vec<Repo>,
    pub search_results: Vec<Repo>,
    pub search_total: Option<u64>,
    pub search_submitted: Option<String>,
    pub search_input: TextInput,
    pub search_filter: TextInput,
    pub search_focus: bool,
    pub trending: Vec<Repo>,
    pub trending_total: Option<u64>,
    pub trending_filter: TextInput,
    pub trending_period: TrendingPeriod,
    trending_selected: usize,
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
    pub repo_actions_offset: usize,
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
    pub readme_branch: String,
    pub readme_fetched_at: Option<i64>,
    pub readme_loading: Option<String>,
    pub readme_scroll: u16,
    pub readme_scroll_max: u16,
    pub readme_fullscreen: bool,

    pub branches: Vec<Branch>,
    pub branches_loading: Option<String>,
    pub branches_fetched_at: Option<i64>,
    pub branch_selected: usize,
    pub branch_offset: usize,
    pub branch_viewport: usize,

    pub commits: Vec<Commit>,
    pub commits_loading: Option<(String, String)>,
    pub commits_fetched_at: Option<i64>,
    pub commit_repo: Option<Repo>,
    pub commit_branch: String,
    pub commit_selected: usize,
    pub commit_scroll: u16,
    pub commit_detail: Option<CommitDetail>,
    pub commit_detail_for: Option<String>,
    pub commit_detail_loading: Option<String>,
    pending_commit_detail: Option<PendingCommitDetail>,

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
    pub repo_languages: Vec<LanguageStat>,
    pub repo_languages_loading: Option<String>,
    pub repo_languages_fetched_at: Option<i64>,
    pub repo_language_offset: usize,
    pub repo_language_viewport: usize,

    pub user_loading: bool,
    pub repos_loading: bool,
    pub starred_loading: bool,
    pub search_loading: bool,
    pub trending_loading: bool,
    pub detail_loading: Option<String>,
    pub releases_loading: Option<String>,
    pub tags: Vec<Tag>,
    pub tags_loading: Option<String>,
    pub tags_fetched_at: Option<i64>,
    pub tag_repo: Option<Repo>,
    pub tag_selected: usize,
    pub tag_scroll: u16,
    pub issue_filter: TextInput,
    pub issue_filter_focus: bool,
    pub commit_filter: TextInput,
    pub commit_filter_focus: bool,
    pub cancel_run: Option<WorkflowRun>,
    pub cancel_loading: bool,

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
            .filter(|value| Theme::exists(&paths, value))
            .unwrap_or_else(|| config.theme.clone());
        let theme = Theme::load(&paths, &theme_name);
        let themes = Theme::available(&paths);
        let client = if settings_only {
            ApiClient::without_auth(config.proxy.clone())
        } else {
            ApiClient::new(config.proxy.clone())
        };
        let cache = Cache::new(paths.cache_dir().to_path_buf());
        let cache_stats = cache.stats();
        let download_dir_initial = config.download_dir.clone();
        let start_page = if settings_only {
            Page::Settings
        } else {
            Page::from_default_name(&config.default_page)
        };
        let (tx, rx) = mpsc::channel();
        let mut app = Self {
            paths,
            config,
            theme,
            themes,
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
            new_issue_title: TextInput::new(""),
            new_issue_body: TextInput::new(""),
            new_issue_body_focus: false,
            new_issue_saving: false,
            new_issue_error: None,
            issue_open: None,
            issue_open_number: None,
            issue_comments: Vec::new(),
            issue_comments_loading: None,
            issue_detail_loading: None,
            issue_scroll: 0,
            issue_scroll_max: 0,
            issue_comment_input: TextInput::new(""),
            issue_comment_focus: false,
            issue_comment_saving: false,
            issue_action_loading: false,
            issue_pr_files: Vec::new(),
            issue_pr_files_loading: None,
            issue_pr_files_scroll: 0,
            gists: Vec::new(),
            gists_loading: false,
            gists_fetched_at: None,
            gist_selected: 0,
            gist_error: None,
            gist_detail: None,
            gist_detail_for: None,
            gist_detail_loading: None,
            gist_file_selected: 0,
            gist_scroll: 0,
            gist_scroll_max: 0,
            gist_desc_input: TextInput::new(""),
            gist_filename_input: TextInput::new("gistfile1.txt"),
            gist_content_input: TextInput::new(""),
            gist_field: GistField::Description,
            gist_public: false,
            gist_saving: false,
            gist_create_error: None,
            gist_delete: None,
            gist_deleting: false,
            compare_repo: None,
            compare_base: String::new(),
            compare_head: String::new(),
            compare_focus: CompareSide::Base,
            compare_result: None,
            compare_loading: false,
            compare_error: None,
            compare_view: CompareView::Commits,
            compare_selected: 0,
            compare_scroll: 0,
            compare_scroll_max: 0,
            compare_fetched_for: None,
            branch_pick_side: None,
            pull_created: None,
            pull_title: TextInput::new(""),
            pull_body: TextInput::new(""),
            pull_draft: false,
            pull_field: PullField::Title,
            pull_saving: false,
            pull_error: None,
            file_repo: None,
            file_path: String::new(),
            file_branch: String::new(),
            file_view: None,
            file_loading: None,
            file_error: None,
            file_wrap: false,
            file_scroll: 0,
            file_scroll_max: 0,
            file_h_scroll: 0,
            file_h_max: 0,
            network_login: None,
            network_tab: NetworkTab::Followers,
            network_users: Vec::new(),
            network_orgs: Vec::new(),
            network_loading: None,
            network_selected: 0,
            network_following: None,
            network_follow_loading: None,
            network_known_following: HashSet::new(),
            network_details: HashSet::new(),
            profile_login: None,
            profile_user: None,
            profile_following: None,
            profile_follow_loading: None,
            profile_user_loading: false,
            profile_repos: Vec::new(),
            profile_repos_loading: false,
            profile_error: None,
            profile_selected: 0,
            profile_return_page: Page::Repositories,
            fork_repo: None,
            fork_loading: None,
            delete_repo: None,
            delete_loading: None,
            cache_row: 0,
            page: start_page,
            return_page: start_page,
            overlay: None,
            palette_input: TextInput::new(""),
            palette_selected: 0,
            theme_selected: 0,
            home_selected: 0,
            nav_offset: start_page.nav_index().unwrap_or(0),
            pin_cache: Vec::new(),
            pin_loading: Vec::new(),
            should_quit: false,
            settings_only,
            repos: Vec::new(),
            starred: Vec::new(),
            search_results: Vec::new(),
            search_total: None,
            search_submitted: None,
            search_input: TextInput::new(""),
            search_filter: TextInput::new(""),
            search_focus: start_page == Page::Search,
            trending: Vec::new(),
            trending_total: None,
            trending_filter: TextInput::new(""),
            trending_period: TrendingPeriod::Weekly,
            trending_selected: 0,
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
            repo_actions_offset: 0,
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
            readme_branch: String::new(),
            readme_fetched_at: None,
            readme_loading: None,
            readme_scroll: 0,
            readme_scroll_max: 0,
            readme_fullscreen: false,
            branches: Vec::new(),
            branches_loading: None,
            branches_fetched_at: None,
            branch_selected: 0,
            branch_offset: 0,
            branch_viewport: 12,
            commits: Vec::new(),
            commits_loading: None,
            commits_fetched_at: None,
            commit_repo: None,
            commit_branch: String::new(),
            commit_selected: 0,
            commit_scroll: 0,
            commit_detail: None,
            commit_detail_for: None,
            commit_detail_loading: None,
            pending_commit_detail: None,
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
            repo_languages: Vec::new(),
            repo_languages_loading: None,
            repo_languages_fetched_at: None,
            repo_language_offset: 0,
            repo_language_viewport: 12,
            user_loading: false,
            repos_loading: false,
            starred_loading: false,
            search_loading: false,
            trending_loading: false,
            detail_loading: None,
            releases_loading: None,
            tags: Vec::new(),
            tags_loading: None,
            tags_fetched_at: None,
            tag_repo: None,
            tag_selected: 0,
            tag_scroll: 0,
            issue_filter: TextInput::new(""),
            issue_filter_focus: false,
            commit_filter: TextInput::new(""),
            commit_filter_focus: false,
            cancel_run: None,
            cancel_loading: false,
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

        self.load_notifications_cache();
        self.fetch_notifications(false);
        if self.page == Page::Home {
            self.sync_pins();
            self.fetch_repos_if_needed(false);
            self.fetch_starred_if_needed(false);
        }
        if self.page == Page::Insights {
            self.fetch_rate_limit(false);
        }
        if self.page == Page::Trending {
            self.fetch_trending(false);
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
            Msg::Trending { period, result } => {
                self.trending_loading = false;
                if self.trending_period != period {
                    if self.page == Page::Trending {
                        self.fetch_trending(false);
                    }
                    return;
                }
                match result {
                    Ok(outcome) => {
                        self.trending = outcome.items;
                        self.trending_total = Some(outcome.total);
                        self.trending_selected = 0;
                        if self.page == Page::Trending {
                            self.list_offset = 0;
                        }
                        let response = SearchResponse {
                            total_count: outcome.total,
                            incomplete_results: false,
                            items: self.trending.clone(),
                        };
                        let _ = self.cache.save(&trending_key(period), &response);
                        self.refresh_cache_stats();
                        if self.page == Page::Trending {
                            self.schedule_selected_detail();
                        }
                        if self.trending.is_empty() {
                            self.set_status(
                                StatusKind::Info,
                                "这段时间没有热门仓库喵~换个时段看看嘛~",
                            );
                        } else {
                            self.set_status(
                                StatusKind::Ok,
                                &format!(
                                    "{}的热门仓库前 100 个已经摆好啦喵~",
                                    period.short_label()
                                ),
                            );
                        }
                    }
                    Err(error) => self.set_status(StatusKind::Error, &error),
                }
            }
            Msg::Commits {
                name,
                branch,
                result,
            } => {
                if self.commits_loading.as_ref() == Some(&(name.clone(), branch.clone())) {
                    self.commits_loading = None;
                }
                let current = self.commit_repo.as_ref().map(Repo::display_full_name);
                if current.as_deref() != Some(name.as_str()) || self.commit_branch != branch {
                    return;
                }
                match result {
                    Ok(commits) => {
                        let _ = self
                            .cache
                            .save(&commits_cache_key(&name, &branch), &commits);
                        self.commits = commits;
                        self.commits_fetched_at = Some(now_epoch());
                        self.clamp_commit_selection();
                        self.commit_scroll = 0;
                        self.refresh_cache_stats();
                        if self.page == Page::Commits {
                            self.schedule_commit_detail(false);
                            self.set_status(
                                StatusKind::Ok,
                                &format!("{} 个提交已经摆好啦喵~", self.commits.len()),
                            );
                        }
                    }
                    Err(error) => {
                        if self.page == Page::Commits {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
                }
            }
            Msg::Branches { name, result } => {
                if self.branches_loading.as_deref() == Some(name.as_str()) {
                    self.branches_loading = None;
                }
                match result {
                    Ok(branches) => {
                        let _ = self.cache.save(&branches_cache_key(&name), &branches);
                        self.branches = branches;
                        self.branches_fetched_at = Some(now_epoch());
                        self.clamp_branch_selection();
                        self.refresh_cache_stats();
                    }
                    Err(error) => {
                        if self
                            .page_repo
                            .as_ref()
                            .map(Repo::display_full_name)
                            .as_deref()
                            == Some(name.as_str())
                        {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
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
            Msg::Tags { name, result } => {
                if self.tags_loading.as_deref() == Some(name.as_str()) {
                    self.tags_loading = None;
                }
                if self
                    .tag_repo
                    .as_ref()
                    .map(|repo| repo.display_full_name())
                    .as_deref()
                    != Some(name.as_str())
                {
                    return;
                }
                match result {
                    Ok(tags) => {
                        if let Some(repo) = self.tag_repo.clone() {
                            let _ = self.cache.save(&repo.tags_cache_key(), &tags);
                        }
                        self.tags = tags;
                        self.tags_fetched_at = Some(now_epoch());
                        self.clamp_tag_selection();
                        self.refresh_cache_stats();
                        if self.page == Page::Tags {
                            self.set_status(
                                StatusKind::Ok,
                                &format!("标签都拿来啦，一共 {} 个喵~", self.tags.len()),
                            );
                        }
                    }
                    Err(error) => {
                        if self.page == Page::Tags {
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
                        if self.matches_current_repo(&name) && branch == self.tree_branch {
                            self.tree = Some(tree);
                            self.tree_for = Some(name);
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
            Msg::Readme {
                name,
                branch,
                result,
            } => {
                if self.readme_loading.as_deref() == Some(name.as_str()) {
                    self.readme_loading = None;
                }
                match result {
                    Ok(readme) => {
                        let _ = self.cache.save(&readme_cache_key(&name, &branch), &readme);
                        self.refresh_cache_stats();
                        if self.matches_current_repo(&name) && branch == self.readme_branch {
                            self.readme = Some(readme);
                            self.readme_for = Some(name);
                            self.readme_fetched_at = Some(now_epoch());
                        }
                    }
                    Err(error) => {
                        if self.matches_current_repo(&name) && branch == self.readme_branch {
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
            Msg::ProfileUser { login, result } => {
                if self.profile_login.as_deref() != Some(login.as_str()) {
                    return;
                }
                self.profile_user_loading = false;
                match result {
                    Ok(user) => {
                        self.profile_user = Some(user);
                        self.profile_error = None;
                    }
                    Err(error) => {
                        self.profile_error = Some(error.clone());
                        self.set_status(StatusKind::Error, &error);
                    }
                }
            }
            Msg::ProfileRepos { login, result } => {
                if self.profile_login.as_deref() != Some(login.as_str()) {
                    return;
                }
                self.profile_repos_loading = false;
                match result {
                    Ok(repos) => {
                        self.profile_repos = repos;
                        self.clamp_selection();
                        self.follow_selection();
                    }
                    Err(error) => {
                        self.profile_error = Some(error.clone());
                        self.set_status(StatusKind::Error, &error);
                    }
                }
            }
            Msg::RepoForked { name, result } => {
                if self.fork_loading.as_deref() == Some(name.as_str()) {
                    self.fork_loading = None;
                }
                match result {
                    Ok(repo) => {
                        self.overlay = None;
                        self.fork_repo = None;
                        let own = self
                            .login
                            .as_deref()
                            .is_some_and(|login| login.eq_ignore_ascii_case(repo.owner_name()));
                        if own {
                            if let Some(existing) = self.repos.iter_mut().find(|item| {
                                item.id == repo.id
                                    || item.display_full_name() == repo.display_full_name()
                            }) {
                                *existing = repo.clone();
                            } else {
                                self.repos.insert(0, repo.clone());
                            }
                            if let Some(login) = self.login.clone() {
                                let _ = self.cache.save(&repos_cache_key(&login), &self.repos);
                            }
                            let _ = self.cache.save(&repo.cache_key(), &repo);
                            self.refresh_cache_stats();
                        }
                        self.set_status(
                            StatusKind::Ok,
                            &format!(
                                "Fork 好啦：{} 已经放进你的仓库列表喵~",
                                repo.display_full_name()
                            ),
                        );
                    }
                    Err(error) => self.set_status(StatusKind::Error, &error),
                }
            }
            Msg::RepoDeleted { name, result } => {
                if self.delete_loading.as_deref() == Some(name.as_str()) {
                    self.delete_loading = None;
                }
                match result {
                    Ok(()) => {
                        self.overlay = None;
                        self.delete_repo = None;
                        self.finish_deleted_repo(&name);
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
            Msg::ActionRunRerun { name, result } => {
                if self.action_repo.as_deref() != Some(name.as_str()) {
                    return;
                }
                match result {
                    Ok(()) => {
                        if self.page == Page::Actions {
                            self.set_status(StatusKind::Ok, "重新跑起来啦喵~");
                        }
                        self.fetch_action_runs(true);
                    }
                    Err(error) => self.set_status(StatusKind::Error, &error),
                }
            }
            Msg::ActionRunCancelled { name, result } => {
                if self.action_repo.as_deref() != Some(name.as_str()) {
                    return;
                }
                self.cancel_loading = false;
                match result {
                    Ok(()) => {
                        self.set_status(StatusKind::Ok, "这次运行已经取消啦喵~");
                        self.cancel_run = None;
                        self.overlay = None;
                        self.fetch_action_runs(true);
                    }
                    Err(error) => self.set_status(StatusKind::Error, &error),
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
            Msg::IssueCreated { name, result } => {
                self.new_issue_saving = false;
                let current = self
                    .issue_repo
                    .as_ref()
                    .map(|repo| repo.display_full_name());
                if current.as_deref() != Some(name.as_str()) {
                    return;
                }
                match result {
                    Ok(issue) => {
                        self.overlay = None;
                        self.new_issue_title = TextInput::new("");
                        self.new_issue_body = TextInput::new("");
                        self.new_issue_error = None;
                        if self.issue_state != IssueState::Open {
                            self.issue_state = IssueState::Open;
                            self.issues.clear();
                            self.issues_fetched_at = None;
                            self.issue_selected = 0;
                        }
                        if self.issue_kind == IssueKind::PullRequests {
                            self.issue_kind = IssueKind::All;
                        }
                        if self.issue_state == IssueState::Open
                            && self.issue_kind.matches(&issue)
                            && !self.issues.iter().any(|item| item.id == issue.id)
                        {
                            self.issues.insert(0, issue.clone());
                            let _ = self
                                .cache
                                .save(&issues_cache_key(&name, IssueState::Open), &self.issues);
                            self.clamp_issue_selection();
                        }
                        self.set_status(
                            StatusKind::Ok,
                            &format!("议题 #{} 建好啦喵~", issue.number),
                        );
                        self.fetch_issues(true);
                    }
                    Err(error) => {
                        self.new_issue_error = Some(error.clone());
                        self.set_status(StatusKind::Error, &error);
                    }
                }
            }
            Msg::RepoLanguages { name, result } => {
                if self.repo_languages_loading.as_deref() == Some(name.as_str()) {
                    self.repo_languages_loading = None;
                }
                let current = self.context_repo().map(|repo| repo.display_full_name());
                if current.as_deref() != Some(name.as_str()) {
                    return;
                }
                match result {
                    Ok(languages) => {
                        let _ = self.cache.save(&languages_cache_key(&name), &languages);
                        self.repo_languages = languages;
                        self.repo_languages_fetched_at = Some(now_epoch());
                        self.repo_language_offset = 0;
                        self.refresh_cache_stats();
                    }
                    Err(error) => {
                        if self.overlay == Some(Overlay::RepoLanguages) {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
                }
            }
            Msg::CommitDetail { repo, sha, result } => {
                if self.commit_detail_loading.as_deref() == Some(sha.as_str()) {
                    self.commit_detail_loading = None;
                }
                let current_repo = self.commit_repo.as_ref().map(Repo::display_full_name);
                let current_sha = self.selected_commit().map(|commit| commit.sha);
                if current_repo.as_deref() != Some(repo.as_str())
                    || current_sha.as_deref() != Some(sha.as_str())
                {
                    return;
                }
                match result {
                    Ok(detail) => {
                        let _ = self
                            .cache
                            .save(&commit_detail_cache_key(&repo, &sha), &detail);
                        self.commit_detail = Some(detail);
                        self.commit_detail_for = Some(sha);
                        self.refresh_cache_stats();
                    }
                    Err(error) => {
                        if self.commit_detail_for.as_deref() != Some(sha.as_str()) {
                            self.commit_detail = None;
                            self.commit_detail_for = None;
                        }
                        if self.page == Page::Commits {
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
            Msg::PinnedRepo { name, result } => {
                self.pin_loading
                    .retain(|item| !item.eq_ignore_ascii_case(&name));
                if !self
                    .config
                    .pinned_repos
                    .iter()
                    .any(|item| item.eq_ignore_ascii_case(&name))
                {
                    return;
                }
                match result {
                    Ok(repo) => {
                        if !self
                            .pin_cache
                            .iter()
                            .any(|item| item.display_full_name().eq_ignore_ascii_case(&name))
                        {
                            self.pin_cache.push(repo);
                        }
                    }
                    Err(error) => {
                        if self.page == Page::Home {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
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
            Msg::IssueDetail {
                name,
                number,
                result,
            } => {
                if self.issue_detail_loading == Some(number) {
                    self.issue_detail_loading = None;
                }
                let current = self
                    .issue_repo
                    .as_ref()
                    .map(|repo| repo.display_full_name());
                if current.as_deref() != Some(name.as_str())
                    || self.issue_open_number != Some(number)
                {
                    return;
                }
                match result {
                    Ok(issue) => {
                        self.issue_open = Some(issue);
                        self.issue_action_loading = false;
                    }
                    Err(error) => {
                        self.issue_action_loading = false;
                        if self.page == Page::IssueDetail {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
                }
            }
            Msg::IssueComments {
                name,
                number,
                result,
            } => {
                if self.issue_comments_loading == Some(number) {
                    self.issue_comments_loading = None;
                }
                let current = self
                    .issue_repo
                    .as_ref()
                    .map(|repo| repo.display_full_name());
                if current.as_deref() != Some(name.as_str())
                    || self.issue_open_number != Some(number)
                {
                    return;
                }
                match result {
                    Ok(comments) => {
                        self.issue_comments = comments;
                    }
                    Err(error) => {
                        if self.page == Page::IssueDetail {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
                }
            }
            Msg::IssueCommentCreated {
                name,
                number,
                result,
            } => {
                self.issue_comment_saving = false;
                let current = self
                    .issue_repo
                    .as_ref()
                    .map(|repo| repo.display_full_name());
                if current.as_deref() != Some(name.as_str())
                    || self.issue_open_number != Some(number)
                {
                    return;
                }
                match result {
                    Ok(comment) => {
                        self.issue_comments.push(comment);
                        self.issue_comment_input = TextInput::new("");
                        self.issue_comment_focus = false;
                        if let Some(issue) = self.issue_open.as_mut() {
                            issue.comments = issue.comments.saturating_add(1);
                        }
                        self.issue_scroll = u16::MAX;
                        self.set_status(StatusKind::Ok, "评论发出去啦喵~");
                    }
                    Err(error) => self.set_status(StatusKind::Error, &error),
                }
            }
            Msg::IssueStateChanged {
                name,
                number,
                result,
            } => {
                self.issue_action_loading = false;
                let current = self
                    .issue_repo
                    .as_ref()
                    .map(|repo| repo.display_full_name());
                if current.as_deref() != Some(name.as_str())
                    || self.issue_open_number != Some(number)
                {
                    return;
                }
                match result {
                    Ok(issue) => {
                        if let Some(item) =
                            self.issues.iter_mut().find(|item| item.number == number)
                        {
                            *item = issue.clone();
                        }
                        self.set_status(
                            StatusKind::Ok,
                            &format!("议题 #{} 现在是「{}」喵~", number, issue.state_label()),
                        );
                        self.issue_open = Some(issue);
                        self.fetch_issues(true);
                    }
                    Err(error) => self.set_status(StatusKind::Error, &error),
                }
            }
            Msg::PullFiles {
                name,
                number,
                result,
            } => {
                if self.issue_pr_files_loading == Some(number) {
                    self.issue_pr_files_loading = None;
                }
                let current = self
                    .issue_repo
                    .as_ref()
                    .map(|repo| repo.display_full_name());
                if current.as_deref() != Some(name.as_str())
                    || self.issue_open_number != Some(number)
                {
                    return;
                }
                match result {
                    Ok(files) => {
                        self.issue_pr_files = files;
                        self.issue_pr_files_scroll = 0;
                    }
                    Err(error) => {
                        if self.page == Page::IssueDetail {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
                }
            }
            Msg::PullMerged {
                name,
                number,
                result,
            } => {
                self.issue_action_loading = false;
                let current = self
                    .issue_repo
                    .as_ref()
                    .map(|repo| repo.display_full_name());
                if current.as_deref() != Some(name.as_str())
                    || self.issue_open_number != Some(number)
                {
                    return;
                }
                match result {
                    Ok(message) => {
                        self.set_status(StatusKind::Ok, &format!("{message} 喵~"));
                        self.fetch_issue_detail();
                        self.fetch_pull_files();
                    }
                    Err(error) => self.set_status(StatusKind::Error, &error),
                }
            }
            Msg::Gists { result } => {
                self.gists_loading = false;
                match result {
                    Ok(gists) => {
                        let selected = self.selected_gist().map(|gist| gist.id);
                        self.gists = gists;
                        self.gists_fetched_at = Some(now_epoch());
                        let _ = self.cache.save("gists", &self.gists);
                        self.refresh_cache_stats();
                        if let Some(id) = selected
                            && let Some(index) = self.gists.iter().position(|gist| gist.id == id)
                        {
                            self.gist_selected = index;
                        }
                        self.clamp_gist_selection();
                        self.gist_error = None;
                        self.schedule_gist_detail(true);
                        if self.page == Page::Gists {
                            self.set_status(
                                StatusKind::Ok,
                                &format!("Gist 都拿来啦，一共 {} 个喵~", self.gists.len()),
                            );
                        }
                    }
                    Err(error) => {
                        self.gist_error = Some(error.clone());
                        if self.page == Page::Gists {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
                }
            }
            Msg::GistDetail { id, result } => {
                if self.gist_detail_loading.as_deref() == Some(id.as_str()) {
                    self.gist_detail_loading = None;
                }
                if self.selected_gist().map(|gist| gist.id).as_deref() != Some(id.as_str()) {
                    return;
                }
                match result {
                    Ok(gist) => {
                        self.gist_detail_for = Some(id);
                        self.gist_detail = Some(gist);
                        let count = self.gist_file_names().len();
                        if self.gist_file_selected >= count {
                            self.gist_file_selected = 0;
                        }
                    }
                    Err(error) => {
                        self.gist_error = Some(error.clone());
                        if self.page == Page::Gists {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
                }
            }
            Msg::GistCreated { result } => {
                self.gist_saving = false;
                match result {
                    Ok(gist) => {
                        self.overlay = None;
                        self.gist_create_error = None;
                        self.gists.insert(0, gist.clone());
                        self.gist_selected = 0;
                        self.gist_file_selected = 0;
                        self.gist_scroll = 0;
                        self.gist_detail = Some(gist);
                        self.gist_detail_for = self.selected_gist().map(|gist| gist.id);
                        self.set_status(StatusKind::Ok, "Gist 建好啦喵~");
                        self.fetch_gists(true);
                    }
                    Err(error) => {
                        self.gist_create_error = Some(error.clone());
                        self.set_status(StatusKind::Error, &error);
                    }
                }
            }
            Msg::GistDeleted { id, result } => {
                self.gist_deleting = false;
                match result {
                    Ok(()) => {
                        self.gists.retain(|gist| gist.id != id);
                        self.overlay = None;
                        self.gist_delete = None;
                        self.gist_detail = None;
                        self.gist_detail_for = None;
                        self.clamp_gist_selection();
                        self.schedule_gist_detail(false);
                        let _ = self.cache.save("gists", &self.gists);
                        self.set_status(StatusKind::Ok, "Gist 删掉啦喵~");
                    }
                    Err(error) => self.set_status(StatusKind::Error, &error),
                }
            }
            Msg::Compare {
                name,
                base,
                head,
                result,
            } => {
                self.compare_loading = false;
                let current = self
                    .compare_repo
                    .as_ref()
                    .map(|repo| repo.display_full_name());
                if current.as_deref() != Some(name.as_str())
                    || self.compare_base != base
                    || self.compare_head != head
                {
                    return;
                }
                match result {
                    Ok(compare) => {
                        self.compare_result = Some(compare);
                        self.compare_error = None;
                        self.compare_fetched_for = Some((name, base, head));
                        self.compare_selected = 0;
                        self.compare_scroll = 0;
                        self.pull_created = None;
                        if self.page == Page::Compare {
                            let (ahead, files) = self
                                .compare_result
                                .as_ref()
                                .map(|result| (result.ahead_by, result.files.len()))
                                .unwrap_or((0, 0));
                            self.set_status(
                                StatusKind::Ok,
                                &format!("对比好啦：领先 {ahead} 个提交，动了 {files} 个文件喵~"),
                            );
                        }
                    }
                    Err(error) => {
                        self.compare_error = Some(error.clone());
                        if self.page == Page::Compare {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
                }
            }
            Msg::PullCreated { name, result } => {
                self.pull_saving = false;
                let current = self
                    .compare_repo
                    .as_ref()
                    .map(|repo| repo.display_full_name());
                if current.as_deref() != Some(name.as_str()) {
                    return;
                }
                match result {
                    Ok(pull) => {
                        self.overlay = None;
                        self.pull_error = None;
                        self.pull_created = Some(pull.html_url.clone());
                        self.set_status(
                            StatusKind::Ok,
                            &format!("合并请求 #{} 建好啦喵~ {}", pull.number, pull.html_url),
                        );
                    }
                    Err(error) => {
                        self.pull_error = Some(error.clone());
                        self.set_status(StatusKind::Error, &error);
                    }
                }
            }
            Msg::FileContent {
                name,
                path,
                branch,
                result,
            } => {
                if self.file_loading.as_deref() == Some(path.as_str()) {
                    self.file_loading = None;
                }
                let current = self.file_repo.as_ref().map(Repo::display_full_name);
                if current.as_deref() != Some(name.as_str())
                    || self.file_path != path
                    || self.file_branch != branch
                {
                    return;
                }
                match result {
                    Ok(file) => {
                        self.file_error = None;
                        self.file_view = Some(file);
                        self.file_scroll = 0;
                        self.file_h_scroll = 0;
                        self.set_status(StatusKind::Ok, "文件读出来啦喵~");
                    }
                    Err(error) => {
                        self.file_view = None;
                        self.file_error = Some(error.clone());
                        self.set_status(StatusKind::Error, &error);
                    }
                }
            }
            Msg::NetworkUsers { login, tab, result } => {
                if self.network_loading == Some(tab) {
                    self.network_loading = None;
                }
                if self.network_login.as_deref() != Some(login.as_str()) || self.network_tab != tab
                {
                    return;
                }
                match result {
                    Ok(users) => {
                        if tab == NetworkTab::Following {
                            self.network_known_following =
                                users.iter().map(|user| user.login.clone()).collect();
                        }
                        self.network_users = users;
                        self.network_details.clear();
                        self.network_selected = 0;
                        self.network_following = None;
                        let logins: Vec<String> = self
                            .network_users
                            .iter()
                            .map(|user| user.login.clone())
                            .collect();
                        if !logins.is_empty() {
                            let tx = self.tx.clone();
                            let client = self.client.clone();
                            std::thread::spawn(move || {
                                let result = client.user_summaries(&logins);
                                let _ = tx.send(Msg::NetworkUserDetails { login, tab, result });
                            });
                        }
                        if self.page == Page::Network {
                            self.set_status(
                                StatusKind::Ok,
                                &format!("{} 页已经摆好啦喵~", tab.label()),
                            );
                        }
                    }
                    Err(error) => {
                        if self.page == Page::Network {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
                }
            }
            Msg::NetworkUserDetails { login, tab, result } => {
                if self.network_login.as_deref() != Some(login.as_str()) || self.network_tab != tab
                {
                    return;
                }
                match result {
                    Ok(details) => {
                        for detail in details {
                            let login = detail.login.clone();
                            let Some(user) = self
                                .network_users
                                .iter_mut()
                                .find(|user| user.login == login)
                            else {
                                continue;
                            };
                            user.name = detail.name;
                            user.bio = detail.bio;
                            user.followers = detail.followers;
                            user.following = detail.following;
                            user.public_repos = detail.public_repos;
                            self.network_details.insert(login);
                        }
                    }
                    Err(error) => self.set_status(StatusKind::Error, &error),
                }
            }
            Msg::NetworkOrgs { login, result } => {
                if self.network_loading == Some(NetworkTab::Orgs) {
                    self.network_loading = None;
                }
                if self.network_login.as_deref() != Some(login.as_str())
                    || self.network_tab != NetworkTab::Orgs
                {
                    return;
                }
                match result {
                    Ok(orgs) => {
                        self.network_orgs = orgs;
                        self.network_selected = 0;
                    }
                    Err(error) => {
                        if self.page == Page::Network {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
                }
            }
            Msg::FollowChecked { login, result } => {
                if self.network_follow_loading.as_deref() != Some(login.as_str()) {
                    return;
                }
                self.network_follow_loading = None;
                if self.selected_network_login().as_deref() != Some(login.as_str()) {
                    return;
                }
                match result {
                    Ok(following) => {
                        self.network_following = Some(following);
                        self.apply_follow(login, !following);
                    }
                    Err(error) => self.set_status(StatusKind::Error, &error),
                }
            }
            Msg::FollowToggled {
                login,
                following,
                result,
            } => {
                if self.network_follow_loading.as_deref() == Some(login.as_str()) {
                    self.network_follow_loading = None;
                }
                match result {
                    Ok(()) => {
                        if following {
                            self.network_known_following.insert(login.clone());
                        } else {
                            self.network_known_following.remove(&login);
                        }
                        if self.selected_network_login().as_deref() == Some(login.as_str()) {
                            self.network_following = Some(following);
                        }
                        if !following && self.network_tab == NetworkTab::Following {
                            self.network_users.retain(|user| user.login != login);
                            let count = self.network_users.len();
                            self.network_selected =
                                self.network_selected.min(count.saturating_sub(1));
                        }
                        self.set_status(
                            StatusKind::Ok,
                            if following {
                                "关注好啦喵~"
                            } else {
                                "取消关注啦喵~"
                            },
                        );
                    }
                    Err(error) => self.set_status(StatusKind::Error, &error),
                }
            }
            Msg::ProfileFollowChecked { login, result } => {
                if self.profile_follow_loading.as_deref() != Some(login.as_str()) {
                    return;
                }
                self.profile_follow_loading = None;
                if self.profile_login.as_deref() != Some(login.as_str()) {
                    return;
                }
                match result {
                    Ok(following) => {
                        self.profile_following = Some(following);
                        self.apply_profile_follow(login, !following);
                    }
                    Err(error) => self.set_status(StatusKind::Error, &error),
                }
            }
            Msg::ProfileFollowToggled {
                login,
                following,
                result,
            } => {
                if self.profile_follow_loading.as_deref() == Some(login.as_str()) {
                    self.profile_follow_loading = None;
                }
                match result {
                    Ok(()) => {
                        if self.profile_login.as_deref() == Some(login.as_str()) {
                            self.profile_following = Some(following);
                            if let Some(user) = self
                                .profile_user
                                .as_mut()
                                .filter(|user| user.login == login)
                            {
                                if following {
                                    user.followers = user.followers.saturating_add(1);
                                } else {
                                    user.followers = user.followers.saturating_sub(1);
                                }
                            }
                        }
                        self.set_status(
                            StatusKind::Ok,
                            if following {
                                "关注好啦喵~"
                            } else {
                                "取消关注啦喵~"
                            },
                        );
                    }
                    Err(error) => self.set_status(StatusKind::Error, &error),
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
        let old_name = self
            .page_repo
            .as_ref()
            .or(self.detail.as_ref())
            .map(Repo::display_full_name)
            .unwrap_or_else(|| name.clone());
        let key = repo.cache_key();
        let _ = self.cache.save(&key, &repo);
        for list in [
            &mut self.repos,
            &mut self.starred,
            &mut self.search_results,
            &mut self.trending,
            &mut self.pin_cache,
        ] {
            for item in list.iter_mut() {
                if item.id == repo.id || item.display_full_name() == old_name {
                    *item = repo.clone();
                }
            }
        }
        if self
            .detail
            .as_ref()
            .is_some_and(|detail| detail.id == repo.id || detail.display_full_name() == old_name)
        {
            self.detail = Some(repo.clone());
            self.detail_fetched_at = Some(now_epoch());
        }
        if self.page_repo.as_ref().is_some_and(|page_repo| {
            page_repo.id == repo.id || page_repo.display_full_name() == old_name
        }) {
            self.page_repo = Some(repo.clone());
        }
        if self.issue_repo.as_ref().is_some_and(|issue_repo| {
            issue_repo.id == repo.id || issue_repo.display_full_name() == old_name
        }) {
            self.issue_repo = Some(repo.clone());
        }
        if self.commit_repo.as_ref().is_some_and(|commit_repo| {
            commit_repo.id == repo.id || commit_repo.display_full_name() == old_name
        }) {
            self.commit_repo = Some(repo.clone());
        }
        if self.action_repo.as_deref() == Some(old_name.as_str()) {
            self.action_repo = Some(name.clone());
        }
        if let Some(login) = self.login.clone() {
            let _ = self.cache.save(&repos_cache_key(&login), &self.repos);
            let _ = self.cache.save(&starred_cache_key(&login), &self.starred);
        }
        if name != old_name {
            self.tree = None;
            self.readme = None;
            self.commits.clear();
            self.branches.clear();
            self.branches_fetched_at = None;
            self.tree_fetched_at = None;
            self.readme_fetched_at = None;
            self.commits_fetched_at = None;
            if self.page == Page::Repo {
                self.schedule_repo_extras(&repo, true);
            }
            if self.page == Page::Commits {
                self.fetch_commits(true);
            }
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
            Page::Repo | Page::ReleaseDetail | Page::Tags => {
                self.page_repo.clone().or_else(|| self.detail.clone())
            }
            Page::Actions => self
                .action_repo
                .as_deref()
                .and_then(|name| self.repo_by_name(name))
                .or_else(|| self.page_repo.clone()),
            Page::Issues | Page::IssueDetail => self.issue_repo.clone(),
            Page::Commits => self.commit_repo.clone(),
            Page::Compare => self.compare_repo.clone(),
            Page::Files => self.file_repo.clone(),
            Page::Gists | Page::Network => None,
            Page::Notifications | Page::Settings | Page::Home | Page::Insights => None,
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

    pub fn rerun_action_run(&mut self) {
        let Some(run) = self.selected_action_run() else {
            self.set_status(StatusKind::Info, "先挑一次工作流运行嘛喵~");
            return;
        };
        let Some(name) = self.action_repo.clone() else {
            return;
        };
        let Some(repo) = self.repo_by_name(&name) else {
            self.set_status(StatusKind::Error, "找不到这个仓库的凭据喵~");
            return;
        };
        if run.status != "completed" {
            self.set_status(StatusKind::Info, "这次运行还没结束，先别重跑喵~");
            return;
        }
        self.set_status(StatusKind::Info, "人家正在重新运行这次工作流喵~...");
        let tx = self.tx.clone();
        let client = self.client.clone();
        let run_id = run.id;
        std::thread::spawn(move || {
            let result = client.rerun_action_run(&name, run_id, repo.private);
            let _ = tx.send(Msg::ActionRunRerun { name, result });
        });
    }

    pub fn open_cancel_action_run(&mut self) {
        let Some(run) = self.selected_action_run() else {
            self.set_status(StatusKind::Info, "先挑一次工作流运行嘛喵~");
            return;
        };
        if run.status == "completed" {
            self.set_status(StatusKind::Info, "这次运行已经结束了，不用取消喵~");
            return;
        }
        self.cancel_loading = false;
        self.cancel_run = Some(run);
        self.overlay = Some(Overlay::CancelRun);
    }

    pub fn confirm_cancel_action_run(&mut self) {
        if self.cancel_loading {
            return;
        }
        let Some(run) = self.cancel_run.clone() else {
            self.overlay = None;
            return;
        };
        let Some(name) = self.action_repo.clone() else {
            self.overlay = None;
            return;
        };
        let Some(repo) = self.repo_by_name(&name) else {
            self.set_status(StatusKind::Error, "找不到这个仓库的凭据喵~");
            return;
        };
        self.cancel_loading = true;
        self.set_status(StatusKind::Info, "人家正在取消这次运行喵~...");
        let tx = self.tx.clone();
        let client = self.client.clone();
        let run_id = run.id;
        std::thread::spawn(move || {
            let result = client.cancel_action_run(&name, run_id, repo.private);
            let _ = tx.send(Msg::ActionRunCancelled { name, result });
        });
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
        let query = self.issue_filter.value.trim().to_lowercase();
        self.issues
            .iter()
            .enumerate()
            .filter(|(_, issue)| self.issue_kind.matches(issue))
            .filter(|(_, issue)| {
                query.is_empty()
                    || issue.title.to_lowercase().contains(&query)
                    || issue.author().to_lowercase().contains(&query)
                    || issue.labels_label().to_lowercase().contains(&query)
            })
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
        self.open_issue_detail();
    }

    pub fn open_selected_issue_browser(&mut self) {
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

    pub fn open_new_issue(&mut self) {
        if self.issue_repo.is_none() {
            self.set_status(StatusKind::Info, "先打开一个仓库的议题页嘛喵~");
            return;
        }
        self.overlay = Some(Overlay::NewIssue);
        self.new_issue_title = TextInput::new("");
        self.new_issue_body = TextInput::new("");
        self.new_issue_body_focus = false;
        self.new_issue_saving = false;
        self.new_issue_error = None;
    }

    fn active_new_issue_input(&mut self) -> &mut TextInput {
        if self.new_issue_body_focus {
            &mut self.new_issue_body
        } else {
            &mut self.new_issue_title
        }
    }

    fn handle_new_issue_key(&mut self, key: KeyEvent) {
        if self.new_issue_saving {
            return;
        }
        match key.code {
            KeyCode::Esc => {
                self.overlay = None;
                self.new_issue_error = None;
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.new_issue_body_focus = !self.new_issue_body_focus;
            }
            KeyCode::Enter => {
                if self.new_issue_body_focus {
                    self.new_issue_body.insert('\n');
                } else {
                    self.new_issue_body_focus = true;
                }
            }
            KeyCode::Backspace => self.active_new_issue_input().backspace(),
            KeyCode::Delete => self.active_new_issue_input().delete(),
            KeyCode::Left => self.active_new_issue_input().left(),
            KeyCode::Right => self.active_new_issue_input().right(),
            KeyCode::Home => self.active_new_issue_input().home(),
            KeyCode::End => self.active_new_issue_input().end(),
            KeyCode::Char(ch) if !ch.is_control() => self.active_new_issue_input().insert(ch),
            _ => {}
        }
    }

    fn submit_new_issue(&mut self) {
        if self.new_issue_saving {
            return;
        }
        let Some(repo) = self.issue_repo.clone() else {
            self.set_status(StatusKind::Info, "先打开一个仓库的议题页嘛喵~");
            return;
        };
        let title = self.new_issue_title.value.trim().to_string();
        if title.is_empty() {
            self.new_issue_body_focus = false;
            self.new_issue_error = Some("标题不能空着喵~".to_string());
            return;
        }
        let body = self.new_issue_body.value.clone();
        let name = repo.display_full_name();
        self.new_issue_saving = true;
        self.new_issue_error = None;
        self.set_status(StatusKind::Info, "正在把新议题送去 GitHub 喵~...");
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.create_issue(&name, &title, &body);
            let _ = tx.send(Msg::IssueCreated { name, result });
        });
    }

    pub fn open_repo_languages(&mut self) {
        let Some(repo) = self.context_repo() else {
            self.set_status(StatusKind::Info, "先挑一个仓库嘛喵~");
            return;
        };
        let name = repo.display_full_name();
        self.overlay = Some(Overlay::RepoLanguages);
        self.repo_language_offset = 0;
        match self
            .cache
            .load::<Vec<LanguageStat>>(&languages_cache_key(&name))
        {
            Some(cached) => {
                self.repo_languages = cached.data;
                self.repo_languages_fetched_at = Some(cached.fetched_at);
            }
            None => {
                self.repo_languages.clear();
                self.repo_languages_fetched_at = None;
            }
        }
        self.fetch_repo_languages(false);
    }

    fn fetch_repo_languages(&mut self, force: bool) {
        let Some(repo) = self.context_repo() else {
            return;
        };
        let name = repo.display_full_name();
        if self.repo_languages_loading.as_deref() == Some(name.as_str()) {
            return;
        }
        let fresh = self
            .repo_languages_fetched_at
            .map(|at| now_epoch() - at <= self.config.detail_ttl_secs as i64)
            .unwrap_or(false);
        if !force && fresh {
            return;
        }
        self.repo_languages_loading = Some(name.clone());
        let tx = self.tx.clone();
        let client = self.client.clone();
        let private = repo.private;
        std::thread::spawn(move || {
            let result = client.languages(&name, private);
            let _ = tx.send(Msg::RepoLanguages { name, result });
        });
    }

    fn schedule_commit_detail(&mut self, force: bool) {
        let (Some(repo), Some(commit)) = (self.commit_repo.clone(), self.selected_commit()) else {
            self.commit_detail = None;
            self.commit_detail_for = None;
            self.commit_detail_loading = None;
            self.pending_commit_detail = None;
            return;
        };
        let sha = commit.sha.clone();
        if sha.is_empty() {
            return;
        }
        let name = repo.display_full_name();
        if self.commit_detail_for.as_deref() != Some(sha.as_str()) {
            self.commit_detail = None;
            self.commit_detail_for = None;
        }
        let key = commit_detail_cache_key(&name, &sha);
        if let Some(cached) = self.cache.load::<CommitDetail>(&key) {
            let fresh = cached.is_fresh(self.config.detail_ttl_secs);
            if self.commit_detail_for.as_deref() != Some(sha.as_str()) {
                self.commit_detail = Some(cached.data);
                self.commit_detail_for = Some(sha.clone());
            }
            if !force && fresh {
                return;
            }
        }
        if self.commit_detail_loading.as_deref() == Some(sha.as_str()) {
            return;
        }
        self.pending_commit_detail = Some(PendingCommitDetail {
            repo: name,
            sha,
            private: repo.private,
            at: Instant::now(),
        });
    }

    fn fetch_pending_commit_detail(&mut self, pending: PendingCommitDetail) {
        let current_repo = self.commit_repo.as_ref().map(Repo::display_full_name);
        let current_sha = self.selected_commit().map(|commit| commit.sha);
        if current_repo.as_deref() != Some(pending.repo.as_str())
            || current_sha.as_deref() != Some(pending.sha.as_str())
        {
            return;
        }
        if self.commit_detail_loading.as_deref() == Some(pending.sha.as_str()) {
            return;
        }
        self.commit_detail_loading = Some(pending.sha.clone());
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.commit_detail(&pending.repo, &pending.sha, pending.private);
            let _ = tx.send(Msg::CommitDetail {
                repo: pending.repo,
                sha: pending.sha,
                result,
            });
        });
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
        let next = (edit.row as i32 + delta).clamp(0, 8) as usize;
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

    pub fn toggle_repo_flag(&mut self, row: usize) {
        let Some(edit) = self.repo_edit.as_mut() else {
            return;
        };
        match row {
            6 => edit.has_issues = !edit.has_issues,
            7 => edit.has_discussions = !edit.has_discussions,
            8 => edit.has_wiki = !edit.has_wiki,
            _ => {}
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
        let name = edit.name.value.trim().to_string();
        if name.is_empty() {
            edit.saving = false;
            edit.error = Some("仓库名不能空着呀喵~".to_string());
            return;
        }
        let description = edit.description.value.trim().to_string();
        let homepage = edit.homepage.value.trim().to_string();
        let topics = edit.topics_list();
        let private = edit.private;
        let archived = edit.archived;
        let has_issues = edit.has_issues;
        let has_discussions = edit.has_discussions;
        let has_wiki = edit.has_wiki;
        let patch = serde_json::json!({
            "name": name,
            "description": if description.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(description) },
            "homepage": if homepage.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(homepage) },
            "topics": topics,
            "private": private,
            "archived": archived,
            "has_issues": has_issues,
            "has_discussions": has_discussions,
            "has_wiki": has_wiki,
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
            let row = edit.row;
            match key.code {
                KeyCode::Enter | KeyCode::Char('\r') | KeyCode::Char('\n') => {
                    edit.editing = false;
                    if row < 3 {
                        edit.row += 1;
                    }
                }
                KeyCode::Esc => edit.editing = false,
                KeyCode::Backspace => {
                    if let Some(input) = repo_edit_input(edit, row) {
                        input.backspace();
                    }
                }
                KeyCode::Delete => {
                    if let Some(input) = repo_edit_input(edit, row) {
                        input.delete();
                    }
                }
                KeyCode::Left => {
                    if let Some(input) = repo_edit_input(edit, row) {
                        input.left();
                    }
                }
                KeyCode::Right => {
                    if let Some(input) = repo_edit_input(edit, row) {
                        input.right();
                    }
                }
                KeyCode::Home => {
                    if let Some(input) = repo_edit_input(edit, row) {
                        input.home();
                    }
                }
                KeyCode::End => {
                    if let Some(input) = repo_edit_input(edit, row) {
                        input.end();
                    }
                }
                KeyCode::Char(ch) => {
                    if let Some(input) = repo_edit_input(edit, row) {
                        input.insert(ch);
                    }
                }
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
                    edit.row = 8;
                }
            }
            KeyCode::Enter | KeyCode::Char('\r') | KeyCode::Char('\n') => match row {
                0..=3 => {
                    if let Some(edit) = self.repo_edit.as_mut() {
                        edit.editing = true;
                    }
                }
                4 => self.toggle_repo_visibility(),
                5 => self.toggle_repo_archived(),
                6 => self.toggle_repo_flag(6),
                7 => self.toggle_repo_flag(7),
                8 => self.toggle_repo_flag(8),
                _ => {}
            },
            KeyCode::Left | KeyCode::Right => match row {
                4 => self.toggle_repo_visibility(),
                5 => self.toggle_repo_archived(),
                6..=8 => self.toggle_repo_flag(row),
                _ => {}
            },
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
        if let Some(pending) = self.pending_commit_detail.take() {
            if pending.at.elapsed() >= Duration::from_millis(220) {
                self.fetch_pending_commit_detail(pending);
            } else {
                self.pending_commit_detail = Some(pending);
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
        if self.page == Page::Profile && (self.profile_user_loading || self.profile_repos_loading) {
            return Some("人家正在翻这个人的主页喵~...");
        }
        if self.fork_loading.is_some() {
            return Some("人家正在帮你 Fork 仓库喵~...");
        }
        if self.delete_loading.is_some() {
            return Some("人家正在删除仓库喵~...");
        }
        if self.search_loading && self.page == Page::Search {
            return Some("人家正在翻找仓库喵~...");
        }
        if self.commits_loading.is_some() && self.page == Page::Commits {
            return Some("人家正在翻提交记录喵~...");
        }
        if self.branches_loading.is_some() && self.overlay == Some(Overlay::Branches) {
            return Some("人家正在取分支列表喵~...");
        }
        if self.trending_loading && self.page == Page::Trending {
            return Some("人家正在打听最近的热门仓库喵~...");
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
                Page::Repositories | Page::Starred | Page::Search | Page::Trending | Page::Repo
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
            Page::Profile => &self.profile_repos,
            Page::Search => &self.search_results,
            Page::Trending => &self.trending,
            _ => &self.repos,
        }
    }

    pub fn visible_indices(&self) -> Vec<usize> {
        self.visible_indices_for(self.current_list_page())
    }

    fn filter_for(&self, page: Page) -> &TextInput {
        match page {
            Page::Starred => &self.star_filter,
            Page::Search => &self.search_filter,
            Page::Trending => &self.trending_filter,
            _ => &self.repo_filter,
        }
    }

    fn filter_for_mut(&mut self, page: Page) -> &mut TextInput {
        match page {
            Page::Starred => &mut self.star_filter,
            Page::Search => &mut self.search_filter,
            Page::Trending => &mut self.trending_filter,
            _ => &mut self.repo_filter,
        }
    }

    pub fn current_filter(&self) -> &TextInput {
        self.filter_for(self.page)
    }

    pub fn list_filter_active(&self) -> bool {
        let page = self.page;
        if !matches!(
            page,
            Page::Repositories | Page::Starred | Page::Search | Page::Trending
        ) {
            return false;
        }
        let language_active = matches!(page, Page::Repositories | Page::Search | Page::Trending)
            && self.language_filter.is_some();
        language_active || !self.current_filter().value.trim().is_empty()
    }

    pub fn focus_list_filter(&mut self) {
        if !matches!(
            self.page,
            Page::Repositories | Page::Starred | Page::Search | Page::Trending
        ) {
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
        let language = if matches!(page, Page::Repositories | Page::Search | Page::Trending) {
            self.language_filter.as_deref()
        } else {
            None
        };
        let query = if matches!(
            page,
            Page::Repositories | Page::Starred | Page::Search | Page::Trending
        ) {
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
            Page::Profile => self.profile_selected,
            Page::Search => self.search_selected,
            Page::Trending => self.trending_selected,
            _ => self.repo_selected,
        }
    }

    fn set_selection_for(&mut self, page: Page, value: usize) {
        match page {
            Page::Starred => self.star_selected = value,
            Page::Profile => self.profile_selected = value,
            Page::Search => self.search_selected = value,
            Page::Trending => self.trending_selected = value,
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
        matches!(
            page,
            Page::Repositories | Page::Starred | Page::Search | Page::Trending | Page::Profile
        )
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
        for page in [
            Page::Repositories,
            Page::Starred,
            Page::Profile,
            Page::Search,
            Page::Trending,
        ] {
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
        for repo in self.raw_list(self.current_list_page()) {
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
        let mut choices = vec![(
            None,
            "全部语言喵~".to_string(),
            self.raw_list(self.current_list_page()).len(),
        )];
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
        let language = self
            .language_filter
            .as_deref()
            .map(|language| format!(" · 只看 {language}"))
            .unwrap_or_default();
        format!("「{query}」一共找到 {total} 个{language}呢喵~")
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

    fn fetch_trending(&mut self, force: bool) {
        if self.trending_loading {
            return;
        }
        let period = self.trending_period;
        let key = trending_key(period);
        let cached = self.cache.load::<SearchResponse>(&key);
        let fresh = cached
            .as_ref()
            .is_some_and(|cached| cached.is_fresh(self.config.search_ttl_secs));
        if let Some(cached) = &cached {
            self.trending = cached.data.items.clone();
            self.trending_total = Some(cached.data.total_count);
        }
        if force || cached.is_none() {
            self.trending.clear();
            self.trending_total = None;
        }
        self.clamp_selection();
        if fresh && !force {
            if self.page == Page::Trending {
                self.schedule_selected_detail();
                self.set_status(StatusKind::Info, "热门列表刚从缓存里拿出来啦喵~");
            }
            return;
        }
        self.trending_loading = true;
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Trending {
                period,
                result: client.trending(period),
            });
        });
    }

    fn schedule_selected_detail(&mut self) {
        // Repo/Profile pages own their detail state; rescheduling would replace the
        // releases of the repo the user is currently viewing.
        if !should_schedule_selected_detail(self.page) {
            return;
        }
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

    pub fn open_tags(&mut self) {
        let Some(repo) = self.context_repo() else {
            self.set_status(StatusKind::Info, "先挑一个仓库嘛，人家才知道看哪个标签喵~");
            return;
        };
        let name = repo.display_full_name();
        let changed = self
            .tag_repo
            .as_ref()
            .map(|current| current.display_full_name() != name)
            .unwrap_or(true);
        if changed {
            self.tags.clear();
            self.tags_fetched_at = None;
            self.tag_selected = 0;
            self.tag_scroll = 0;
        }
        self.tag_repo = Some(repo.clone());
        self.return_page = return_page_when_opening(self.page, self.return_page);
        self.page = Page::Tags;
        self.load_tags_cache();
        self.fetch_tags(false);
    }

    fn load_tags_cache(&mut self) {
        let Some(repo) = self.tag_repo.clone() else {
            return;
        };
        match self.cache.load::<Vec<Tag>>(&repo.tags_cache_key()) {
            Some(cached) => {
                self.tags = cached.data;
                self.tags_fetched_at = Some(cached.fetched_at);
            }
            None => {
                self.tags.clear();
                self.tags_fetched_at = None;
            }
        }
        self.clamp_tag_selection();
    }

    fn fetch_tags(&mut self, force: bool) {
        let Some(repo) = self.tag_repo.clone() else {
            return;
        };
        let name = repo.display_full_name();
        if self.tags_loading.as_deref() == Some(name.as_str()) {
            return;
        }
        let fresh = self
            .tags_fetched_at
            .map(|at| now_epoch() - at <= self.config.detail_ttl_secs as i64)
            .unwrap_or(false);
        if !force && fresh {
            return;
        }
        self.tags_loading = Some(name.clone());
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Tags {
                name: name.clone(),
                result: client.tags(&name, repo.private),
            });
        });
    }

    pub fn tag_repo_label(&self) -> String {
        self.tag_repo
            .as_ref()
            .map(|repo| repo.display_full_name())
            .unwrap_or_else(|| "还没有选仓库".to_string())
    }

    pub fn selected_tag(&self) -> Option<Tag> {
        self.tags.get(self.tag_selected).cloned()
    }

    pub fn select_tag(&mut self, index: usize) {
        if !self.tags.is_empty() {
            self.tag_selected = index.min(self.tags.len() - 1);
        }
    }

    pub fn move_tag_selection(&mut self, delta: i32) {
        if self.tags.is_empty() {
            return;
        }
        self.tag_selected =
            (self.tag_selected as i32 + delta).clamp(0, self.tags.len() as i32 - 1) as usize;
    }

    fn clamp_tag_selection(&mut self) {
        self.tag_selected = self.tag_selected.min(self.tags.len().saturating_sub(1));
    }

    pub fn open_selected_tag_page(&mut self) {
        let Some(tag) = self.selected_tag() else {
            self.set_status(StatusKind::Info, "先挑一个标签嘛喵~");
            return;
        };
        let Some(repo) = self.tag_repo.clone() else {
            return;
        };
        let base = if repo.html_url.trim().is_empty() {
            format!("https://github.com/{}", repo.display_full_name())
        } else {
            repo.html_url.clone()
        };
        let url = format!("{}/tree/{}", base.trim_end_matches('/'), tag.name);
        self.open_url(&url);
    }

    pub fn copy_selected_tag_name(&mut self) {
        let Some(tag) = self.selected_tag() else {
            self.set_status(StatusKind::Info, "先挑一个标签嘛喵~");
            return;
        };
        self.copy_text(&tag.name, "标签名喵~");
    }

    pub fn download_selected_tag_archive(&mut self, zip: bool) {
        let Some(tag) = self.selected_tag() else {
            self.set_status(StatusKind::Info, "先挑一个标签嘛喵~");
            return;
        };
        let Some(repo) = self.tag_repo.clone() else {
            return;
        };
        let url = if zip {
            tag.zipball_url.clone()
        } else {
            tag.tarball_url.clone()
        };
        if url.trim().is_empty() {
            self.set_status(StatusKind::Error, "这个标签没有源码包地址喵~");
            return;
        }
        let filename = format!(
            "{}-{}.{}",
            repo.name,
            tag.name,
            if zip { "zip" } else { "tar.gz" }
        );
        self.start_download(
            format!("标签 {} 的源码包喵~", format::clean_text(&tag.name)),
            url,
            repo.private,
            filename,
        );
    }

    fn schedule_repo_extras(&mut self, repo: &Repo, force: bool) {
        let name = repo.display_full_name();
        let branch = self.branch_for(repo);
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
            if self.readme_for.as_deref() != Some(name.as_str()) || self.readme_branch != branch {
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

    pub fn branch_for(&self, repo: &Repo) -> String {
        if self.tree_branch.trim().is_empty() {
            repo.default_branch.clone()
        } else {
            self.tree_branch.clone()
        }
    }

    fn load_tree_cache(&mut self, repo: &Repo) {
        let name = repo.display_full_name();
        let branch = self.branch_for(repo);
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
        let branch = self.branch_for(repo);
        self.readme = None;
        self.readme_fetched_at = None;
        self.readme_for = Some(name.clone());
        self.readme_branch = branch.clone();
        self.readme_scroll = 0;
        self.readme_scroll_max = 0;
        if let Some(cached) = self
            .cache
            .load::<ReadmeContent>(&readme_cache_key(&name, &branch))
        {
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
        let branch = self.branch_for(repo);
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
        let branch = self.branch_for(repo);
        let private = repo.private;
        std::thread::spawn(move || {
            let result = client.readme(&name, &branch, private);
            let _ = tx.send(Msg::Readme {
                name,
                branch,
                result,
            });
        });
    }

    fn load_branches_cache(&mut self, repo: &Repo) {
        let name = repo.display_full_name();
        self.branches.clear();
        self.branches_fetched_at = None;
        self.branch_selected = 0;
        self.branch_offset = 0;
        if let Some(cached) = self.cache.load::<Vec<Branch>>(&branches_cache_key(&name)) {
            self.branches = cached.data;
            self.branches_fetched_at = Some(cached.fetched_at);
        }
        self.clamp_branch_selection();
    }

    fn fetch_branches(&mut self, force: bool) {
        let Some(repo) = self.context_repo() else {
            return;
        };
        let name = repo.display_full_name();
        if self.branches_loading.as_deref() == Some(name.as_str()) {
            return;
        }
        let fresh = self
            .branches_fetched_at
            .map(|at| now_epoch() - at <= self.config.detail_ttl_secs as i64)
            .unwrap_or(false);
        if !force && fresh {
            return;
        }
        self.branches_loading = Some(name.clone());
        let tx = self.tx.clone();
        let client = self.client.clone();
        let private = repo.private;
        std::thread::spawn(move || {
            let result = client.branches(&name, private);
            let _ = tx.send(Msg::Branches { name, result });
        });
    }

    pub fn open_branches(&mut self) {
        let Some(repo) = self.context_repo() else {
            self.set_status(StatusKind::Info, "先挑一个仓库嘛喵~");
            return;
        };
        let current = self.branch_for(&repo);
        if self.branches_fetched_at.is_none() {
            self.load_branches_cache(&repo);
        }
        self.branch_selected = self
            .branches
            .iter()
            .position(|branch| branch.name == current)
            .unwrap_or(0);
        let viewport = self.branch_viewport.max(1);
        self.branch_offset = self.branch_selected.saturating_sub(viewport - 1);
        self.overlay = Some(Overlay::Branches);
        self.fetch_branches(false);
    }

    pub fn move_branch_selection(&mut self, delta: i32) {
        let len = self.branches.len();
        if len == 0 {
            return;
        }
        let next = (self.branch_selected as i32 + delta).clamp(0, len as i32 - 1) as usize;
        self.branch_selected = next;
        let viewport = self.branch_viewport.max(1);
        if next < self.branch_offset {
            self.branch_offset = next;
        } else if next >= self.branch_offset + viewport {
            self.branch_offset = next + 1 - viewport;
        }
    }

    pub fn select_branch(&mut self, branch: String) {
        let Some(repo) = self.context_repo() else {
            return;
        };
        if branch.is_empty() {
            return;
        }
        self.overlay = None;
        if branch == self.branch_for(&repo) {
            return;
        }
        self.tree_branch = branch.clone();
        self.readme_branch = branch.clone();
        self.commit_branch = branch.clone();
        self.tree = None;
        self.tree_for = None;
        self.tree_fetched_at = None;
        self.tree_loading = None;
        self.readme = None;
        self.readme_for = None;
        self.readme_fetched_at = None;
        self.readme_loading = None;
        self.commits.clear();
        self.commits_fetched_at = None;
        self.commits_loading = None;
        self.commit_selected = 0;
        self.commit_detail = None;
        self.commit_detail_for = None;
        self.commit_detail_loading = None;
        self.pending_commit_detail = None;
        self.tree_scroll = 0;
        self.readme_scroll = 0;
        self.commit_scroll = 0;
        self.set_status(StatusKind::Ok, &format!("切到 {branch} 分支啦喵~"));
        self.schedule_repo_extras(&repo, true);
        if matches!(self.page, Page::Commits) {
            self.fetch_commits(true);
        }
    }

    pub fn open_commits(&mut self) {
        let Some(repo) = self.context_repo() else {
            self.set_status(StatusKind::Info, "先挑一个仓库嘛喵~");
            return;
        };
        let name = repo.display_full_name();
        let branch = self.branch_for(&repo);
        if self
            .commit_repo
            .as_ref()
            .map(Repo::display_full_name)
            .as_deref()
            != Some(name.as_str())
            || self.commit_branch != branch
        {
            self.commits.clear();
            self.commits_fetched_at = None;
            self.commit_selected = 0;
            self.commit_scroll = 0;
            self.commit_detail = None;
            self.commit_detail_for = None;
            self.commit_detail_loading = None;
            self.pending_commit_detail = None;
            self.commit_repo = Some(repo);
            self.commit_branch = branch.clone();
            if let Some(cached) = self
                .cache
                .load::<Vec<Commit>>(&commits_cache_key(&name, &branch))
            {
                self.commits = cached.data;
                self.commits_fetched_at = Some(cached.fetched_at);
            }
        }
        self.return_page = return_page_when_opening(self.page, self.return_page);
        self.page = Page::Commits;
        self.clamp_commit_selection();
        self.schedule_commit_detail(false);
        self.fetch_commits(false);
    }

    fn fetch_commits(&mut self, force: bool) {
        let Some(repo) = self.commit_repo.clone() else {
            return;
        };
        let name = repo.display_full_name();
        let branch = self.commit_branch.clone();
        let key = (name.clone(), branch.clone());
        if self.commits_loading.as_ref() == Some(&key) {
            return;
        }
        let fresh = self
            .commits_fetched_at
            .map(|at| now_epoch() - at <= self.config.detail_ttl_secs as i64)
            .unwrap_or(false);
        if !force && fresh {
            return;
        }
        self.commits_loading = Some(key);
        let tx = self.tx.clone();
        let client = self.client.clone();
        let limit = self.config.commit_limit.clamp(10, 100);
        let private = repo.private;
        std::thread::spawn(move || {
            let result = client.commits(&name, &branch, limit, private);
            let _ = tx.send(Msg::Commits {
                name,
                branch,
                result,
            });
        });
    }

    fn clamp_branch_selection(&mut self) {
        if self.branches.is_empty() {
            self.branch_selected = 0;
            self.branch_offset = 0;
        } else {
            self.branch_selected = self.branch_selected.min(self.branches.len() - 1);
        }
    }

    fn clamp_commit_selection(&mut self) {
        let len = self.commit_indices().len();
        if len == 0 {
            self.commit_selected = 0;
        } else {
            self.commit_selected = self.commit_selected.min(len - 1);
        }
    }

    pub fn commit_indices(&self) -> Vec<usize> {
        let query = self.commit_filter.value.trim().to_lowercase();
        self.commits
            .iter()
            .enumerate()
            .filter(|(_, commit)| {
                query.is_empty()
                    || commit.summary().to_lowercase().contains(&query)
                    || commit.author_name().to_lowercase().contains(&query)
                    || commit.sha.to_lowercase().contains(&query)
            })
            .map(|(index, _)| index)
            .collect()
    }

    pub fn selected_commit(&self) -> Option<Commit> {
        self.commit_indices()
            .get(self.commit_selected)
            .and_then(|index| self.commits.get(*index))
            .cloned()
    }

    pub fn move_commit_selection(&mut self, delta: i32) {
        let len = self.commit_indices().len();
        if len == 0 {
            return;
        }
        self.commit_selected =
            (self.commit_selected as i32 + delta).clamp(0, len as i32 - 1) as usize;
        self.commit_scroll = 0;
        self.schedule_commit_detail(false);
    }

    pub fn select_commit(&mut self, index: usize) {
        let len = self.commit_indices().len();
        if len == 0 {
            return;
        }
        self.commit_selected = index.min(len - 1);
        self.commit_scroll = 0;
        self.schedule_commit_detail(false);
    }

    pub fn open_selected_commit(&mut self) {
        let Some(commit) = self.selected_commit() else {
            self.set_status(StatusKind::Info, "先挑一个提交嘛喵~");
            return;
        };
        self.open_url(&commit.html_url);
    }

    pub fn commit_repo_label(&self) -> String {
        self.commit_repo
            .as_ref()
            .map(|repo| format!("{} · {}", repo.display_full_name(), self.commit_branch))
            .unwrap_or_else(|| "还没有选仓库".to_string())
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
            Page::Trending => self.fetch_trending(true),
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
            Page::Profile => {
                if let Some(login) = self.profile_login.clone() {
                    self.fetch_profile(login);
                }
            }
            Page::Actions => {
                self.fetch_action_runs(true);
                self.schedule_action_jobs(true);
            }
            Page::Issues => self.fetch_issues(true),
            Page::Commits => {
                self.fetch_commits(true);
                self.schedule_commit_detail(true);
            }
            Page::ReleaseDetail => {
                if let Some(repo) = self.page_repo.clone() {
                    self.schedule_repo_extras(&repo, true);
                }
            }
            Page::Tags => self.fetch_tags(true),
            Page::IssueDetail => self.refresh_issue_detail(),
            Page::Gists => self.fetch_gists(true),
            Page::Compare => self.fetch_compare(),
            Page::Files => self.refresh_file(),
            Page::Network => self.fetch_network(true),
            Page::Notifications => self.fetch_notifications(true),
            Page::Settings => self.refresh_cache_stats(),
            Page::Home => {
                self.fetch_repos_if_needed(true);
                self.fetch_starred_if_needed(true);
                self.fetch_notifications(true);
                self.sync_pins();
                self.refresh_cache_stats();
            }
            Page::Insights => {
                self.refresh_cache_stats();
                self.fetch_rate_limit(true);
            }
        }
    }

    pub fn navigate(&mut self, page: Page) {
        if self.settings_only && page != Page::Settings {
            return;
        }
        if page == Page::Gists {
            self.open_gists();
            return;
        }
        if page == Page::Compare {
            self.open_compare();
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
        if let Some(index) = page.nav_index() {
            self.nav_offset = index;
        }
        self.page_repo = None;
        self.issue_repo = None;
        self.search_focus = page == Page::Search;
        self.list_filter_focus = false;
        self.release_filter_focus = false;
        self.readme_fullscreen = false;
        self.list_offset = 0;
        self.pending_detail = None;
        if page == Page::Settings {
            self.refresh_cache_stats();
            self.editing_proxy = false;
            self.editing_download_dir = false;
        }
        if page == Page::Home {
            self.sync_pins();
            self.refresh_cache_stats();
        }
        if page == Page::Insights {
            self.refresh_cache_stats();
            self.fetch_rate_limit(false);
        }
        if matches!(page, Page::Repositories | Page::Starred) {
            self.schedule_selected_detail();
            self.follow_selection();
        }
        if page == Page::Search {
            self.schedule_selected_detail();
        }
        if page == Page::Trending {
            self.schedule_selected_detail();
            self.fetch_trending(false);
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
        self.open_repo(repo);
    }

    pub fn open_repo(&mut self, repo: Repo) {
        self.return_page = return_page_when_opening(self.page, self.return_page);
        self.issue_repo = None;
        self.release_filter = TextInput::new("");
        self.release_filter_focus = false;
        self.page_repo = Some(repo.clone());
        self.page = Page::Repo;
        if let Some(index) = self.return_page.nav_index() {
            self.nav_offset = index;
        }
        self.page_scroll = 0;
        self.repo_actions_offset = 0;
        self.search_focus = false;
        self.readme_fullscreen = false;
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
        self.tree_branch = repo.default_branch.clone();
        self.readme_branch = repo.default_branch.clone();
        self.commit_branch = repo.default_branch.clone();
        self.tree_loading = None;
        self.readme_loading = None;
        self.commits.clear();
        self.commits_fetched_at = None;
        self.commits_loading = None;
        self.commit_repo = None;
        self.commit_selected = 0;
        self.commit_scroll = 0;
        self.commit_detail = None;
        self.commit_detail_for = None;
        self.commit_detail_loading = None;
        self.pending_commit_detail = None;
        self.branches.clear();
        self.branches_fetched_at = None;
        self.branches_loading = None;
        self.schedule_repo_extras(&repo, false);
        self.pending_detail = Some(PendingDetail {
            repo,
            at: Instant::now(),
        });
    }

    pub fn pinned_list(&self) -> Vec<Repo> {
        self.config
            .pinned_repos
            .iter()
            .map(|name| {
                self.pin_cache
                    .iter()
                    .chain(self.repos.iter())
                    .chain(self.starred.iter())
                    .chain(self.search_results.iter())
                    .chain(self.trending.iter())
                    .chain(self.profile_repos.iter())
                    .find(|repo| repo.display_full_name().eq_ignore_ascii_case(name))
                    .cloned()
                    .unwrap_or_else(|| Repo {
                        full_name: name.clone(),
                        ..Repo::default()
                    })
            })
            .collect()
    }

    pub fn is_pinned(&self, repo: &Repo) -> bool {
        let name = repo.display_full_name();
        self.config
            .pinned_repos
            .iter()
            .any(|item| item.eq_ignore_ascii_case(&name))
    }

    pub fn toggle_pin(&mut self) {
        let target = if self.page == Page::Repo {
            self.page_repo.clone().or_else(|| self.detail.clone())
        } else if matches!(
            self.page,
            Page::Repositories | Page::Starred | Page::Search | Page::Trending | Page::Profile
        ) {
            self.selected_repo()
        } else {
            None
        };
        let Some(repo) = target else {
            self.set_status(StatusKind::Info, "先挑一个仓库，人家才好在首页钉住它喵~");
            return;
        };
        let name = repo.display_full_name();
        if let Some(position) = self
            .config
            .pinned_repos
            .iter()
            .position(|item| item.eq_ignore_ascii_case(&name))
        {
            self.config.pinned_repos.remove(position);
            self.pin_cache
                .retain(|item| !item.display_full_name().eq_ignore_ascii_case(&name));
            self.home_selected = self
                .home_selected
                .min(self.pinned_list().len().saturating_sub(1));
            match self.config.save(&self.paths) {
                Ok(()) => self.set_status(StatusKind::Ok, &format!("{name} 从首页取下啦喵~")),
                Err(error) => {
                    self.set_status(StatusKind::Error, &format!("置顶改动没存住：{error}喵~"))
                }
            }
            return;
        }
        self.config.pinned_repos.push(name.clone());
        self.config.pinned_repos.truncate(24);
        if !self
            .pin_cache
            .iter()
            .any(|item| item.display_full_name().eq_ignore_ascii_case(&name))
        {
            self.pin_cache.push(repo);
        }
        match self.config.save(&self.paths) {
            Ok(()) => self.set_status(StatusKind::Ok, &format!("{name} 钉到首页啦喵~")),
            Err(error) => {
                self.set_status(StatusKind::Error, &format!("置顶改动没存住：{error}喵~"))
            }
        }
    }

    pub fn sync_pins(&mut self) {
        let pins = self.pinned_list();
        let names = self.config.pinned_repos.clone();
        for (index, name) in names.into_iter().enumerate() {
            if pins.get(index).map(|repo| repo.id).unwrap_or(0) == 0 {
                self.fetch_pin(name);
            }
        }
        self.home_selected = self.home_selected.min(pins.len().saturating_sub(1));
    }

    fn fetch_pin(&mut self, name: String) {
        if self
            .pin_loading
            .iter()
            .any(|item| item.eq_ignore_ascii_case(&name))
        {
            return;
        }
        self.pin_loading.push(name.clone());
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.repository(&name, false);
            let _ = tx.send(Msg::PinnedRepo { name, result });
        });
    }

    pub fn open_pin(&mut self, index: usize) {
        let pins = self.pinned_list();
        let Some(repo) = pins.get(index).cloned() else {
            return;
        };
        if repo.id != 0 {
            self.open_repo(repo);
            return;
        }
        if let Some(name) = self.config.pinned_repos.get(index).cloned() {
            self.fetch_pin(name.clone());
            self.set_status(StatusKind::Info, &format!("人家正在把 {name} 取回来喵~"));
        }
    }

    pub fn move_home_selection(&mut self, delta: i32) {
        let len = self.pinned_list().len();
        if len == 0 {
            self.home_selected = 0;
            return;
        }
        self.home_selected = (self.home_selected as i32 + delta).clamp(0, len as i32 - 1) as usize;
    }

    pub fn random_repo(&mut self) {
        let mut candidates = Vec::new();
        for page in [
            self.current_list_page(),
            Page::Trending,
            Page::Starred,
            Page::Repositories,
        ] {
            for repo in self.raw_list(page) {
                if !candidates
                    .iter()
                    .any(|item: &Repo| item.display_full_name() == repo.display_full_name())
                {
                    candidates.push(repo.clone());
                }
            }
            if !candidates.is_empty() {
                break;
            }
        }
        if candidates.is_empty() {
            self.set_status(StatusKind::Info, "仓库列表还空着，先按 r 刷新一下嘛喵~");
            return;
        }
        let seed = self.started_at.elapsed().subsec_nanos() as usize;
        let repo = candidates[seed % candidates.len()].clone();
        self.set_status(
            StatusKind::Info,
            &format!("命运选中了 {} 喵~", repo.display_full_name()),
        );
        self.open_repo(repo);
    }

    pub fn unread_notifications(&self) -> usize {
        self.notifications.iter().filter(|item| item.unread).count()
    }

    pub fn language_overview(&self) -> Vec<(String, usize)> {
        let mut counts: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for repo in self.repos.iter().chain(self.starred.iter()) {
            *counts.entry(repo.language_label().to_string()).or_insert(0) += 1;
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

    pub fn top_repos_by_stars(&self, limit: usize) -> Vec<Repo> {
        let mut repos: Vec<Repo> = Vec::new();
        for repo in self.repos.iter().chain(self.starred.iter()) {
            if !repos
                .iter()
                .any(|item| item.display_full_name() == repo.display_full_name())
            {
                repos.push(repo.clone());
            }
        }
        repos.sort_by_key(|left| std::cmp::Reverse(left.stargazers_count));
        repos.truncate(limit);
        repos
    }

    pub fn palette_items(&self) -> Vec<PaletteItem> {
        vec![
            PaletteItem {
                label: "首页".to_string(),
                hint: "总览 · 置顶 · 快捷入口".to_string(),
                action: Action::Navigate(Page::Home),
            },
            PaletteItem {
                label: "人家的仓库".to_string(),
                hint: "账号下的全部仓库".to_string(),
                action: Action::Navigate(Page::Repositories),
            },
            PaletteItem {
                label: "星星收藏".to_string(),
                hint: "收藏的仓库".to_string(),
                action: Action::Navigate(Page::Starred),
            },
            PaletteItem {
                label: "搜索仓库".to_string(),
                hint: "全局搜索".to_string(),
                action: Action::Navigate(Page::Search),
            },
            PaletteItem {
                label: "热门仓库".to_string(),
                hint: "按日 / 周 / 月涨星".to_string(),
                action: Action::Navigate(Page::Trending),
            },
            PaletteItem {
                label: "账号与额度".to_string(),
                hint: "资料 · API 额度".to_string(),
                action: Action::Navigate(Page::Account),
            },
            PaletteItem {
                label: "通知中心".to_string(),
                hint: "未读 / 全部通知".to_string(),
                action: Action::Navigate(Page::Notifications),
            },
            PaletteItem {
                label: "Gist 工作台".to_string(),
                hint: "7 · 自己的代码片段".to_string(),
                action: Action::Navigate(Page::Gists),
            },
            PaletteItem {
                label: "分支对比 / 建合并请求".to_string(),
                hint: "m · 比较分支，开 PR".to_string(),
                action: Action::Navigate(Page::Compare),
            },
            PaletteItem {
                label: "社交网络".to_string(),
                hint: "粉丝 · 关注 · 组织".to_string(),
                action: Action::OpenNetwork,
            },
            PaletteItem {
                label: "数据洞察".to_string(),
                hint: "语言 · 星标 · 缓存".to_string(),
                action: Action::Navigate(Page::Insights),
            },
            PaletteItem {
                label: "设置".to_string(),
                hint: "主题 · 代理 · 缓存".to_string(),
                action: Action::Navigate(Page::Settings),
            },
            PaletteItem {
                label: "仓库标签".to_string(),
                hint: "v · 查看 tag、复制名、下载源码包".to_string(),
                action: Action::OpenTags,
            },
            PaletteItem {
                label: "复制 SSH 克隆地址".to_string(),
                hint: "u · git@github.com 地址".to_string(),
                action: Action::CopySsh,
            },
            PaletteItem {
                label: "复制仓库全名".to_string(),
                hint: "N · owner/repo".to_string(),
                action: Action::CopyFullName,
            },
            PaletteItem {
                label: "复制发行版文件链接".to_string(),
                hint: "y · 当前附带文件的下载地址".to_string(),
                action: Action::CopyAssetUrl {
                    asset: self.release_asset_selected,
                },
            },
            PaletteItem {
                label: "随机探索一个仓库".to_string(),
                hint: "E · 打开命运选中的仓库".to_string(),
                action: Action::RandomRepo,
            },
            PaletteItem {
                label: "置顶 / 取消置顶当前仓库".to_string(),
                hint: "P · 固定到首页".to_string(),
                action: Action::TogglePin,
            },
            PaletteItem {
                label: "主题工作室".to_string(),
                hint: "T · 挑一套配色".to_string(),
                action: Action::OpenThemes,
            },
            PaletteItem {
                label: "切换下一套主题".to_string(),
                hint: "t · 立即换色".to_string(),
                action: Action::CycleTheme,
            },
            PaletteItem {
                label: "缓存管理".to_string(),
                hint: "c · 清理缓存".to_string(),
                action: Action::OpenCacheManager,
            },
            PaletteItem {
                label: "操作说明".to_string(),
                hint: "? · 全部按键".to_string(),
                action: Action::OpenHelp,
            },
            PaletteItem {
                label: "刷新当前页".to_string(),
                hint: "r · 重新取数据".to_string(),
                action: Action::Refresh,
            },
            PaletteItem {
                label: "保存设置".to_string(),
                hint: "Ctrl+S · 写入配置文件".to_string(),
                action: Action::SaveConfig,
            },
        ]
    }

    pub fn palette_filtered(&self) -> Vec<PaletteItem> {
        let query = self.palette_input.value.trim().to_lowercase();
        self.palette_items()
            .into_iter()
            .filter(|item| {
                query.is_empty()
                    || item.label.to_lowercase().contains(&query)
                    || item.hint.to_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn open_palette(&mut self) {
        self.palette_input = TextInput::new("");
        self.palette_selected = 0;
        self.overlay = Some(Overlay::Palette);
    }

    pub fn move_palette_selection(&mut self, delta: i32) {
        let len = self.palette_filtered().len();
        if len == 0 {
            self.palette_selected = 0;
            return;
        }
        self.palette_selected =
            (self.palette_selected as i32 + delta).clamp(0, len as i32 - 1) as usize;
    }

    pub fn activate_palette(&mut self) {
        let Some(item) = self.palette_filtered().get(self.palette_selected).cloned() else {
            return;
        };
        self.overlay = None;
        self.handle_action(item.action);
    }

    pub fn open_themes(&mut self) {
        self.theme_selected = self
            .themes
            .iter()
            .position(|flavor| *flavor == self.theme.flavor)
            .unwrap_or(0);
        self.overlay = Some(Overlay::Themes);
    }

    pub fn move_theme_selection(&mut self, delta: i32) {
        if self.themes.is_empty() {
            return;
        }
        self.theme_selected =
            (self.theme_selected as i32 + delta).clamp(0, self.themes.len() as i32 - 1) as usize;
    }

    pub fn choose_theme_selection(&mut self) {
        if let Some(flavor) = self.themes.get(self.theme_selected).cloned() {
            self.set_theme(flavor);
        }
    }

    pub fn go_back(&mut self) {
        if self.page == Page::Profile {
            self.close_profile();
            return;
        }
        if self.page == Page::IssueDetail {
            self.issue_comment_focus = false;
            self.page = Page::Issues;
            self.issue_open = None;
            self.issue_open_number = None;
            return;
        }
        if matches!(self.page, Page::Gists | Page::Network) {
            let target = if matches!(self.return_page, Page::Gists | Page::Network) {
                Page::Home
            } else {
                self.return_page
            };
            self.page = target;
            if let Some(index) = target.nav_index() {
                self.nav_offset = index;
            }
            return;
        }
        let Some(target) = back_target(self.page, self.return_page, self.page_repo.is_some())
        else {
            return;
        };
        self.search_focus = false;
        self.readme_fullscreen = false;
        if target == Page::Repo {
            self.page = Page::Repo;
            return;
        }
        self.page = target;
        self.page_repo = None;
        self.issue_repo = None;
        if let Some(index) = target.nav_index() {
            self.nav_offset = index;
        }
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
        self.search_filter = TextInput::new("");
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

    pub fn set_trending_period(&mut self, period: TrendingPeriod) {
        if self.trending_period == period {
            return;
        }
        self.trending_period = period;
        self.trending_selected = 0;
        self.list_offset = 0;
        self.fetch_trending(false);
        self.schedule_selected_detail();
    }

    pub fn toggle_readme_fullscreen(&mut self) {
        if self.page != Page::Repo || !self.config.show_readme {
            self.set_status(StatusKind::Info, "仓库页里打开 README 才能全屏看哦喵~");
            return;
        }
        self.readme_fullscreen = !self.readme_fullscreen;
        self.readme_scroll = 0;
        self.list_filter_focus = false;
        self.release_filter_focus = false;
    }

    pub fn set_language(&mut self, language: Option<String>) {
        self.language_filter = language;
        let page = self.current_list_page();
        self.set_selection_for(page, 0);
        self.list_offset = 0;
        self.overlay = None;
        if Self::is_list_page(page) {
            self.follow_selection();
        }
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

    pub fn open_user_profile(&mut self, login: String) {
        let login = login.trim().trim_start_matches('@').to_string();
        if login.is_empty() {
            self.set_status(StatusKind::Info, "这个作者没有留下名字喵~");
            return;
        }
        if self.page != Page::Profile {
            self.profile_return_page = if self.page.is_repo_page() {
                if self.return_page == Page::Profile {
                    Page::Repositories
                } else {
                    self.return_page
                }
            } else {
                self.page
            };
        }
        if self.profile_login.as_deref() != Some(login.as_str()) {
            self.profile_login = Some(login.clone());
            self.profile_user = None;
            self.profile_following = None;
            self.profile_follow_loading = None;
            self.profile_repos.clear();
            self.profile_selected = 0;
            self.profile_error = None;
        }
        self.page = Page::Profile;
        self.search_focus = false;
        self.list_filter_focus = false;
        self.release_filter_focus = false;
        self.readme_fullscreen = false;
        self.overlay = None;
        self.pending_detail = None;
        self.list_offset = 0;
        self.fetch_profile(login);
    }

    fn fetch_profile(&mut self, login: String) {
        self.profile_user_loading = true;
        self.profile_repos_loading = true;
        self.profile_error = None;
        self.set_status(
            StatusKind::Info,
            &format!("人家正在翻 @{login} 的主页喵~..."),
        );
        let tx = self.tx.clone();
        let client = self.client.clone();
        let user_login = login.clone();
        std::thread::spawn(move || {
            let result = client.user_profile(&user_login);
            let _ = tx.send(Msg::ProfileUser {
                login: user_login,
                result,
            });
        });
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.user_repositories(&login);
            let _ = tx.send(Msg::ProfileRepos { login, result });
        });
    }

    fn close_profile(&mut self) {
        let target = match self.profile_return_page {
            Page::Profile => Page::Repositories,
            page => page,
        };
        let target = if target.is_repo_page() {
            Page::Repositories
        } else {
            target
        };
        self.page = target;
        self.page_repo = None;
        self.issue_repo = None;
        self.pending_detail = None;
        self.search_focus = false;
        self.list_filter_focus = false;
        self.release_filter_focus = false;
        self.readme_fullscreen = false;
        self.list_offset = 0;
        if Self::is_list_page(target) {
            self.follow_selection();
            self.schedule_selected_detail();
        }
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

    pub fn copy_ssh_clone(&mut self) {
        let Some(repo) = self.effective_repo() else {
            self.set_status(StatusKind::Info, "还没有选中仓库呢喵~");
            return;
        };
        let url = if !repo.ssh_url.is_empty() {
            repo.ssh_url.clone()
        } else {
            format!("git@github.com:{}.git", repo.display_full_name())
        };
        self.copy_text(&url, "SSH 克隆地址喵~");
    }

    pub fn copy_full_name(&mut self) {
        let Some(repo) = self.effective_repo() else {
            self.set_status(StatusKind::Info, "还没有选中仓库呢喵~");
            return;
        };
        self.copy_text(&repo.display_full_name(), "仓库全名喵~");
    }

    pub fn copy_selected_asset_url(&mut self) {
        if self.page != Page::ReleaseDetail {
            self.set_status(StatusKind::Info, "先打开一个发行版详情页嘛喵~");
            return;
        }
        let Some(release) = self.selected_release() else {
            self.set_status(StatusKind::Info, "先挑一个发行版嘛喵~");
            return;
        };
        let Some(asset) = release.assets.get(self.release_asset_selected) else {
            self.set_status(StatusKind::Info, "这个发行版没有附带文件喵~");
            return;
        };
        self.copy_text(&asset.browser_download_url, "文件下载地址喵~");
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

    pub fn open_fork_repo(&mut self) {
        let Some(repo) = self.effective_repo() else {
            self.set_status(StatusKind::Info, "先选一个仓库，人家才知道 Fork 哪个喵~");
            return;
        };
        if self.is_own_repo(&repo) {
            self.set_status(StatusKind::Info, "这是你自己的仓库，不用再 Fork 啦喵~");
            return;
        }
        if !self.client.token_present() {
            self.set_status(StatusKind::Error, "还没登录呢，人家 Fork 不了哦喵~");
            return;
        }
        if self.fork_loading.is_some() {
            self.set_status(
                StatusKind::Info,
                "上一个 Fork 还在路上呢，等人家一下下嘛喵~",
            );
            return;
        }
        self.fork_repo = Some(repo);
        self.overlay = Some(Overlay::ForkRepo);
    }

    pub fn confirm_fork_repo(&mut self) {
        let Some(repo) = self.fork_repo.clone() else {
            return;
        };
        if self.fork_loading.is_some() {
            return;
        }
        let name = repo.display_full_name();
        self.fork_loading = Some(name.clone());
        self.set_status(
            StatusKind::Info,
            &format!("人家正在把 {name} Fork 到你名下喵~..."),
        );
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.fork_repository(&name);
            let _ = tx.send(Msg::RepoForked { name, result });
        });
    }

    pub fn open_delete_repo(&mut self) {
        let Some(repo) = self.effective_repo() else {
            self.set_status(StatusKind::Info, "先选一个仓库，人家才知道删哪个喵~");
            return;
        };
        if !self.is_own_repo(&repo) {
            self.set_status(StatusKind::Info, "只能删除自己的仓库哦喵~");
            return;
        }
        if !self.client.token_present() {
            self.set_status(StatusKind::Error, "还没登录呢，人家删不了仓库哦喵~");
            return;
        }
        if self.delete_loading.is_some() {
            self.set_status(StatusKind::Info, "上一个删除还在路上呢，等人家一下下嘛喵~");
            return;
        }
        self.delete_repo = Some(repo);
        self.overlay = Some(Overlay::DeleteRepo);
    }

    pub fn confirm_delete_repo(&mut self) {
        let Some(repo) = self.delete_repo.clone() else {
            return;
        };
        if !self.is_own_repo(&repo) {
            self.set_status(StatusKind::Error, "这个仓库不是你的，删不了哦喵~");
            return;
        }
        if self.delete_loading.is_some() {
            return;
        }
        let name = repo.display_full_name();
        self.delete_loading = Some(name.clone());
        self.set_status(
            StatusKind::Error,
            &format!("人家正在删除 {name}，这个操作不能撤销喵~..."),
        );
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.delete_repository(&name);
            let _ = tx.send(Msg::RepoDeleted { name, result });
        });
    }

    fn finish_deleted_repo(&mut self, name: &str) {
        for list in [
            &mut self.repos,
            &mut self.starred,
            &mut self.search_results,
            &mut self.trending,
            &mut self.profile_repos,
        ] {
            list.retain(|repo| repo.display_full_name() != name);
        }
        if let Some(login) = self.login.clone() {
            let _ = self.cache.save(&repos_cache_key(&login), &self.repos);
            let _ = self.cache.save(&starred_cache_key(&login), &self.starred);
        }
        let key = key_part(name);
        let prefixes = [
            format!("repo_{key}"),
            format!("releases_{key}"),
            format!("tree_{key}_"),
            format!("readme_{key}_"),
            format!("branches_{key}"),
            format!("commits_{key}_"),
            format!("actions_{key}"),
        ];
        let prefixes = prefixes.iter().map(String::as_str).collect::<Vec<_>>();
        let _ = self.cache.clear_prefixes(&prefixes);
        self.refresh_cache_stats();
        let current = self
            .page_repo
            .as_ref()
            .is_some_and(|repo| repo.display_full_name() == name)
            || self
                .detail
                .as_ref()
                .is_some_and(|repo| repo.display_full_name() == name);
        if current {
            self.page_repo = None;
            self.detail = None;
            self.detail_for = None;
            self.detail_fetched_at = None;
            self.releases.clear();
            self.releases_fetched_at = None;
            self.tree = None;
            self.readme = None;
            self.branches.clear();
            self.commits.clear();
            self.go_back();
        }
        self.clamp_selection();
        self.set_status(StatusKind::Ok, &format!("{name} 已经从 GitHub 删除喵~"));
    }

    pub fn open_external_url(&mut self, url: &str) {
        self.open_url(url);
    }

    pub fn open_tree_path(&mut self, path: &str, directory: bool) {
        if !directory {
            self.open_file(path);
            return;
        }
        let Some(repo) = self.page_repo.clone().or_else(|| self.selected_repo()) else {
            return;
        };
        let branch = if self.tree_branch.is_empty() {
            repo.default_branch.clone()
        } else {
            self.tree_branch.clone()
        };
        let branch = if branch.is_empty() {
            "main".to_string()
        } else {
            branch
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
        if self.overlay == Some(Overlay::RepoLanguages) {
            let viewport = self.repo_language_viewport.max(1);
            let max = self.repo_languages.len().saturating_sub(viewport);
            let next = self.repo_language_offset as i32 + delta;
            self.repo_language_offset = next.clamp(0, max as i32) as usize;
            return;
        }
        if self.overlay == Some(Overlay::Languages) {
            let choices = self.language_choices().len();
            let viewport = self.language_viewport.max(1);
            let max = choices.saturating_sub(viewport);
            let next = self.language_offset as i32 + delta;
            self.language_offset = next.clamp(0, max as i32) as usize;
            return;
        }
        if self.overlay == Some(Overlay::Branches) {
            let len = self.branches.len();
            let viewport = self.branch_viewport.max(1);
            let max = len.saturating_sub(viewport);
            let next = self.branch_offset as i32 + delta;
            self.branch_offset = next.clamp(0, max as i32) as usize;
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
                self.settings_row = (self.settings_row + 1).min(11);
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
        if self.page == Page::IssueDetail {
            if delta != 0 {
                self.issue_scroll_by(3 * delta.signum());
            }
            return;
        }
        if self.page == Page::Gists {
            if layout.contains(layout.list, x, y) && delta != 0 {
                self.move_gist_selection(delta.signum());
            } else if delta != 0 {
                self.gist_scroll_by(3 * delta.signum());
            }
            return;
        }
        if self.page == Page::Compare {
            if layout.contains(layout.list, x, y) && delta != 0 {
                self.move_compare_selection(delta.signum());
            } else if delta != 0 {
                self.compare_scroll_by(3 * delta.signum());
            }
            return;
        }
        if self.page == Page::Files {
            if delta != 0 {
                self.file_scroll_by(3 * delta.signum());
            }
            return;
        }
        if self.page == Page::Network {
            if delta != 0 {
                self.move_network_selection(delta.signum());
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

    // ---- Gist ----
    pub fn open_gists(&mut self) {
        if self.page == Page::Gists {
            self.fetch_gists(true);
            self.schedule_gist_detail(true);
            return;
        }
        self.return_page = return_page_when_opening(self.page, self.return_page);
        self.page = Page::Gists;
        self.search_focus = false;
        self.list_filter_focus = false;
        self.release_filter_focus = false;
        self.readme_fullscreen = false;
        if let Some(cached) = self.cache.load::<Vec<Gist>>("gists") {
            self.gists = cached.data;
            self.gists_fetched_at = Some(cached.fetched_at);
        }
        self.clamp_gist_selection();
        self.fetch_gists(false);
        self.schedule_gist_detail(false);
    }

    pub fn selected_gist(&self) -> Option<Gist> {
        self.gists.get(self.gist_selected).cloned()
    }

    fn clamp_gist_selection(&mut self) {
        self.gist_selected = self.gist_selected.min(self.gists.len().saturating_sub(1));
    }

    fn fetch_gists(&mut self, force: bool) {
        if self.gists_loading {
            return;
        }
        let fresh = self
            .gists_fetched_at
            .map(|at| now_epoch() - at <= self.config.detail_ttl_secs as i64)
            .unwrap_or(false);
        if !force && fresh && !self.gists.is_empty() {
            return;
        }
        self.gists_loading = true;
        if self.gists.is_empty() {
            self.set_status(StatusKind::Info, "人家正在搬 Gist 喵~...");
        }
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.gists();
            let _ = tx.send(Msg::Gists { result });
        });
    }

    fn schedule_gist_detail(&mut self, force: bool) {
        let Some(gist) = self.selected_gist() else {
            self.gist_detail = None;
            self.gist_detail_for = None;
            return;
        };
        let id = gist.id.clone();
        if self.gist_detail_for.as_deref() == Some(id.as_str()) && !force {
            return;
        }
        if self.gist_detail_loading.as_deref() == Some(id.as_str()) {
            return;
        }
        if self.gist_detail_for.as_deref() != Some(id.as_str()) {
            self.gist_detail = None;
            self.gist_file_selected = 0;
            self.gist_scroll = 0;
        }
        self.gist_detail_loading = Some(id.clone());
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.gist(&id);
            let _ = tx.send(Msg::GistDetail { id, result });
        });
    }

    pub fn select_gist(&mut self, index: usize) {
        if self.gists.is_empty() {
            return;
        }
        let next = index.min(self.gists.len() - 1);
        if next != self.gist_selected {
            self.gist_selected = next;
            self.gist_scroll = 0;
            self.schedule_gist_detail(false);
        }
    }

    pub fn move_gist_selection(&mut self, delta: i32) {
        if self.gists.is_empty() {
            return;
        }
        let next = (self.gist_selected as i32 + delta).clamp(0, self.gists.len() as i32 - 1);
        self.select_gist(next as usize);
    }

    fn gist_source(&self) -> Option<&Gist> {
        self.gist_detail
            .as_ref()
            .or_else(|| self.gists.get(self.gist_selected))
    }

    pub fn gist_file_names(&self) -> Vec<String> {
        self.gist_source().map(Gist::file_names).unwrap_or_default()
    }

    pub fn current_gist_file(&self) -> Option<crate::model::GistFile> {
        let names = self.gist_file_names();
        let name = names.get(self.gist_file_selected)?;
        self.gist_source()
            .and_then(|gist| gist.files.get(name).cloned())
    }

    pub fn cycle_gist_file(&mut self) {
        let count = self.gist_file_names().len();
        if count == 0 {
            return;
        }
        self.gist_file_selected = (self.gist_file_selected + 1) % count;
        self.gist_scroll = 0;
    }

    pub fn gist_scroll_by(&mut self, delta: i32) {
        self.gist_scroll =
            (self.gist_scroll as i32 + delta).clamp(0, self.gist_scroll_max as i32) as u16;
    }

    pub fn open_gist_browser(&mut self) {
        if let Some(gist) = self.gist_detail.clone().or_else(|| self.selected_gist()) {
            self.open_url(&gist.html_url);
        }
    }

    pub fn copy_gist_url(&mut self) {
        if let Some(gist) = self.gist_detail.clone().or_else(|| self.selected_gist()) {
            self.copy_text(&gist.html_url, "Gist 地址喵~");
        }
    }

    pub fn open_new_gist(&mut self) {
        if !self.client.token_present() {
            self.set_status(StatusKind::Info, "还没有登录 GitHub，建不了 Gist 哦喵~");
            return;
        }
        self.overlay = Some(Overlay::NewGist);
        self.gist_desc_input = TextInput::new("");
        self.gist_filename_input = TextInput::new("gistfile1.txt");
        self.gist_content_input = TextInput::new("");
        self.gist_field = GistField::Description;
        self.gist_public = false;
        self.gist_saving = false;
        self.gist_create_error = None;
    }

    fn active_gist_input(&mut self) -> &mut TextInput {
        match self.gist_field {
            GistField::Description => &mut self.gist_desc_input,
            GistField::Filename => &mut self.gist_filename_input,
            GistField::Content => &mut self.gist_content_input,
        }
    }

    fn next_gist_field(&mut self, forward: bool) {
        self.gist_field = match (self.gist_field, forward) {
            (GistField::Description, true) => GistField::Filename,
            (GistField::Filename, true) => GistField::Content,
            (GistField::Content, true) => GistField::Description,
            (GistField::Description, false) => GistField::Content,
            (GistField::Filename, false) => GistField::Description,
            (GistField::Content, false) => GistField::Filename,
        };
    }

    pub fn handle_new_gist_key(&mut self, key: KeyEvent) {
        if self.gist_saving {
            return;
        }
        match key.code {
            KeyCode::Esc => {
                self.overlay = None;
                self.gist_create_error = None;
            }
            KeyCode::Tab => self.next_gist_field(true),
            KeyCode::BackTab => self.next_gist_field(false),
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.gist_public = !self.gist_public;
            }
            KeyCode::Enter => {
                if self.gist_field == GistField::Content {
                    self.gist_content_input.insert('\n');
                } else {
                    self.next_gist_field(true);
                }
            }
            KeyCode::Backspace => self.active_gist_input().backspace(),
            KeyCode::Delete => self.active_gist_input().delete(),
            KeyCode::Left => self.active_gist_input().left(),
            KeyCode::Right => self.active_gist_input().right(),
            KeyCode::Home => self.active_gist_input().home(),
            KeyCode::End => self.active_gist_input().end(),
            KeyCode::Char(ch) if !ch.is_control() => self.active_gist_input().insert(ch),
            _ => {}
        }
    }

    pub fn submit_new_gist(&mut self) {
        if self.gist_saving {
            return;
        }
        let filename = self.gist_filename_input.value.trim().to_string();
        let content = self.gist_content_input.value.clone();
        if filename.is_empty() {
            self.gist_field = GistField::Filename;
            self.gist_create_error = Some("文件名不能空着喵~".to_string());
            return;
        }
        if content.trim().is_empty() {
            self.gist_field = GistField::Content;
            self.gist_create_error = Some("内容不能空着喵~".to_string());
            return;
        }
        self.gist_saving = true;
        self.gist_create_error = None;
        self.set_status(StatusKind::Info, "正在把 Gist 送去 GitHub 喵~...");
        let description = self.gist_desc_input.value.clone();
        let public = self.gist_public;
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.create_gist(&description, &filename, &content, public);
            let _ = tx.send(Msg::GistCreated { result });
        });
    }

    pub fn open_delete_gist(&mut self) {
        let Some(gist) = self.selected_gist() else {
            return;
        };
        self.gist_delete = Some(gist);
        self.gist_deleting = false;
        self.overlay = Some(Overlay::DeleteGist);
    }

    pub fn confirm_delete_gist(&mut self) {
        if self.gist_deleting {
            return;
        }
        let Some(gist) = self.gist_delete.clone() else {
            return;
        };
        self.gist_deleting = true;
        let id = gist.id.clone();
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.delete_gist(&id);
            let _ = tx.send(Msg::GistDeleted { id, result });
        });
    }

    // ---- 分支对比 / 新建合并请求 ----
    pub fn open_compare(&mut self) {
        let Some(repo) = self.context_repo() else {
            self.set_status(StatusKind::Info, "先挑一个仓库嘛喵~");
            return;
        };
        let name = repo.display_full_name();
        let changed = self
            .compare_repo
            .as_ref()
            .map(|current| current.display_full_name() != name)
            .unwrap_or(true);
        self.return_page = return_page_when_opening(self.page, self.return_page);
        self.compare_repo = Some(repo.clone());
        self.page = Page::Compare;
        self.search_focus = false;
        self.list_filter_focus = false;
        self.release_filter_focus = false;
        self.readme_fullscreen = false;
        if changed {
            self.compare_base = repo.default_branch.clone();
            self.compare_head =
                if self.tree_branch.is_empty() || self.tree_branch == self.compare_base {
                    repo.default_branch.clone()
                } else {
                    self.tree_branch.clone()
                };
            self.compare_result = None;
            self.compare_error = None;
            self.compare_selected = 0;
            self.compare_scroll = 0;
            self.compare_fetched_for = None;
            self.compare_view = CompareView::Commits;
            self.pull_created = None;
        }
        self.branch_pick_side = None;
        if self.branches.is_empty() {
            self.load_branches_cache(&repo);
        }
        self.fetch_branches(false);
        self.fetch_compare();
    }

    pub fn fetch_compare(&mut self) {
        if self.compare_loading {
            return;
        }
        let Some(repo) = self.compare_repo.clone() else {
            return;
        };
        let base = self.compare_base.trim().to_string();
        let head = self.compare_head.trim().to_string();
        if base.is_empty() || head.is_empty() {
            self.set_status(StatusKind::Info, "先选好 base 和 head 分支嘛喵~");
            return;
        }
        let name = repo.display_full_name();
        let private = repo.private;
        self.compare_loading = true;
        self.compare_error = None;
        self.set_status(StatusKind::Info, "人家正在比较这两个分支喵~...");
        let tx = self.tx.clone();
        let client = self.client.clone();
        let sent_base = base.clone();
        let sent_head = head.clone();
        std::thread::spawn(move || {
            let result = client.compare(&name, &sent_base, &sent_head, private);
            let _ = tx.send(Msg::Compare {
                name,
                base: sent_base,
                head: sent_head,
                result,
            });
        });
    }

    pub fn compare_row_count(&self) -> usize {
        let Some(result) = self.compare_result.as_ref() else {
            return 0;
        };
        match self.compare_view {
            CompareView::Commits => result.commits.len(),
            CompareView::Files => result.files.len(),
        }
    }

    pub fn move_compare_selection(&mut self, delta: i32) {
        let count = self.compare_row_count();
        if count == 0 {
            self.compare_selected = 0;
            return;
        }
        self.compare_selected =
            (self.compare_selected as i32 + delta).clamp(0, count as i32 - 1) as usize;
        self.compare_scroll = 0;
    }

    pub fn select_compare_row(&mut self, index: usize) {
        let count = self.compare_row_count();
        if count == 0 {
            return;
        }
        self.compare_selected = index.min(count - 1);
        self.compare_scroll = 0;
    }

    pub fn toggle_compare_view(&mut self) {
        self.compare_view = match self.compare_view {
            CompareView::Commits => CompareView::Files,
            CompareView::Files => CompareView::Commits,
        };
        self.compare_selected = 0;
        self.compare_scroll = 0;
    }

    pub fn compare_scroll_by(&mut self, delta: i32) {
        self.compare_scroll =
            (self.compare_scroll as i32 + delta).clamp(0, self.compare_scroll_max as i32) as u16;
    }

    pub fn focus_compare_side(&mut self, side: CompareSide) {
        self.compare_focus = side;
    }

    pub fn open_compare_branches(&mut self, side: CompareSide) {
        let Some(repo) = self.compare_repo.clone() else {
            return;
        };
        self.compare_focus = side;
        if self.branches.is_empty() {
            self.load_branches_cache(&repo);
        }
        let current = match side {
            CompareSide::Base => self.compare_base.clone(),
            CompareSide::Head => self.compare_head.clone(),
        };
        self.branch_selected = self
            .branches
            .iter()
            .position(|branch| branch.name == current)
            .unwrap_or(0);
        self.branch_offset = self
            .branch_selected
            .saturating_sub(self.branch_viewport.max(1) - 1);
        self.branch_pick_side = Some(side);
        self.overlay = Some(Overlay::Branches);
        self.fetch_branches(false);
    }

    pub fn set_compare_branch(&mut self, side: CompareSide, branch: String) {
        if branch.is_empty() {
            return;
        }
        self.overlay = None;
        self.branch_pick_side = None;
        match side {
            CompareSide::Base => self.compare_base = branch,
            CompareSide::Head => self.compare_head = branch,
        }
        self.compare_result = None;
        self.compare_error = None;
        self.compare_fetched_for = None;
        self.compare_selected = 0;
        self.fetch_compare();
    }

    pub fn start_new_pull(&mut self) {
        if self.compare_repo.is_none() {
            return;
        }
        if self.compare_base.trim().is_empty() || self.compare_head.trim().is_empty() {
            self.set_status(StatusKind::Info, "先选好 base 和 head 分支嘛喵~");
            return;
        }
        if self.compare_base == self.compare_head {
            self.set_status(
                StatusKind::Info,
                "base 和 head 是同一个分支，没法建合并请求喵~",
            );
            return;
        }
        if !self.client.token_present() {
            self.set_status(StatusKind::Info, "还没有登录 GitHub，建不了合并请求哦喵~");
            return;
        }
        self.overlay = Some(Overlay::NewPull);
        let title = self
            .compare_result
            .as_ref()
            .and_then(|result| result.commits.last())
            .map(|commit| commit.summary().to_string())
            .filter(|summary| !summary.is_empty())
            .unwrap_or_else(|| format!("合并 {} 到 {}", self.compare_head, self.compare_base));
        self.pull_title = TextInput::new(title);
        self.pull_body = TextInput::new("");
        self.pull_draft = false;
        self.pull_field = PullField::Title;
        self.pull_saving = false;
        self.pull_error = None;
    }

    pub fn handle_new_pull_key(&mut self, key: KeyEvent) {
        if self.pull_saving {
            return;
        }
        match key.code {
            KeyCode::Esc => {
                self.overlay = None;
                self.pull_error = None;
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.pull_field = match self.pull_field {
                    PullField::Title => PullField::Body,
                    PullField::Body => PullField::Title,
                };
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.pull_draft = !self.pull_draft;
            }
            KeyCode::Enter => {
                if self.pull_field == PullField::Body {
                    self.pull_body.insert('\n');
                } else {
                    self.pull_field = PullField::Body;
                }
            }
            KeyCode::Backspace => {
                if self.pull_field == PullField::Title {
                    self.pull_title.backspace();
                } else {
                    self.pull_body.backspace();
                }
            }
            KeyCode::Delete => {
                if self.pull_field == PullField::Title {
                    self.pull_title.delete();
                } else {
                    self.pull_body.delete();
                }
            }
            KeyCode::Left => {
                if self.pull_field == PullField::Title {
                    self.pull_title.left();
                } else {
                    self.pull_body.left();
                }
            }
            KeyCode::Right => {
                if self.pull_field == PullField::Title {
                    self.pull_title.right();
                } else {
                    self.pull_body.right();
                }
            }
            KeyCode::Home => {
                if self.pull_field == PullField::Title {
                    self.pull_title.home();
                } else {
                    self.pull_body.home();
                }
            }
            KeyCode::End => {
                if self.pull_field == PullField::Title {
                    self.pull_title.end();
                } else {
                    self.pull_body.end();
                }
            }
            KeyCode::Char(ch) if !ch.is_control() => {
                if self.pull_field == PullField::Title {
                    self.pull_title.insert(ch);
                } else {
                    self.pull_body.insert(ch);
                }
            }
            _ => {}
        }
    }

    pub fn submit_new_pull(&mut self) {
        if self.pull_saving {
            return;
        }
        let Some(repo) = self.compare_repo.clone() else {
            return;
        };
        let title = self.pull_title.value.trim().to_string();
        if title.is_empty() {
            self.pull_field = PullField::Title;
            self.pull_error = Some("标题不能空着喵~".to_string());
            return;
        }
        let body = self.pull_body.value.clone();
        let base = self.compare_base.clone();
        let head = self.compare_head.clone();
        let draft = self.pull_draft;
        let name = repo.display_full_name();
        self.pull_saving = true;
        self.pull_error = None;
        self.set_status(StatusKind::Info, "正在把合并请求送去 GitHub 喵~...");
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.create_pull_request(&name, &title, &body, &head, &base, draft);
            let _ = tx.send(Msg::PullCreated { name, result });
        });
    }

    // ---- 文件预览 ----
    pub fn open_file(&mut self, path: &str) {
        let Some(repo) = self.page_repo.clone().or_else(|| self.selected_repo()) else {
            return;
        };
        let branch = if self.tree_branch.trim().is_empty() {
            repo.default_branch.clone()
        } else {
            self.tree_branch.clone()
        };
        self.return_page = return_page_when_opening(self.page, self.return_page);
        self.file_repo = Some(repo);
        self.file_path = path.trim_start_matches('/').to_string();
        self.file_branch = if branch.is_empty() {
            "main".to_string()
        } else {
            branch
        };
        self.file_view = None;
        self.file_error = None;
        self.file_loading = None;
        self.file_scroll = 0;
        self.file_h_scroll = 0;
        self.page = Page::Files;
        self.search_focus = false;
        self.list_filter_focus = false;
        self.release_filter_focus = false;
        self.readme_fullscreen = false;
        self.fetch_file_content();
    }

    fn fetch_file_content(&mut self) {
        let Some(repo) = self.file_repo.clone() else {
            return;
        };
        if self.file_loading.as_deref() == Some(self.file_path.as_str()) {
            return;
        }
        let name = repo.display_full_name();
        let path = self.file_path.clone();
        let branch = self.file_branch.clone();
        let private = repo.private;
        self.file_loading = Some(path.clone());
        self.file_error = None;
        self.set_status(
            StatusKind::Info,
            &format!("人家正在读 {} 喵~...", format::clean_text(&path)),
        );
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.file_content(&name, &path, &branch, private);
            let _ = tx.send(Msg::FileContent {
                name,
                path,
                branch,
                result,
            });
        });
    }

    pub fn refresh_file(&mut self) {
        self.fetch_file_content();
    }

    pub fn file_scroll_by(&mut self, delta: i32) {
        self.file_scroll =
            (self.file_scroll as i32 + delta).clamp(0, self.file_scroll_max as i32) as u16;
    }

    pub fn file_scroll_x(&mut self, delta: i32) {
        self.file_h_scroll =
            (self.file_h_scroll as i32 + delta).clamp(0, self.file_h_max as i32) as u16;
    }

    pub fn toggle_file_wrap(&mut self) {
        self.file_wrap = !self.file_wrap;
        self.file_scroll = 0;
        self.file_h_scroll = 0;
        self.set_status(
            StatusKind::Info,
            if self.file_wrap {
                "自动换行打开啦喵~"
            } else {
                "自动换行关掉啦，用 h/l 横着看喵~"
            },
        );
    }

    pub fn copy_file_path(&mut self) {
        if !self.file_path.is_empty() {
            let path = self.file_path.clone();
            self.copy_text(&path, "文件路径喵~");
        }
    }

    pub fn download_file(&mut self) {
        let Some(repo) = self.file_repo.clone() else {
            return;
        };
        let Some(file) = self.file_view.clone() else {
            return;
        };
        let Some(url) = file.download_url.clone() else {
            self.set_status(StatusKind::Error, "这个文件没有下载地址喵~");
            return;
        };
        let filename = file
            .path
            .rsplit('/')
            .next()
            .unwrap_or("download.bin")
            .to_string();
        self.start_download(
            format!("文件 {} 喵~", format::clean_text(&file.path)),
            url,
            repo.private,
            filename,
        );
    }

    pub fn open_file_browser(&mut self) {
        let Some(file) = self.file_view.as_ref() else {
            return;
        };
        let url = file.html_url.clone();
        self.open_url(&url);
    }

    // ---- 社交网络 ----
    pub fn open_network(&mut self, login: Option<String>) {
        if self.settings_only {
            return;
        }
        let login = login
            .or_else(|| {
                (self.page == Page::Profile)
                    .then(|| self.profile_login.clone())
                    .flatten()
            })
            .or_else(|| self.login.clone());
        let Some(login) = login else {
            self.set_status(StatusKind::Info, "还没有登录，也不知道要看谁的朋友圈喵~");
            return;
        };
        let changed = self.network_login.as_deref() != Some(login.as_str());
        self.return_page = return_page_when_opening(self.page, self.return_page);
        self.network_login = Some(login);
        self.page = Page::Network;
        if changed {
            self.network_tab = NetworkTab::Followers;
            self.network_users.clear();
            self.network_orgs.clear();
            self.network_selected = 0;
            self.network_following = None;
            self.network_known_following.clear();
            self.network_details.clear();
        }
        self.fetch_network(false);
    }

    fn fetch_network(&mut self, force: bool) {
        let Some(login) = self.network_login.clone() else {
            return;
        };
        let tab = self.network_tab;
        if self.network_loading == Some(tab) {
            return;
        }
        if !force {
            let populated = match tab {
                NetworkTab::Orgs => !self.network_orgs.is_empty(),
                _ => !self.network_users.is_empty(),
            };
            if populated {
                return;
            }
        }
        self.network_loading = Some(tab);
        let tx = self.tx.clone();
        let client = self.client.clone();
        match tab {
            NetworkTab::Orgs => {
                std::thread::spawn(move || {
                    let result = client.organizations(&login);
                    let _ = tx.send(Msg::NetworkOrgs { login, result });
                });
            }
            NetworkTab::Followers | NetworkTab::Following => {
                let sent_login = login.clone();
                std::thread::spawn(move || {
                    let result = match tab {
                        NetworkTab::Followers => client.followers(&sent_login),
                        _ => client.following(&sent_login),
                    };
                    let _ = tx.send(Msg::NetworkUsers {
                        login: sent_login,
                        tab,
                        result,
                    });
                });
            }
        }
    }

    pub fn set_network_tab(&mut self, tab: NetworkTab) {
        if self.network_tab == tab {
            return;
        }
        let previous = self.network_tab;
        self.network_tab = tab;
        self.network_selected = 0;
        self.network_following = None;
        let users_swapped = matches!(previous, NetworkTab::Followers | NetworkTab::Following)
            && matches!(tab, NetworkTab::Followers | NetworkTab::Following);
        self.fetch_network(users_swapped);
    }

    pub fn cycle_network_tab(&mut self) {
        self.set_network_tab(self.network_tab.next());
    }

    pub fn network_row_count(&self) -> usize {
        match self.network_tab {
            NetworkTab::Orgs => self.network_orgs.len(),
            _ => self.network_users.len(),
        }
    }

    pub fn move_network_selection(&mut self, delta: i32) {
        let count = self.network_row_count();
        if count == 0 {
            self.network_selected = 0;
            return;
        }
        let next = (self.network_selected as i32 + delta).clamp(0, count as i32 - 1);
        if next as usize != self.network_selected {
            self.network_selected = next as usize;
            self.network_following = None;
        }
    }

    pub fn select_network_row(&mut self, index: usize) {
        let count = self.network_row_count();
        if count == 0 {
            return;
        }
        let next = index.min(count - 1);
        if next != self.network_selected {
            self.network_selected = next;
            self.network_following = None;
        }
    }

    pub fn selected_network_login(&self) -> Option<String> {
        match self.network_tab {
            NetworkTab::Orgs => self
                .network_orgs
                .get(self.network_selected)
                .map(|org| org.login.clone()),
            _ => self
                .network_users
                .get(self.network_selected)
                .map(|user| user.login.clone()),
        }
        .filter(|login| !login.is_empty())
    }

    pub fn open_selected_network_profile(&mut self) {
        if let Some(login) = self.selected_network_login() {
            self.open_user_profile(login);
        }
    }

    pub fn open_selected_network_browser(&mut self) {
        let login = self.selected_network_login();
        let url = match self.network_tab {
            NetworkTab::Orgs => self
                .network_orgs
                .get(self.network_selected)
                .map(|org| org.html_url.clone()),
            _ => self
                .network_users
                .get(self.network_selected)
                .map(|user| user.html_url.clone()),
        };
        if let Some(url) = url.filter(|url| !url.is_empty()) {
            self.open_url(&url);
        } else if let Some(login) = login {
            self.open_url(&format!("https://github.com/{login}"));
        }
    }

    pub fn toggle_follow(&mut self) {
        if self.network_tab == NetworkTab::Orgs {
            self.set_status(StatusKind::Info, "组织不能关注喵~");
            return;
        }
        let Some(login) = self.selected_network_login() else {
            self.set_status(StatusKind::Info, "先挑一个用户嘛喵~");
            return;
        };
        if self.network_follow_loading.is_some() {
            return;
        }
        let following = self.network_following.or_else(|| {
            self.network_known_following
                .contains(&login)
                .then_some(true)
        });
        if let Some(following) = following {
            self.apply_follow(login, !following);
            return;
        }
        self.network_follow_loading = Some(login.clone());
        self.set_status(StatusKind::Info, "人家正在看你们的关注关系喵~...");
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.is_following(&login);
            let _ = tx.send(Msg::FollowChecked { login, result });
        });
    }

    pub fn apply_follow(&mut self, login: String, following: bool) {
        self.network_follow_loading = Some(login.clone());
        self.set_status(
            StatusKind::Info,
            if following {
                "正在关注喵~..."
            } else {
                "正在取消关注喵~..."
            },
        );
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.set_following(&login, following);
            let _ = tx.send(Msg::FollowToggled {
                login,
                following,
                result,
            });
        });
    }

    pub fn toggle_profile_follow(&mut self) {
        let Some(login) = self.profile_login.clone() else {
            self.set_status(StatusKind::Info, "还没有打开任何人的主页喵~");
            return;
        };
        if self.login.as_deref() == Some(login.as_str()) {
            self.set_status(StatusKind::Info, "这是你自己的主页，不能关注自己喵~");
            return;
        }
        if !self.client.token_present() {
            self.set_status(StatusKind::Error, "还没登录呢，人家关注不了哦喵~");
            return;
        }
        if self.profile_follow_loading.is_some() {
            return;
        }
        if let Some(following) = self.profile_following {
            self.apply_profile_follow(login, !following);
            return;
        }
        self.profile_follow_loading = Some(login.clone());
        self.set_status(StatusKind::Info, "人家正在看你们的关注关系喵~...");
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.is_following(&login);
            let _ = tx.send(Msg::ProfileFollowChecked { login, result });
        });
    }

    fn apply_profile_follow(&mut self, login: String, following: bool) {
        self.profile_follow_loading = Some(login.clone());
        self.set_status(
            StatusKind::Info,
            if following {
                "正在关注喵~..."
            } else {
                "正在取消关注喵~..."
            },
        );
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.set_following(&login, following);
            let _ = tx.send(Msg::ProfileFollowToggled {
                login,
                following,
                result,
            });
        });
    }

    // ---- 议题详情 / 评论 ----
    pub fn open_issue_detail(&mut self) {
        let Some(issue) = self.selected_issue() else {
            self.set_status(StatusKind::Info, "先挑一个议题嘛喵~");
            return;
        };
        if self.issue_repo.is_none() {
            self.set_status(StatusKind::Info, "先打开一个仓库的议题页嘛喵~");
            return;
        }
        self.return_page = if self.page == Page::Issues {
            Page::Issues
        } else {
            return_page_when_opening(self.page, self.return_page)
        };
        self.page = Page::IssueDetail;
        self.search_focus = false;
        self.list_filter_focus = false;
        self.release_filter_focus = false;
        self.readme_fullscreen = false;
        self.issue_open_number = Some(issue.number);
        self.issue_open = Some(issue.clone());
        self.issue_comments.clear();
        self.issue_pr_files.clear();
        self.issue_comment_input = TextInput::new("");
        self.issue_comment_focus = false;
        self.issue_comment_saving = false;
        self.issue_action_loading = false;
        self.issue_scroll = 0;
        self.issue_pr_files_scroll = 0;
        self.fetch_issue_detail();
        self.fetch_issue_comments();
        if issue.is_pull_request() {
            self.fetch_pull_files();
        }
    }

    fn fetch_issue_detail(&mut self) {
        let (Some(repo), Some(number)) = (self.issue_repo.clone(), self.issue_open_number) else {
            return;
        };
        if self.issue_detail_loading == Some(number) {
            return;
        }
        self.issue_detail_loading = Some(number);
        let name = repo.display_full_name();
        let private = repo.private;
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.issue_detail(&name, number, private);
            let _ = tx.send(Msg::IssueDetail {
                name,
                number,
                result,
            });
        });
    }

    fn fetch_issue_comments(&mut self) {
        let (Some(repo), Some(number)) = (self.issue_repo.clone(), self.issue_open_number) else {
            return;
        };
        if self.issue_comments_loading == Some(number) {
            return;
        }
        self.issue_comments_loading = Some(number);
        let name = repo.display_full_name();
        let private = repo.private;
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.issue_comments(&name, number, private);
            let _ = tx.send(Msg::IssueComments {
                name,
                number,
                result,
            });
        });
    }

    fn fetch_pull_files(&mut self) {
        let (Some(repo), Some(number)) = (self.issue_repo.clone(), self.issue_open_number) else {
            return;
        };
        if self.issue_pr_files_loading == Some(number) {
            return;
        }
        self.issue_pr_files_loading = Some(number);
        let name = repo.display_full_name();
        let private = repo.private;
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.pull_files(&name, number, private);
            let _ = tx.send(Msg::PullFiles {
                name,
                number,
                result,
            });
        });
    }

    pub fn refresh_issue_detail(&mut self) {
        self.fetch_issue_detail();
        self.fetch_issue_comments();
        if self.issue_open.as_ref().is_some_and(Issue::is_pull_request) {
            self.fetch_pull_files();
        }
    }

    pub fn issue_scroll_by(&mut self, delta: i32) {
        self.issue_scroll =
            (self.issue_scroll as i32 + delta).clamp(0, self.issue_scroll_max as i32) as u16;
    }

    pub fn start_issue_comment(&mut self) {
        self.issue_comment_focus = true;
    }

    pub fn submit_issue_comment(&mut self) {
        if self.issue_comment_saving {
            return;
        }
        let body = self.issue_comment_input.value.trim().to_string();
        if body.is_empty() {
            self.set_status(StatusKind::Info, "评论还空着喵~先写点什么嘛喵~");
            return;
        }
        let (Some(repo), Some(number)) = (self.issue_repo.clone(), self.issue_open_number) else {
            return;
        };
        let name = repo.display_full_name();
        self.issue_comment_saving = true;
        self.set_status(StatusKind::Info, "正在把评论送给 GitHub 喵~...");
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.create_issue_comment(&name, number, &body);
            let _ = tx.send(Msg::IssueCommentCreated {
                name,
                number,
                result,
            });
        });
    }

    pub fn toggle_issue_state(&mut self) {
        if self.issue_action_loading {
            return;
        }
        let Some(issue) = self.issue_open.clone() else {
            return;
        };
        let (Some(repo), Some(number)) = (self.issue_repo.clone(), self.issue_open_number) else {
            return;
        };
        let state = if issue.state == "open" {
            "closed"
        } else {
            "open"
        };
        let name = repo.display_full_name();
        self.issue_action_loading = true;
        self.set_status(StatusKind::Info, "正在改议题状态喵~...");
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.set_issue_state(&name, number, state);
            let _ = tx.send(Msg::IssueStateChanged {
                name,
                number,
                result,
            });
        });
    }

    pub fn open_merge_confirm(&mut self) {
        let Some(issue) = self.issue_open.as_ref() else {
            return;
        };
        if !issue.is_pull_request() {
            self.set_status(StatusKind::Info, "这是议题不是合并请求喵~");
            return;
        }
        if issue.is_merged() || issue.state != "open" {
            self.set_status(StatusKind::Info, "这个合并请求已经结束啦喵~");
            return;
        }
        self.overlay = Some(Overlay::MergePr);
    }

    pub fn confirm_merge_pull(&mut self) {
        if self.issue_action_loading {
            return;
        }
        let (Some(repo), Some(number)) = (self.issue_repo.clone(), self.issue_open_number) else {
            return;
        };
        self.overlay = None;
        self.issue_action_loading = true;
        self.set_status(StatusKind::Info, "正在合并喵~...");
        let name = repo.display_full_name();
        let tx = self.tx.clone();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let result = client.merge_pull(&name, number);
            let _ = tx.send(Msg::PullMerged {
                name,
                number,
                result,
            });
        });
    }

    pub fn open_issue_browser(&mut self) {
        if let Some(issue) = self.issue_open.as_ref() {
            self.open_url(&issue.html_url.clone());
        }
    }

    pub fn handle_issue_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                if self.issue_comment_focus {
                    self.issue_comment_focus = false;
                } else {
                    self.go_back();
                }
            }
            KeyCode::Char('c') if !self.issue_comment_focus => self.start_issue_comment(),
            KeyCode::Char('x') if !self.issue_comment_focus => self.toggle_issue_state(),
            KeyCode::Char('M') if !self.issue_comment_focus => self.open_merge_confirm(),
            KeyCode::Char('o') if !self.issue_comment_focus => self.open_issue_browser(),
            KeyCode::Char('r') if !self.issue_comment_focus => self.refresh_issue_detail(),
            _ => {}
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
            if self.overlay == Some(Overlay::NewIssue) {
                self.submit_new_issue();
            } else if self.overlay == Some(Overlay::NewGist) {
                self.submit_new_gist();
            } else if self.overlay == Some(Overlay::NewPull) {
                self.submit_new_pull();
            } else if self.overlay == Some(Overlay::EditRepo) {
                self.save_repo_edit();
            } else if self.overlay == Some(Overlay::RepoLanguages) {
                return;
            } else if self.page == Page::IssueDetail && self.issue_comment_focus {
                self.submit_issue_comment();
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
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('k') | KeyCode::Char('K'))
        {
            self.open_palette();
            return;
        }
        if self.overlay == Some(Overlay::Palette) {
            match key.code {
                KeyCode::Esc => self.overlay = None,
                KeyCode::Enter => self.activate_palette(),
                KeyCode::Up => self.move_palette_selection(-1),
                KeyCode::Down => self.move_palette_selection(1),
                KeyCode::PageUp => self.move_palette_selection(-5),
                KeyCode::PageDown => self.move_palette_selection(5),
                KeyCode::Home => self.palette_selected = 0,
                KeyCode::End => {
                    self.palette_selected = self.palette_filtered().len().saturating_sub(1);
                }
                KeyCode::Backspace => {
                    self.palette_input.backspace();
                    self.palette_selected = 0;
                }
                KeyCode::Delete => {
                    self.palette_input.delete();
                    self.palette_selected = 0;
                }
                KeyCode::Left => self.palette_input.left(),
                KeyCode::Right => self.palette_input.right(),
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.palette_input = TextInput::new("");
                    self.palette_selected = 0;
                }
                KeyCode::Char(ch) => {
                    self.palette_input.insert(ch);
                    self.palette_selected = 0;
                }
                _ => {}
            }
            return;
        }
        if self.overlay == Some(Overlay::Themes) {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('T') => self.overlay = None,
                KeyCode::Up | KeyCode::Char('k') => self.move_theme_selection(-1),
                KeyCode::Down | KeyCode::Char('j') => self.move_theme_selection(1),
                KeyCode::Home | KeyCode::Char('g') => self.theme_selected = 0,
                KeyCode::End | KeyCode::Char('G') => {
                    self.theme_selected = self.themes.len().saturating_sub(1);
                }
                KeyCode::Enter => self.choose_theme_selection(),
                _ => {}
            }
            return;
        }
        if self.overlay == Some(Overlay::NewIssue) {
            self.handle_new_issue_key(key);
            return;
        }
        if self.overlay == Some(Overlay::RepoLanguages) {
            match key.code {
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => self.overlay = None,
                KeyCode::Up | KeyCode::Char('k') => {
                    self.repo_language_offset = self.repo_language_offset.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let max = self
                        .repo_languages
                        .len()
                        .saturating_sub(self.repo_language_viewport.max(1));
                    self.repo_language_offset = (self.repo_language_offset + 1).min(max);
                }
                KeyCode::Char('g') | KeyCode::Home => self.repo_language_offset = 0,
                KeyCode::Char('G') | KeyCode::End => {
                    self.repo_language_offset = self
                        .repo_languages
                        .len()
                        .saturating_sub(self.repo_language_viewport.max(1));
                }
                KeyCode::Char('r') => self.fetch_repo_languages(true),
                _ => {}
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
        if self.overlay == Some(Overlay::ForkRepo) {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    if self.fork_loading.is_none() {
                        self.overlay = None;
                        self.fork_repo = None;
                    }
                }
                KeyCode::Enter | KeyCode::Char('\r') | KeyCode::Char('\n') => {
                    self.confirm_fork_repo();
                }
                _ => {}
            }
            return;
        }
        if self.overlay == Some(Overlay::DeleteRepo) {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    if self.delete_loading.is_none() {
                        self.overlay = None;
                        self.delete_repo = None;
                    }
                }
                KeyCode::Enter | KeyCode::Char('\r') | KeyCode::Char('\n') => {
                    self.confirm_delete_repo();
                }
                _ => {}
            }
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
        if self.overlay == Some(Overlay::Branches) {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.overlay = None;
                    self.branch_pick_side = None;
                }
                KeyCode::Up | KeyCode::Char('k') => self.move_branch_selection(-1),
                KeyCode::Down | KeyCode::Char('j') => self.move_branch_selection(1),
                KeyCode::Char('g') | KeyCode::Home => {
                    self.branch_selected = 0;
                    self.branch_offset = 0;
                }
                KeyCode::Char('G') | KeyCode::End => {
                    let len = self.branches.len();
                    if len > 0 {
                        self.branch_selected = len - 1;
                        let viewport = self.branch_viewport.max(1);
                        self.branch_offset = len.saturating_sub(viewport);
                    }
                }
                KeyCode::Enter => {
                    if let Some(branch) = self.branches.get(self.branch_selected).cloned() {
                        if let Some(side) = self.branch_pick_side {
                            self.set_compare_branch(side, branch.name);
                        } else {
                            self.select_branch(branch.name);
                        }
                    }
                }
                KeyCode::Char('r') => self.fetch_branches(true),
                _ => {}
            }
            return;
        }
        if self.overlay == Some(Overlay::CancelRun) {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    if !self.cancel_loading {
                        self.overlay = None;
                        self.cancel_run = None;
                    }
                }
                KeyCode::Enter => self.confirm_cancel_action_run(),
                _ => {}
            }
            return;
        }
        if self.overlay == Some(Overlay::NewGist) {
            self.handle_new_gist_key(key);
            return;
        }
        if self.overlay == Some(Overlay::NewPull) {
            self.handle_new_pull_key(key);
            return;
        }
        if self.overlay == Some(Overlay::DeleteGist) {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    if !self.gist_deleting {
                        self.overlay = None;
                        self.gist_delete = None;
                    }
                }
                KeyCode::Enter => self.confirm_delete_gist(),
                _ => {}
            }
            return;
        }
        if self.overlay == Some(Overlay::MergePr) {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => self.overlay = None,
                KeyCode::Enter => self.confirm_merge_pull(),
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

        if self.page == Page::IssueDetail && self.issue_comment_focus {
            match key.code {
                KeyCode::Esc => self.issue_comment_focus = false,
                KeyCode::Enter => self.issue_comment_input.insert('\n'),
                KeyCode::Backspace => self.issue_comment_input.backspace(),
                KeyCode::Delete => self.issue_comment_input.delete(),
                KeyCode::Left => self.issue_comment_input.left(),
                KeyCode::Right => self.issue_comment_input.right(),
                KeyCode::Home => self.issue_comment_input.home(),
                KeyCode::End => self.issue_comment_input.end(),
                KeyCode::Char(ch) if !ch.is_control() => self.issue_comment_input.insert(ch),
                _ => {}
            }
            return;
        }
        if self.page == Page::IssueDetail {
            match key.code {
                KeyCode::Char('c')
                | KeyCode::Char('x')
                | KeyCode::Char('M')
                | KeyCode::Char('o')
                | KeyCode::Char('r') => {
                    self.handle_issue_key(key);
                    return;
                }
                _ => {}
            }
        }
        if self.page == Page::Gists {
            match key.code {
                KeyCode::Char('n') => {
                    self.open_new_gist();
                    return;
                }
                KeyCode::Char('D') => {
                    self.open_delete_gist();
                    return;
                }
                KeyCode::Char('y') => {
                    self.copy_gist_url();
                    return;
                }
                KeyCode::Char('o') => {
                    self.open_gist_browser();
                    return;
                }
                KeyCode::Tab => {
                    self.cycle_gist_file();
                    return;
                }
                _ => {}
            }
        }
        if self.page == Page::Compare {
            match key.code {
                KeyCode::Tab => {
                    self.compare_focus = match self.compare_focus {
                        CompareSide::Base => CompareSide::Head,
                        CompareSide::Head => CompareSide::Base,
                    };
                    return;
                }
                KeyCode::Char('b') => {
                    self.open_compare_branches(self.compare_focus);
                    return;
                }
                KeyCode::Char('v') => {
                    self.toggle_compare_view();
                    return;
                }
                KeyCode::Char('n') => {
                    self.start_new_pull();
                    return;
                }
                _ => {}
            }
        }
        if self.page == Page::Files {
            match key.code {
                KeyCode::Char('w') => {
                    self.toggle_file_wrap();
                    return;
                }
                KeyCode::Char('y') => {
                    self.copy_file_path();
                    return;
                }
                KeyCode::Char('o') => {
                    self.open_file_browser();
                    return;
                }
                KeyCode::Char('d') => {
                    self.download_file();
                    return;
                }
                KeyCode::Char('h') => {
                    self.file_scroll_x(-4);
                    return;
                }
                KeyCode::Char('l') => {
                    self.file_scroll_x(4);
                    return;
                }
                _ => {}
            }
        }
        if self.page == Page::Network {
            match key.code {
                KeyCode::Tab => {
                    self.cycle_network_tab();
                    return;
                }
                KeyCode::Char('f') => {
                    self.toggle_follow();
                    return;
                }
                _ => {}
            }
        }
        if matches!(self.page, Page::Account | Page::Profile) && key.code == KeyCode::Char('n') {
            self.open_network(None);
            return;
        }
        if self.readme_fullscreen {
            match key.code {
                KeyCode::Esc | KeyCode::Char('F') | KeyCode::Char('q') => {
                    self.readme_fullscreen = false;
                }
                KeyCode::Up | KeyCode::Char('k') => self.scroll_repo_pane(-1),
                KeyCode::Down | KeyCode::Char('j') => self.scroll_repo_pane(1),
                KeyCode::Char('g') | KeyCode::Home => self.set_repo_pane_scroll(0),
                KeyCode::Char('G') | KeyCode::End => self.set_repo_pane_scroll(u16::MAX),
                KeyCode::PageUp => self.scroll_repo_pane(-10),
                KeyCode::PageDown => self.scroll_repo_pane(10),
                _ => {}
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

        if self.issue_filter_focus {
            match key.code {
                KeyCode::Enter | KeyCode::Esc => self.issue_filter_focus = false,
                KeyCode::Tab => {
                    self.issue_filter_focus = false;
                    self.navigate_next_page();
                }
                KeyCode::Backspace => self.issue_filter.backspace(),
                KeyCode::Delete => self.issue_filter.delete(),
                KeyCode::Left => self.issue_filter.left(),
                KeyCode::Right => self.issue_filter.right(),
                KeyCode::Home => self.issue_filter.home(),
                KeyCode::End => self.issue_filter.end(),
                KeyCode::Char(ch) => self.issue_filter.insert(ch),
                _ => {}
            }
            self.issue_selected = 0;
            self.clamp_issue_selection();
            return;
        }

        if self.commit_filter_focus {
            match key.code {
                KeyCode::Enter | KeyCode::Esc => self.commit_filter_focus = false,
                KeyCode::Tab => {
                    self.commit_filter_focus = false;
                    self.navigate_next_page();
                }
                KeyCode::Backspace => self.commit_filter.backspace(),
                KeyCode::Delete => self.commit_filter.delete(),
                KeyCode::Left => self.commit_filter.left(),
                KeyCode::Right => self.commit_filter.right(),
                KeyCode::Home => self.commit_filter.home(),
                KeyCode::End => self.commit_filter.end(),
                KeyCode::Char(ch) => self.commit_filter.insert(ch),
                _ => {}
            }
            self.commit_selected = 0;
            self.schedule_commit_detail(false);
            return;
        }

        if self.settings_only {
            match key.code {
                KeyCode::Esc => {
                    self.quit();
                    return;
                }
                KeyCode::Tab => {
                    self.settings_row = (self.settings_row + 1).min(11);
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
                KeyCode::Char('1'..='6')
                | KeyCode::Char('0')
                | KeyCode::Char('9')
                | KeyCode::Char('I')
                | KeyCode::Char('E')
                | KeyCode::Char('P')
                | KeyCode::Char('T')
                | KeyCode::Char(':')
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
        if self.page == Page::Repo {
            match key.code {
                KeyCode::Left => {
                    self.repo_actions_offset = self.repo_actions_offset.saturating_sub(1);
                    return;
                }
                KeyCode::Right => {
                    self.repo_actions_offset += 1;
                    return;
                }
                _ => {}
            }
        }
        if self.page == Page::Tags {
            match key.code {
                KeyCode::Esc => self.go_back(),
                KeyCode::Char('q') => self.quit(),
                KeyCode::Char('j') | KeyCode::Down => self.move_tag_selection(1),
                KeyCode::Char('k') | KeyCode::Up => self.move_tag_selection(-1),
                KeyCode::Char('g') | KeyCode::Home => self.select_tag(0),
                KeyCode::Char('G') | KeyCode::End => {
                    self.select_tag(self.tags.len().saturating_sub(1))
                }
                KeyCode::Enter | KeyCode::Char('o') => self.open_selected_tag_page(),
                KeyCode::Char('y') => self.copy_selected_tag_name(),
                KeyCode::Char('z') => self.download_selected_tag_archive(true),
                KeyCode::Char('Z') => self.download_selected_tag_archive(false),
                KeyCode::Char('u') => self.copy_ssh_clone(),
                KeyCode::Char('r') => self.fetch_tags(true),
                _ => {}
            }
            return;
        }
        if self.page == Page::Actions {
            match key.code {
                KeyCode::Char('R') => {
                    self.rerun_action_run();
                    return;
                }
                KeyCode::Char('X') => {
                    self.open_cancel_action_run();
                    return;
                }
                _ => {}
            }
        }
        if self.page.is_repo_page() {
            match key.code {
                KeyCode::Char('u') => {
                    self.copy_ssh_clone();
                    return;
                }
                KeyCode::Char('N') => {
                    self.copy_full_name();
                    return;
                }
                KeyCode::Char('v') if self.page == Page::Repo => {
                    self.open_tags();
                    return;
                }
                KeyCode::Char('y') if self.page == Page::ReleaseDetail => {
                    self.copy_selected_asset_url();
                    return;
                }
                _ => {}
            }
        }
        if self.page == Page::Issues {
            match key.code {
                KeyCode::Char('n') => {
                    self.open_new_issue();
                    return;
                }
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
            KeyCode::Char('/') if matches!(self.page, Page::Search | Page::Trending) => {
                self.focus_list_filter()
            }
            KeyCode::Char('/') if self.page == Page::Issues => {
                self.issue_filter_focus = true;
            }
            KeyCode::Char('/') if self.page == Page::Commits => {
                self.commit_filter_focus = true;
            }
            KeyCode::Char('/')
                if self.page == Page::Repo && self.repo_pane == RepoPane::Releases =>
            {
                self.focus_release_filter()
            }
            KeyCode::Char('/') | KeyCode::Char('s') => self.focus_search(),
            KeyCode::Char('1') => self.navigate(Page::Repositories),
            KeyCode::Char('0') => self.navigate(Page::Home),
            KeyCode::Char('2') => self.navigate(Page::Starred),
            KeyCode::Char('3') => self.navigate(Page::Search),
            KeyCode::Char('4') => self.open_account(),
            KeyCode::Char('5') => self.navigate(Page::Notifications),
            KeyCode::Char('6') => self.navigate(Page::Trending),
            KeyCode::Char('7') => self.navigate(Page::Gists),
            KeyCode::Char('9') | KeyCode::Char('I') => self.navigate(Page::Insights),
            KeyCode::Char('E') => self.random_repo(),
            KeyCode::Char('P') => self.toggle_pin(),
            KeyCode::Char('T') => self.open_themes(),
            KeyCode::Char(':') => self.open_palette(),
            KeyCode::Char('p') if self.page == Page::Trending => {
                self.set_trending_period(self.trending_period.next())
            }
            KeyCode::Char('F') if self.page == Page::Repo => self.toggle_readme_fullscreen(),
            KeyCode::Tab => self.navigate_next_page(),
            KeyCode::Esc => match self.page {
                Page::Repo
                | Page::Profile
                | Page::Actions
                | Page::Issues
                | Page::ReleaseDetail
                | Page::IssueDetail
                | Page::Gists
                | Page::Compare
                | Page::Files
                | Page::Network => self.go_back(),
                Page::Settings
                | Page::Account
                | Page::Notifications
                | Page::Home
                | Page::Insights => self.navigate(Page::Repositories),
                _ => {}
            },
            KeyCode::Char('r') => self.refresh_current(),
            KeyCode::Char('o') => match self.page {
                Page::Actions => self.open_action_run(None),
                Page::Issues => self.open_selected_issue_browser(),
                Page::Commits => self.open_selected_commit(),
                Page::ReleaseDetail => self.open_selected_release_page(),
                Page::Notifications => self.open_selected_notification(),
                Page::Network => self.open_selected_network_browser(),
                _ => self.open_current(),
            },
            KeyCode::Char('b') if self.page.is_repo_page() => self.open_branches(),
            KeyCode::Char('m') if self.page.is_repo_page() => self.open_compare(),
            KeyCode::Char('C') if self.page.is_repo_page() => self.open_commits(),
            KeyCode::Char('a') => self.open_actions(),
            KeyCode::Char('L') if self.page.is_repo_page() => self.open_repo_languages(),
            KeyCode::Char('i') => self.open_issues(IssueKind::All),
            KeyCode::Char('Y') if self.page == Page::Repo => self.open_fork_repo(),
            KeyCode::Char('D') if self.page == Page::Repo => self.open_delete_repo(),
            KeyCode::Char('R') | KeyCode::Char('x') if self.page == Page::Actions => {
                self.open_workflow_runner()
            }
            KeyCode::Char('c') => self.open_cache_manager(),
            KeyCode::Char('y') => self.copy_clone(),
            KeyCode::Char('f') if self.page == Page::Profile => self.toggle_profile_follow(),
            KeyCode::Char('f') => self.toggle_current_star(),
            KeyCode::Char('e') => self.open_repo_edit(),
            KeyCode::Char('[') => self.prev_list_page(),
            KeyCode::Char(']') => self.next_list_page(),
            KeyCode::Char('l') => {
                if matches!(
                    self.page,
                    Page::Repositories | Page::Search | Page::Trending
                ) {
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
                Page::Commits => self.open_selected_commit(),
                Page::ReleaseDetail => self.download_selected_release_asset(),
                Page::Notifications => self.open_selected_notification(),
                Page::IssueDetail => self.start_issue_comment(),
                Page::Gists => self.open_gist_browser(),
                Page::Compare => self.fetch_compare(),
                Page::Files => self.refresh_file(),
                Page::Network => self.open_selected_network_profile(),
                Page::Home => {
                    if self.pinned_list().is_empty() {
                        self.navigate(Page::Search);
                    } else {
                        self.open_pin(self.home_selected);
                    }
                }
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
        let next = (current + 1) % 8;
        self.navigate(Page::from_nav(next));
    }

    fn key_up(&mut self) {
        match self.page {
            Page::Repo => self.scroll_repo_pane(-1),
            Page::Actions => self.move_action_selection(-1),
            Page::Issues => self.move_issue_selection(-1),
            Page::Commits => self.move_commit_selection(-1),
            Page::ReleaseDetail => self.move_release_asset_selection(-1),
            Page::Notifications => self.move_notification_selection(-1),
            Page::Settings => self.settings_row = self.settings_row.saturating_sub(1),
            Page::Home => self.move_home_selection(-1),
            Page::IssueDetail => self.issue_scroll_by(-1),
            Page::Gists => self.move_gist_selection(-1),
            Page::Compare => self.move_compare_selection(-1),
            Page::Files => self.file_scroll_by(-1),
            Page::Network => self.move_network_selection(-1),
            Page::Insights => {}
            _ => self.move_selection(-1),
        }
    }

    fn key_down(&mut self) {
        match self.page {
            Page::Repo => self.scroll_repo_pane(1),
            Page::Actions => self.move_action_selection(1),
            Page::Issues => self.move_issue_selection(1),
            Page::Commits => self.move_commit_selection(1),
            Page::ReleaseDetail => self.move_release_asset_selection(1),
            Page::Notifications => self.move_notification_selection(1),
            Page::Settings => self.settings_row = (self.settings_row + 1).min(11),
            Page::Home => self.move_home_selection(1),
            Page::IssueDetail => self.issue_scroll_by(1),
            Page::Gists => self.move_gist_selection(1),
            Page::Compare => self.move_compare_selection(1),
            Page::Files => self.file_scroll_by(1),
            Page::Network => self.move_network_selection(1),
            Page::Insights => {}
            _ => self.move_selection(1),
        }
    }

    fn key_home(&mut self) {
        match self.page {
            Page::Repo => self.set_repo_pane_scroll(0),
            Page::Settings => self.settings_row = 0,
            Page::Actions => self.select_action_run(0),
            Page::Issues => self.select_issue(0),
            Page::Commits => self.select_commit(0),
            Page::ReleaseDetail => self.select_release_asset(0),
            Page::Notifications => self.notification_selected = 0,
            Page::Account => {}
            Page::Home => self.home_selected = 0,
            Page::IssueDetail => self.issue_scroll = 0,
            Page::Gists => self.select_gist(0),
            Page::Compare => self.select_compare_row(0),
            Page::Files => self.file_scroll = 0,
            Page::Network => self.select_network_row(0),
            Page::Insights => {}
            _ => self.first_list_page(),
        }
    }

    fn key_end(&mut self) {
        match self.page {
            Page::Repo => self.set_repo_pane_scroll(u16::MAX),
            Page::Settings => self.settings_row = 11,
            Page::Actions => {
                self.select_action_run(self.action_runs.len().saturating_sub(1));
            }
            Page::Issues => {
                let last = self.issue_indices().len().saturating_sub(1);
                self.select_issue(last);
            }
            Page::Commits => {
                let last = self.commits.len().saturating_sub(1);
                self.select_commit(last);
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
            Page::Home => {
                self.home_selected = self.pinned_list().len().saturating_sub(1);
            }
            Page::IssueDetail => self.issue_scroll = self.issue_scroll_max,
            Page::Gists => self.select_gist(self.gists.len().saturating_sub(1)),
            Page::Compare => self.select_compare_row(self.compare_row_count().saturating_sub(1)),
            Page::Files => self.file_scroll = self.file_scroll_max,
            Page::Network => self.select_network_row(self.network_row_count().saturating_sub(1)),
            Page::Insights => {}
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
            Page::Commits => self.move_commit_selection(-5),
            Page::ReleaseDetail => {
                self.release_scroll = self.release_scroll.saturating_sub(10);
            }
            Page::Notifications => self.move_notification_selection(-5),
            Page::IssueDetail => self.issue_scroll_by(-10),
            Page::Gists => self.gist_scroll_by(-10),
            Page::Compare => self.compare_scroll_by(-10),
            Page::Files => self.file_scroll_by(-10),
            Page::Network => self.move_network_selection(-5),
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
            Page::Commits => self.move_commit_selection(5),
            Page::ReleaseDetail => {
                self.release_scroll = (self.release_scroll + 10).min(self.release_scroll_max);
            }
            Page::Notifications => self.move_notification_selection(5),
            Page::IssueDetail => self.issue_scroll_by(10),
            Page::Gists => self.gist_scroll_by(10),
            Page::Compare => self.compare_scroll_by(10),
            Page::Files => self.file_scroll_by(10),
            Page::Network => self.move_network_selection(5),
            Page::Settings => self.settings_row = (self.settings_row + 3).min(11),
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
        match self.page {
            Page::Settings => self.adjust_settings(-1),
            Page::Files => self.file_scroll_x(-4),
            Page::Compare => self.focus_compare_side(CompareSide::Base),
            Page::Gists => {}
            _ => {}
        }
    }

    fn key_right(&mut self) {
        match self.page {
            Page::Settings => self.adjust_settings(1),
            Page::Files => self.file_scroll_x(4),
            Page::Compare => self.focus_compare_side(CompareSide::Head),
            _ => {}
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
            8 => self.adjust_commit_limit(direction),
            9 => self.cycle_default_page(direction),
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
            8 => self.adjust_commit_limit(1),
            9 => self.cycle_default_page(1),
            10 => self.clear_cache(),
            11 => self.save_config(true),
            _ => {}
        }
    }

    pub fn cycle_default_page(&mut self, direction: i32) {
        let current = Page::from_default_name(&self.config.default_page);
        let index = Page::DEFAULT_PAGES
            .iter()
            .position(|page| *page == current)
            .unwrap_or(0) as i32;
        let next = (index + direction).rem_euclid(Page::DEFAULT_PAGES.len() as i32) as usize;
        let page = Page::DEFAULT_PAGES[next];
        self.config.default_page = page.default_name().to_string();
        self.config_dirty = true;
        self.settings_row = 9;
        self.set_status(
            StatusKind::Info,
            &format!(
                "下次启动会进「{}」，记得到设置页保存哦喵~",
                page.default_label()
            ),
        );
    }

    pub fn adjust_commit_limit(&mut self, direction: i32) {
        let step = 10_i64 * i64::from(direction.signum());
        let next = (self.config.commit_limit as i64 + step).clamp(10, 100);
        self.config.commit_limit = next as usize;
        self.config_dirty = true;
        self.set_status(
            StatusKind::Info,
            &format!("提交页每次读 {} 条，记得到设置页保存哦喵~", next),
        );
    }

    pub fn cycle_theme(&mut self, direction: i32) {
        let index = self
            .themes
            .iter()
            .position(|flavor| *flavor == self.theme.flavor)
            .unwrap_or(0) as i32;
        let next = (index + direction).rem_euclid(self.themes.len() as i32) as usize;
        self.set_theme(self.themes[next].clone());
    }

    pub fn set_theme(&mut self, flavor: String) {
        if !Theme::exists(&self.paths, &flavor) {
            return;
        }
        self.theme = Theme::load(&self.paths, &flavor);
        self.config.theme = flavor;
        self.config_dirty = true;
        self.set_status(
            StatusKind::Info,
            &format!(
                "主题换成 {} 啦，记得到设置页保存哦喵~",
                self.theme.display_name()
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
        self.settings_row = row.min(11);
    }

    pub fn handle_paste(&mut self, text: &str) {
        if self.overlay == Some(Overlay::EditRepo) {
            if let Some(edit) = self.repo_edit.as_mut()
                && edit.editing
            {
                for ch in text.chars().filter(|ch| *ch != '\n' && *ch != '\r') {
                    match edit.row {
                        0 => edit.name.insert(ch),
                        1 => edit.description.insert(ch),
                        2 => edit.homepage.insert(ch),
                        3 => edit.topics.insert(ch),
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
        if self.overlay == Some(Overlay::NewIssue) {
            if !self.new_issue_saving {
                let multiline = self.new_issue_body_focus;
                let input = self.active_new_issue_input();
                for ch in text.chars() {
                    if ch == '\r' {
                        continue;
                    }
                    if ch == '\n' && !multiline {
                        continue;
                    }
                    if !ch.is_control() {
                        input.insert(ch);
                    }
                }
            }
            return;
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
                if self.overlay == Some(Overlay::Branches) {
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
            Action::SetTrendingPeriod(period) => self.set_trending_period(period),
            Action::ToggleReadmeFullscreen => self.toggle_readme_fullscreen(),
            Action::SearchFocus => self.focus_search(),
            Action::ListFilterFocus => self.focus_list_filter(),
            Action::ReleaseFilterFocus => self.focus_release_filter(),
            Action::SearchHistory(query) => self.select_search_history(query),
            Action::SubmitSearch => self.submit_search(),
            Action::OpenPage => self.open_repo_page(),
            Action::Back => self.go_back(),
            Action::Refresh => self.refresh_current(),
            Action::OpenCurrent => self.open_current(),
            Action::RepoActionsLeft => {
                self.repo_actions_offset = self.repo_actions_offset.saturating_sub(1)
            }
            Action::RepoActionsRight => self.repo_actions_offset += 1,
            Action::CopyClone => self.copy_clone(),
            Action::SetTheme(flavor) => self.set_theme(flavor),
            Action::CycleTheme => self.cycle_theme(1),
            Action::OpenThemes => self.open_themes(),
            Action::OpenPalette => self.open_palette(),
            Action::OpenHelp => self.overlay = Some(Overlay::Help),
            Action::RandomRepo => self.random_repo(),
            Action::TogglePin => self.toggle_pin(),
            Action::OpenPin(index) => self.open_pin(index),
            Action::NavLeft => self.nav_offset = self.nav_offset.saturating_sub(1),
            Action::NavRight => self.nav_offset = (self.nav_offset + 1).min(7),
            Action::StartProxyEdit => self.start_proxy_edit(),
            Action::StartDownloadDirEdit => self.start_download_dir_edit(),
            Action::ToggleFileTree => self.toggle_file_tree(),
            Action::ToggleReadme => self.toggle_readme(),
            Action::Ttl(kind, direction) => self.adjust_ttl(kind, direction),
            Action::ClearCache => self.clear_cache(),
            Action::SaveConfig => self.save_config(true),
            Action::OpenDownloadDir => self.open_download_dir(),
            Action::SetSettingsRow(row) => self.handle_settings_row(row),
            Action::CommitLimit(direction) => {
                self.settings_row = 8;
                self.adjust_commit_limit(direction);
            }
            Action::OpenLanguages => self.open_languages(),
            Action::DownloadAsset { release, asset } => self.download_asset(release, asset),
            Action::DownloadSource => self.download_source_zip(),
            Action::OpenReleasePage(index) => self.open_release_page(index),
            Action::OpenReleaseDetail(index) => self.open_release_detail(index),
            Action::OpenTreePath { path, directory } => self.open_tree_path(&path, directory),
            Action::FocusRepoPane(pane) => self.set_repo_pane(pane),
            Action::OpenExternal(url) => self.open_external_url(&url),
            Action::OpenProfile(login) => self.open_user_profile(login),
            Action::ForkRepo => self.open_fork_repo(),
            Action::ConfirmFork => self.confirm_fork_repo(),
            Action::DeleteRepo => self.open_delete_repo(),
            Action::ConfirmDeleteRepo => self.confirm_delete_repo(),
            Action::ToggleStar => self.toggle_current_star(),
            Action::CloseOverlay => {
                if self.overlay == Some(Overlay::NewIssue) && self.new_issue_saving {
                    return;
                }
                if self.overlay == Some(Overlay::NewGist) && self.gist_saving {
                    return;
                }
                if self.overlay == Some(Overlay::NewPull) && self.pull_saving {
                    return;
                }
                if self.overlay == Some(Overlay::DeleteGist) && self.gist_deleting {
                    return;
                }
                if self.overlay == Some(Overlay::MergePr) && self.issue_action_loading {
                    return;
                }
                self.overlay = None;
                self.repo_edit = None;
                self.fork_repo = None;
                self.delete_repo = None;
                self.cancel_run = None;
            }
            Action::PrevListPage => self.prev_list_page(),
            Action::NextListPage => self.next_list_page(),
            Action::OpenRepoEdit => self.open_repo_edit(),
            Action::EditRepoRow(row) => {
                if let Some(edit) = self.repo_edit.as_mut() {
                    edit.row = row.min(8);
                    edit.editing = row <= 3;
                    match row {
                        4 => edit.private = !edit.private,
                        5 => edit.archived = !edit.archived,
                        6 => edit.has_issues = !edit.has_issues,
                        7 => edit.has_discussions = !edit.has_discussions,
                        8 => edit.has_wiki = !edit.has_wiki,
                        _ => {}
                    }
                }
            }
            Action::SaveRepoEdit => self.save_repo_edit(),
            Action::RefreshRateLimit => self.fetch_rate_limit(true),
            Action::OpenActions => self.open_actions(),
            Action::OpenBranches => self.open_branches(),
            Action::SelectBranch(branch) => self.select_branch(branch),
            Action::OpenCommits => self.open_commits(),
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
            Action::SelectCommit(index) => self.select_commit(index),
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
            Action::ToggleIssueState => self.toggle_issue_state(),
            Action::FocusIssueComment => self.start_issue_comment(),
            Action::SubmitIssueComment => self.submit_issue_comment(),
            Action::MergePull => self.open_merge_confirm(),
            Action::ConfirmMergePull => self.confirm_merge_pull(),
            Action::OpenNetwork => self.open_network(None),
            Action::SelectGist(index) => self.select_gist(index),
            Action::SelectGistFile(index) => {
                let count = self.gist_file_names().len();
                if count > 0 {
                    self.gist_file_selected = index.min(count - 1);
                    self.gist_scroll = 0;
                }
            }
            Action::StartNewGist => self.open_new_gist(),
            Action::DeleteGist => self.open_delete_gist(),
            Action::ConfirmDeleteGist => self.confirm_delete_gist(),
            Action::ToggleGistPublic => self.gist_public = !self.gist_public,
            Action::FocusGistField(field) => self.gist_field = field,
            Action::SubmitNewGist => self.submit_new_gist(),
            Action::CopyGistUrl => self.copy_gist_url(),
            Action::RefreshGists => self.fetch_gists(true),
            Action::SelectCompareRow(index) => self.select_compare_row(index),
            Action::PickCompareBranch(side) => self.open_compare_branches(side),
            Action::RefreshCompare => self.fetch_compare(),
            Action::ToggleCompareView => self.toggle_compare_view(),
            Action::StartNewPull => self.start_new_pull(),
            Action::ConfirmNewPull => self.submit_new_pull(),
            Action::TogglePullDraft => self.pull_draft = !self.pull_draft,
            Action::FocusPullField(field) => self.pull_field = field,
            Action::SelectNetwork(index) => self.select_network_row(index),
            Action::SetNetworkTab(tab) => self.set_network_tab(tab),
            Action::ToggleFollow => self.toggle_follow(),
            Action::ToggleFileWrap => self.toggle_file_wrap(),
            Action::CopyFilePath => self.copy_file_path(),
            Action::DownloadFile => self.download_file(),
            Action::OpenCacheManager => self.open_cache_manager(),
            Action::ClearCacheGroup(index) => self.clear_cache_group(index),
            Action::SelectCacheRow(index) => {
                let last = cache_groups().len().saturating_sub(1);
                self.cache_row = index.min(last);
            }
            Action::OpenCacheDir => self.open_cache_dir(),
            Action::OpenRepoLanguages => self.open_repo_languages(),
            Action::OpenTags => self.open_tags(),
            Action::OpenTagPage => self.open_selected_tag_page(),
            Action::SelectTag(index) => self.select_tag(index),
            Action::CopyTagName => self.copy_selected_tag_name(),
            Action::DownloadTagZip => self.download_selected_tag_archive(true),
            Action::DownloadTagTarball => self.download_selected_tag_archive(false),
            Action::CopySsh => self.copy_ssh_clone(),
            Action::CopyFullName => self.copy_full_name(),
            Action::CopyAssetUrl { asset } => {
                self.release_asset_selected = asset;
                self.copy_selected_asset_url();
            }
            Action::FocusIssueFilter => {
                self.issue_filter_focus = true;
                self.list_filter_focus = false;
            }
            Action::FocusCommitFilter => {
                self.commit_filter_focus = true;
                self.list_filter_focus = false;
            }
            Action::RerunActionRun => self.rerun_action_run(),
            Action::CancelActionRun => self.open_cancel_action_run(),
            Action::ConfirmCancelRun => self.confirm_cancel_action_run(),
            Action::StartNewIssue => self.open_new_issue(),
            Action::FocusNewIssue(body) => self.new_issue_body_focus = body,
            Action::SubmitNewIssue => self.submit_new_issue(),
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

fn should_schedule_selected_detail(page: Page) -> bool {
    matches!(
        page,
        Page::Repositories | Page::Starred | Page::Search | Page::Trending
    )
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

fn readme_cache_key(name: &str, branch: &str) -> String {
    format!("readme_{}_{}", key_part(name), key_part(branch))
}

fn branches_cache_key(name: &str) -> String {
    format!("branches_{}", key_part(name))
}

fn commits_cache_key(name: &str, branch: &str) -> String {
    format!("commits_{}_{}", key_part(name), key_part(branch))
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

fn languages_cache_key(name: &str) -> String {
    format!("languages_{}", key_part(name))
}

fn commit_detail_cache_key(name: &str, sha: &str) -> String {
    format!("commit_{}_{}", key_part(name), key_part(sha))
}

fn issues_cache_key(name: &str, state: IssueState) -> String {
    format!("issues_{}_{}", key_part(name), state.api_value())
}

fn notifications_cache_key(all: bool) -> String {
    format!("notifications_{}", if all { "all" } else { "unread" })
}

fn trending_key(period: TrendingPeriod) -> String {
    search_key(&format!("trending:{}", period.key()), "stars")
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
    fn issue_and_commit_filters_follow_selection() {
        let dir = std::env::temp_dir().join(format!("gxt-filter-test-{}", std::process::id()));
        let paths = Paths::for_test(dir.join("config"), dir.join("cache"));
        let mut app = App::settings(paths, None);
        app.issues = vec![
            crate::model::Issue {
                title: "crash on start".to_string(),
                ..Default::default()
            },
            crate::model::Issue {
                title: "docs typo".to_string(),
                ..Default::default()
            },
        ];
        app.issue_filter = TextInput::new("crash");
        assert_eq!(app.issue_indices(), vec![0]);
        assert_eq!(
            app.selected_issue().map(|issue| issue.title),
            Some("crash on start".to_string())
        );
        app.select_issue(9);
        assert_eq!(app.issue_selected, 0);

        app.commits = vec![
            crate::model::Commit {
                sha: "abcdef1234".to_string(),
                commit: crate::model::CommitInfo {
                    message: "fix parser".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            },
            crate::model::Commit {
                sha: "1234567890".to_string(),
                commit: crate::model::CommitInfo {
                    message: "docs update".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            },
        ];
        app.commit_filter = TextInput::new("parser");
        assert_eq!(app.commit_indices(), vec![0]);
        assert_eq!(
            app.selected_commit().map(|commit| commit.sha),
            Some("abcdef1234".to_string())
        );
        app.move_commit_selection(5);
        assert_eq!(app.commit_selected, 0);
        let _ = std::fs::remove_dir_all(dir);
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
    fn detail_rescheduling_skips_open_repo_pages() {
        assert!(should_schedule_selected_detail(Page::Starred));
        assert!(should_schedule_selected_detail(Page::Search));
        assert!(!should_schedule_selected_detail(Page::Repo));
        assert!(!should_schedule_selected_detail(Page::ReleaseDetail));
        assert!(!should_schedule_selected_detail(Page::Profile));
    }

    fn test_repo(owner: &str, name: &str) -> Repo {
        Repo {
            name: name.to_string(),
            full_name: format!("{owner}/{name}"),
            owner: crate::model::Owner {
                login: owner.to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn unstar_from_open_favorite_keeps_current_repo_detail() {
        let dir = std::env::temp_dir().join(format!("gitx-unstar-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let paths = Paths::for_test(dir.join("config"), dir.join("cache"));
        let mut app = App::settings(paths, None);
        app.settings_only = false;
        let opened = test_repo("octocat", "hello");
        let other = test_repo("octocat", "world");
        app.page = Page::Repo;
        app.return_page = Page::Starred;
        app.page_repo = Some(opened.clone());
        app.starred = vec![opened.clone(), other];
        app.star_selected = 0;
        app.detail_for = Some(opened.display_full_name());
        app.detail = Some(opened.clone());
        app.releases = vec![Release {
            tag_name: "v1".to_string(),
            ..Default::default()
        }];
        app.handle_message(Msg::StarToggled {
            name: opened.display_full_name(),
            starred: false,
            result: Ok(()),
        });
        assert_eq!(app.detail_for.as_deref(), Some("octocat/hello"));
        assert_eq!(app.releases.len(), 1);
        assert_eq!(app.releases[0].tag_name, "v1");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn deleted_repo_leaves_lists_and_returns_to_source() {
        let dir = std::env::temp_dir().join(format!("gitx-delete-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let paths = Paths::for_test(dir.join("config"), dir.join("cache"));
        let mut app = App::settings(paths, None);
        app.settings_only = false;
        let repo = test_repo("me", "gone");
        app.repos = vec![repo.clone()];
        app.page = Page::Repo;
        app.return_page = Page::Repositories;
        app.page_repo = Some(repo.clone());
        app.detail = Some(repo.clone());
        app.releases = vec![Release {
            tag_name: "v1".to_string(),
            ..Default::default()
        }];
        app.finish_deleted_repo(&repo.display_full_name());
        assert!(app.repos.is_empty());
        assert!(app.page_repo.is_none());
        assert!(app.releases.is_empty());
        assert_eq!(app.page, Page::Repositories);
        let _ = std::fs::remove_dir_all(dir);
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

    #[test]
    fn default_page_names_round_trip() {
        assert_eq!(Page::from_default_name("home"), Page::Home);
        assert_eq!(Page::from_default_name("trending"), Page::Trending);
        assert_eq!(Page::from_default_name("insights"), Page::Insights);
        assert_eq!(Page::from_default_name("nonsense"), Page::Home);
        for page in Page::DEFAULT_PAGES {
            assert_eq!(Page::from_default_name(page.default_name()), page);
        }
    }

    #[test]
    fn default_page_setting_cycles_and_marks_dirty() {
        let root = std::env::temp_dir().join(format!("gitx-default-page-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let paths = Paths::for_test(root.join("config"), root.join("cache"));
        let mut app = App::settings(paths, None);
        app.config.default_page = "home".to_string();
        app.cycle_default_page(1);
        assert_eq!(app.config.default_page, "repositories");
        assert_eq!(app.settings_row, 9);
        assert!(app.config_dirty);
        app.cycle_default_page(-1);
        assert_eq!(app.config.default_page, "home");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn boot_loads_unread_notifications_for_home() {
        let root = std::env::temp_dir().join(format!("gitx-home-unread-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let paths = Paths::for_test(root.join("config"), root.join("cache"));
        std::fs::create_dir_all(paths.cache_dir()).unwrap();
        std::fs::write(
            paths.cache_dir().join("notifications_unread.json"),
            format!(
                "{{\"fetched_at\":{},\"data\":[{{\"id\":\"1\",\"unread\":true}}]}}",
                now_epoch()
            ),
        )
        .unwrap();
        let mut app = App::settings(paths, None);
        app.settings_only = false;
        app.page = Page::Home;
        app.boot();
        assert_eq!(app.unread_notifications(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn dashboard_pages_do_not_schedule_list_detail() {
        assert!(!should_schedule_selected_detail(Page::Home));
        assert!(!should_schedule_selected_detail(Page::Insights));
        assert!(!should_schedule_selected_detail(Page::Settings));
    }

    #[test]
    fn issue_enter_opens_detail_page() {
        let root = std::env::temp_dir().join(format!("gitx-issue-enter-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let paths = Paths::for_test(root.join("config"), root.join("cache"));
        let mut app = App::settings(paths, None);
        app.settings_only = false;
        app.page = Page::Issues;
        app.issue_repo = Some(Repo {
            name: "demo".to_string(),
            full_name: "me/demo".to_string(),
            ..Default::default()
        });
        app.issues = vec![Issue {
            number: 3,
            title: "打开详情".to_string(),
            ..Default::default()
        }];
        app.issue_selected = 0;
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.page, Page::IssueDetail);
        assert_eq!(app.issue_open_number, Some(3));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn gist_page_joins_navigation_and_names() {
        assert_eq!(Page::Gists.nav_index(), Some(7));
        assert_eq!(Page::from_nav(7), Page::Gists);
        assert_eq!(Page::from_nav(0), Page::Home);
        assert_eq!(Page::from_default_name("gists"), Page::Gists);
        assert_eq!(Page::Gists.default_name(), "gists");
        assert!(Page::DEFAULT_PAGES.contains(&Page::Gists));
        assert!(Page::IssueDetail.is_repo_page());
        assert!(Page::Compare.is_repo_page());
        assert!(Page::Files.is_repo_page());
        assert!(!Page::Gists.is_repo_page());
        assert!(!Page::Network.is_repo_page());
    }

    #[test]
    fn gist_files_cycle_and_compare_rows_clamp() {
        let root = std::env::temp_dir().join(format!("gitx-feature-state-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let paths = Paths::for_test(root.join("config"), root.join("cache"));
        let mut app = App::settings(paths, None);
        app.settings_only = false;

        let files = std::collections::HashMap::from([
            (
                "b.md".to_string(),
                crate::model::GistFile {
                    filename: "b.md".to_string(),
                    content: Some("# b".to_string()),
                    ..Default::default()
                },
            ),
            (
                "a.txt".to_string(),
                crate::model::GistFile {
                    filename: "a.txt".to_string(),
                    content: Some("a".to_string()),
                    ..Default::default()
                },
            ),
        ]);
        app.gists = vec![crate::model::Gist {
            id: "g1".to_string(),
            files,
            ..Default::default()
        }];
        app.gist_selected = 0;
        assert_eq!(app.gist_file_names(), vec!["a.txt", "b.md"]);
        app.cycle_gist_file();
        assert_eq!(app.gist_file_selected, 1);
        app.cycle_gist_file();
        assert_eq!(app.gist_file_selected, 0);
        assert_eq!(
            app.current_gist_file().map(|file| file.filename),
            Some("a.txt".to_string())
        );

        app.compare_result = Some(crate::model::CompareResult {
            ahead_by: 1,
            commits: vec![crate::model::Commit {
                sha: "abcdef1234".to_string(),
                ..Default::default()
            }],
            files: vec![crate::model::PullFile {
                filename: "src/main.rs".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        });
        app.compare_view = CompareView::Commits;
        assert_eq!(app.compare_row_count(), 1);
        app.move_compare_selection(9);
        assert_eq!(app.compare_selected, 0);
        app.toggle_compare_view();
        assert_eq!(app.compare_view, CompareView::Files);
        assert_eq!(app.compare_row_count(), 1);
        app.select_compare_row(9);
        assert_eq!(app.compare_selected, 0);

        app.network_users = vec![
            User {
                login: "a".to_string(),
                ..Default::default()
            },
            User {
                login: "b".to_string(),
                ..Default::default()
            },
            User {
                login: "c".to_string(),
                ..Default::default()
            },
        ];
        app.network_tab = NetworkTab::Followers;
        app.move_network_selection(5);
        assert_eq!(app.network_selected, 2);
        assert_eq!(app.selected_network_login().as_deref(), Some("c"));
        app.network_tab = NetworkTab::Orgs;
        app.network_orgs = vec![crate::model::Org {
            login: "org".to_string(),
            ..Default::default()
        }];
        app.network_selected = 0;
        assert_eq!(app.selected_network_login().as_deref(), Some("org"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn profile_f_follows_instead_of_starring() {
        let root = std::env::temp_dir().join(format!("gitx-profile-f-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let paths = Paths::for_test(root.join("config"), root.join("cache"));
        let mut app = App::settings(paths, None);
        app.settings_only = false;
        app.client = crate::api::ApiClient::without_auth(String::new());
        app.login = Some("me".to_string());
        app.profile_login = Some("alice".to_string());
        app.page = Page::Profile;
        app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));
        assert!(app.status.text.contains("关注"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn palette_and_pin_keys_work() {
        let root = std::env::temp_dir().join(format!("gitx-pin-key-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let paths = Paths::for_test(root.join("config"), root.join("cache"));
        let mut app = App::settings(paths, None);
        app.settings_only = false;
        app.page = Page::Repositories;
        let repo = Repo {
            name: "hello-world".to_string(),
            full_name: "octocat/hello-world".to_string(),
            ..Default::default()
        };
        app.repos = vec![repo];
        app.handle_key(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::NONE));
        assert_eq!(app.config.pinned_repos, vec!["octocat/hello-world"]);
        assert_eq!(app.pinned_list().len(), 1);

        app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL));
        assert_eq!(app.overlay, Some(Overlay::Palette));
        app.palette_input = TextInput::new("洞察");
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.page, Page::Insights);
        assert_eq!(app.overlay, None);
        let _ = std::fs::remove_dir_all(root);
    }
}
