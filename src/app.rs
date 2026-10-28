use crate::api::ApiClient;
use crate::cache::{Cache, CacheStats, now_epoch, search_key};
use crate::config::{Config, Paths};
use crate::format::{self, base64};
use crate::model::{Release, Repo, RepoSort, SearchResponse, SearchSort, User, key_part};
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
    Settings,
    Repo,
}

impl Page {
    pub fn nav_index(self) -> Option<usize> {
        match self {
            Self::Repositories => Some(0),
            Self::Starred => Some(1),
            Self::Search => Some(2),
            Self::Settings => Some(3),
            Self::Repo => None,
        }
    }

    pub fn from_nav(index: usize) -> Self {
        match index {
            1 => Self::Starred,
            2 => Self::Search,
            3 => Self::Settings,
            _ => Self::Repositories,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Repositories => "我的仓库",
            Self::Starred => "收藏仓库",
            Self::Search => "搜索仓库",
            Self::Settings => "设置",
            Self::Repo => "仓库详情",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    Help,
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
    SetStarSort(RepoSort),
    SetSearchSort(SearchSort),
    SearchFocus,
    SearchHistory(String),
    SubmitSearch,
    OpenPage,
    Back,
    Refresh,
    OpenCurrent,
    CopyClone,
    SetTheme(String),
    StartProxyEdit,
    Ttl(TtlKind, i32),
    ClearCache,
    SaveConfig,
    SetSettingsRow(usize),
    OpenAsset { release: usize, asset: usize },
    OpenReleasePage(usize),
    SourceZip,
    Topic(String),
    CloseOverlay,
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
    pub settings: Rect,
    pub search: Rect,
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
}

pub struct App {
    pub paths: Paths,
    pub config: Config,
    pub theme: Theme,
    pub client: ApiClient,
    pub cache: Cache,
    pub user: Option<User>,
    pub login: Option<String>,
    pub page: Page,
    pub return_page: Page,
    pub overlay: Option<Overlay>,
    pub should_quit: bool,

    pub repos: Vec<Repo>,
    pub starred: Vec<Repo>,
    pub search_results: Vec<Repo>,
    pub search_total: Option<u64>,
    pub search_submitted: Option<String>,
    pub search_input: TextInput,
    pub search_focus: bool,
    pub search_sort: SearchSort,
    pub repo_sort: RepoSort,
    pub star_sort: RepoSort,
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
    pub detail_scroll: u16,
    pub detail_scroll_max: u16,
    pub page_repo: Option<Repo>,
    pub page_scroll: u16,
    pub page_scroll_max: u16,

    pub settings_row: usize,
    pub editing_proxy: bool,
    pub proxy_input: TextInput,
    pub config_dirty: bool,
    pub cache_stats: CacheStats,

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

    fetched_repos_at: Option<i64>,
    fetched_starred_at: Option<i64>,
    pending_detail: Option<PendingDetail>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
}

impl App {
    pub fn new(paths: Paths, theme_override: Option<String>) -> Self {
        let config = Config::load(&paths);
        let theme_name = theme_override
            .filter(|value| FLAVORS.contains(&value.as_str()))
            .unwrap_or_else(|| config.theme.clone());
        let theme = Theme::load(&paths, &theme_name);
        let client = ApiClient::new(config.proxy.clone());
        let cache = Cache::new(paths.cache_dir().to_path_buf());
        let cache_stats = cache.stats();
        let (tx, rx) = mpsc::channel();
        let mut app = Self {
            paths,
            config,
            theme,
            client,
            cache,
            user: None,
            login: None,
            page: Page::Repositories,
            return_page: Page::Repositories,
            overlay: None,
            should_quit: false,
            repos: Vec::new(),
            starred: Vec::new(),
            search_results: Vec::new(),
            search_total: None,
            search_submitted: None,
            search_input: TextInput::new(""),
            search_focus: false,
            search_sort: SearchSort::BestMatch,
            repo_sort: RepoSort::Updated,
            star_sort: RepoSort::Updated,
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
            detail_scroll: 0,
            detail_scroll_max: 0,
            page_repo: None,
            page_scroll: 0,
            page_scroll_max: 0,
            settings_row: 0,
            editing_proxy: false,
            proxy_input: TextInput::new(""),
            config_dirty: false,
            cache_stats,
            user_loading: false,
            repos_loading: false,
            starred_loading: false,
            search_loading: false,
            detail_loading: None,
            releases_loading: None,
            status: Status {
                text: "准备就绪，人家先看看缓存里有什么。".to_string(),
                kind: StatusKind::Info,
            },
            hitboxes: Vec::new(),
            hover: None,
            layout: LayoutRects::default(),
            started_at: Instant::now(),
            last_click: None,
            detail_scroll_reset: false,
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
        if !self.client.token_present() {
            self.set_status(
                StatusKind::Error,
                "还没有登录 GitHub，只能查看公开内容。先在终端运行 gh auth login 哦。",
            );
        }

        if let Some(cached) = self.cache.load::<User>("user") {
            self.user = Some(cached.data.clone());
            self.login = Some(cached.data.login.clone());
            self.load_account_cache();
        }

        if self.client.token_present() {
            self.fetch_user();
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
                            &format!("已经连上 GitHub，当前账号是 {}。", login),
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
                            &format!("仓库已经同步好啦，一共 {} 个。", self.repos.len()),
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
                            &format!("收藏已经同步好啦，一共 {} 个。", self.starred.len()),
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
                                "没有找到符合条件的仓库，换个词试试嘛。",
                            );
                        } else {
                            self.set_status(
                                StatusKind::Ok,
                                &format!("找到 {} 个仓库，先展示前 40 个。", outcome.total),
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
                            self.detail = Some(repo);
                            self.detail_for = Some(name);
                            self.detail_fetched_at = Some(now_epoch());
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
                        }
                    }
                    Err(error) => {
                        if self.matches_current_repo(&name) {
                            self.set_status(StatusKind::Error, &error);
                        }
                    }
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
            return Some("人家正在确认身份...");
        }
        if self.repos_loading && self.page == Page::Repositories {
            return Some("人家正在搬仓库列表...");
        }
        if self.starred_loading && self.page == Page::Starred {
            return Some("人家正在整理收藏...");
        }
        if self.search_loading && self.page == Page::Search {
            return Some("人家正在找仓库...");
        }
        if self.page == Page::Repo && self.releases_loading.is_some() {
            return Some("人家正在取发行版...");
        }
        if self.detail_loading.is_some() {
            return Some("人家正在读仓库详情...");
        }
        None
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
        if self.page == Page::Repo {
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

    fn visible_indices_for(&self, page: Page) -> Vec<usize> {
        let raw = self.raw_list(page);
        let filter = if page == Page::Repositories {
            self.language_filter.as_deref()
        } else {
            None
        };
        let mut indices: Vec<usize> = raw
            .iter()
            .enumerate()
            .filter(|(_, repo)| {
                filter
                    .map(|language| repo.language_label().eq_ignore_ascii_case(language))
                    .unwrap_or(true)
            })
            .map(|(index, _)| index)
            .collect();
        match page {
            Page::Repositories => sort_indices(raw, &mut indices, self.repo_sort),
            Page::Starred => sort_indices(raw, &mut indices, self.star_sort),
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

    pub fn move_selection_to(&mut self, end: bool) {
        let page = self.current_list_page();
        let len = self.visible_indices_for(page).len();
        if len == 0 {
            return;
        }
        self.set_selected_visible(if end { len - 1 } else { 0 });
    }

    pub fn follow_selection(&mut self) {
        let len = self.visible_indices().len();
        if len == 0 {
            self.list_offset = 0;
            return;
        }
        let selected = self.selection_for(self.current_list_page());
        let viewport = self.viewport_cards.max(1);
        if selected < self.list_offset {
            self.list_offset = selected;
        } else if selected >= self.list_offset + viewport {
            self.list_offset = selected + 1 - viewport;
        }
        let max_offset = len.saturating_sub(viewport);
        self.list_offset = self.list_offset.min(max_offset);
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
            if language == "未标注" {
                continue;
            }
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
        counts.truncate(7);
        counts
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
            return "还没有开始搜索呢".to_string();
        };
        let total = self
            .search_total
            .map(format::human_count)
            .unwrap_or_else(|| "?".to_string());
        format!("「{query}」找到 {total} 个结果")
    }

    pub fn auth_label(&self) -> String {
        if !self.client.token_present() {
            "未登录".to_string()
        } else {
            self.login
                .clone()
                .or_else(|| Some("读取中".to_string()))
                .unwrap_or_else(|| "已登录".to_string())
        }
    }

    fn repos_stale(&self) -> bool {
        self.fetched_repos_at
            .map(|at| now_epoch() - at > self.config.repos_ttl_secs as i64)
            .unwrap_or(true)
    }

    fn starred_stale(&self) -> bool {
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
            self.set_status(StatusKind::Error, "还没有登录 GitHub，读不到仓库列表哦。");
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
            self.set_status(StatusKind::Error, "还没有登录 GitHub，读不到收藏哦。");
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
                    self.fetch_detail_if_needed(repo, true);
                }
            }
            Page::Settings => self.refresh_cache_stats(),
        }
    }

    pub fn navigate(&mut self, page: Page) {
        if page == Page::Repo || page == self.page {
            if page == Page::Search {
                self.search_focus = true;
            }
            return;
        }
        if self.editing_proxy {
            self.commit_proxy_edit();
        }
        if self.page == Page::Settings && self.config_dirty {
            self.save_config(false);
        }
        self.page = page;
        self.return_page = page;
        self.search_focus = page == Page::Search;
        self.list_offset = 0;
        self.pending_detail = None;
        if page == Page::Settings {
            self.refresh_cache_stats();
            self.editing_proxy = false;
        }
        if matches!(page, Page::Repositories | Page::Starred) {
            self.schedule_selected_detail();
            self.follow_selection();
        }
        if page == Page::Search {
            self.schedule_selected_detail();
        }
    }

    pub fn open_repo_page(&mut self) {
        let Some(repo) = self
            .page_repo
            .clone()
            .filter(|_| self.page == Page::Repo)
            .or_else(|| self.selected_repo())
        else {
            self.set_status(StatusKind::Info, "先选一个仓库，人家才知道你要看哪个嘛。");
            return;
        };
        if self.page != Page::Repo {
            self.return_page = self.current_list_page();
        }
        self.page_repo = Some(repo.clone());
        self.page = Page::Repo;
        self.page_scroll = 0;
        self.search_focus = false;
        self.detail_scroll_reset = true;
        self.detail_for = None;
        self.detail = None;
        self.releases.clear();
        self.detail_fetched_at = None;
        self.releases_fetched_at = None;
        self.load_detail_cache(&repo);
        self.pending_detail = Some(PendingDetail {
            repo,
            at: Instant::now(),
        });
    }

    pub fn go_back(&mut self) {
        if self.page != Page::Repo {
            return;
        }
        self.page = self.return_page;
        self.page_repo = None;
        self.search_focus = false;
        self.schedule_selected_detail();
    }

    pub fn perform_search(&mut self, force: bool) {
        let query = self.search_input.value.trim().to_string();
        if query.is_empty() {
            self.set_status(StatusKind::Info, "搜索框还是空的，先告诉人家想找什么嘛。");
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
                self.set_status(StatusKind::Info, "这组结果刚从缓存里拿出来啦。");
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
        if self.page != Page::Search {
            self.navigate(Page::Search);
        }
        self.search_focus = true;
    }

    pub fn select_search_history(&mut self, query: String) {
        self.search_input = TextInput::new(query);
        self.perform_search(false);
    }

    pub fn set_repo_sort(&mut self, sort: RepoSort) {
        self.repo_sort = sort;
        self.repo_selected = 0;
        self.list_offset = 0;
        self.schedule_selected_detail();
    }

    pub fn set_star_sort(&mut self, sort: RepoSort) {
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
        self.schedule_selected_detail();
    }

    pub fn open_current(&mut self) {
        let Some(repo) = self.effective_repo() else {
            self.set_status(StatusKind::Info, "还没有选中仓库呢。");
            return;
        };
        self.open_url(&repo.html_url);
    }

    pub fn copy_clone(&mut self) {
        let Some(repo) = self.effective_repo() else {
            self.set_status(StatusKind::Info, "还没有选中仓库呢。");
            return;
        };
        let url = if !repo.ssh_url.is_empty() {
            repo.ssh_url.clone()
        } else if !repo.clone_url.is_empty() {
            repo.clone_url.clone()
        } else {
            format!("https://github.com/{}.git", repo.display_full_name())
        };
        self.copy_text(&url, "克隆地址");
    }

    fn effective_repo(&self) -> Option<Repo> {
        if self.page == Page::Repo {
            self.page_repo.clone().or_else(|| self.detail.clone())
        } else {
            self.detail
                .clone()
                .filter(|detail| {
                    self.selected_repo()
                        .map(|repo| repo.display_full_name() == detail.display_full_name())
                        .unwrap_or(false)
                })
                .or_else(|| self.selected_repo())
        }
    }

    fn open_url(&mut self, url: &str) {
        if url.trim().is_empty() {
            self.set_status(StatusKind::Error, "这个地址是空的，打不开呢。");
            return;
        }
        let result = open_external(url);
        match result {
            Ok(()) => self.set_status(StatusKind::Ok, "已经交给浏览器打开啦。"),
            Err(error) => self.set_status(StatusKind::Error, &error),
        }
    }

    fn copy_text(&mut self, text: &str, label: &str) {
        if text.trim().is_empty() {
            self.set_status(StatusKind::Error, "没有可以复制的内容哦。");
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
            &format!("{label}已经放进剪贴板啦（终端要支持 OSC 52 才行）。"),
        );
    }

    pub fn open_asset(&mut self, release_index: usize, asset_index: usize) {
        let Some(asset) = self
            .releases
            .get(release_index)
            .and_then(|release| release.assets.get(asset_index))
            .cloned()
        else {
            return;
        };
        let url = self.client.accelerated(&asset.browser_download_url);
        self.open_url(&url);
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

    pub fn open_source_zip(&mut self) {
        let Some(repo) = self.effective_repo() else {
            return;
        };
        let url = self
            .client
            .source_zip(&repo.display_full_name(), &repo.default_branch);
        self.open_url(&url);
    }

    pub fn search_topic(&mut self, topic: String) {
        let query = format!("topic:{topic}");
        self.page = Page::Search;
        self.return_page = Page::Search;
        self.search_input = TextInput::new(query);
        self.search_sort = SearchSort::BestMatch;
        self.search_focus = false;
        self.perform_search(false);
    }

    pub fn scroll(&mut self, x: u16, y: u16, delta: i32) {
        if self.overlay.is_some() {
            return;
        }
        let layout = self.layout;
        if self.page == Page::Repo
            && (layout.contains(layout.page_releases, x, y) || layout.contains(layout.detail, x, y))
        {
            let step = 3_i32 * delta;
            let max = self.page_scroll_max as i32;
            self.page_scroll = (self.page_scroll as i32 + step).clamp(0, max.max(0)) as u16;
            return;
        }
        if self.page == Page::Settings {
            if delta > 0 {
                self.settings_row = (self.settings_row + 1).min(6);
            } else {
                self.settings_row = self.settings_row.saturating_sub(1);
            }
            return;
        }
        if layout.contains(layout.list, x, y) {
            let len = self.visible_indices().len();
            let viewport = self.viewport_cards.max(1);
            let max = len.saturating_sub(viewport);
            let next = self.list_offset as i32 + delta;
            self.list_offset = next.clamp(0, max as i32) as usize;
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
            self.save_config(true);
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

        match key.code {
            KeyCode::Char('q') => self.quit(),
            KeyCode::Char('?') => self.overlay = Some(Overlay::Help),
            KeyCode::Char('/') | KeyCode::Char('s') => self.focus_search(),
            KeyCode::Char('1') => self.navigate(Page::Repositories),
            KeyCode::Char('2') => self.navigate(Page::Starred),
            KeyCode::Char('3') => self.navigate(Page::Search),
            KeyCode::Char('4') => self.navigate(Page::Settings),
            KeyCode::Tab => self.navigate_next_page(),
            KeyCode::Esc => match self.page {
                Page::Repo => self.go_back(),
                Page::Settings => self.navigate(Page::Repositories),
                _ => {}
            },
            KeyCode::Char('r') => self.refresh_current(),
            KeyCode::Char('o') => self.open_current(),
            KeyCode::Char('y') => self.copy_clone(),
            KeyCode::Char('t') => self.cycle_theme(1),
            KeyCode::Enter => match self.page {
                Page::Settings => self.activate_settings_row(),
                Page::Repo => self.open_current(),
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
        let next = (current + 1) % 4;
        self.navigate(Page::from_nav(next));
    }

    fn key_up(&mut self) {
        match self.page {
            Page::Repo => self.page_scroll = self.page_scroll.saturating_sub(1),
            Page::Settings => self.settings_row = self.settings_row.saturating_sub(1),
            _ => self.move_selection(-1),
        }
    }

    fn key_down(&mut self) {
        match self.page {
            Page::Repo => self.page_scroll = (self.page_scroll + 1).min(self.page_scroll_max),
            Page::Settings => self.settings_row = (self.settings_row + 1).min(6),
            _ => self.move_selection(1),
        }
    }

    fn key_home(&mut self) {
        match self.page {
            Page::Repo => self.page_scroll = 0,
            Page::Settings => self.settings_row = 0,
            _ => self.move_selection_to(false),
        }
    }

    fn key_end(&mut self) {
        match self.page {
            Page::Repo => self.page_scroll = self.page_scroll_max,
            Page::Settings => self.settings_row = 6,
            _ => self.move_selection_to(true),
        }
    }

    fn key_page_up(&mut self) {
        match self.page {
            Page::Repo => self.page_scroll = self.page_scroll.saturating_sub(10),
            Page::Settings => self.settings_row = self.settings_row.saturating_sub(3),
            _ => self.move_selection(-5),
        }
    }

    fn key_page_down(&mut self) {
        match self.page {
            Page::Repo => self.page_scroll = (self.page_scroll + 10).min(self.page_scroll_max),
            Page::Settings => self.settings_row = (self.settings_row + 3).min(6),
            _ => self.move_selection(5),
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
            2 => self.adjust_ttl(TtlKind::Repositories, direction),
            3 => self.adjust_ttl(TtlKind::Detail, direction),
            4 => self.adjust_ttl(TtlKind::Search, direction),
            _ => {}
        }
    }

    fn activate_settings_row(&mut self) {
        match self.settings_row {
            0 => self.cycle_theme(1),
            1 => self.start_proxy_edit(),
            2 => self.adjust_ttl(TtlKind::Repositories, 1),
            3 => self.adjust_ttl(TtlKind::Detail, 1),
            4 => self.adjust_ttl(TtlKind::Search, 1),
            5 => self.clear_cache(),
            6 => self.save_config(true),
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
                "主题换成 {} 啦，记得到设置页保存哦。",
                Theme::label(&flavor)
            ),
        );
    }

    pub fn start_proxy_edit(&mut self) {
        self.editing_proxy = true;
        self.proxy_input = TextInput::new(self.config.proxy.clone());
        self.settings_row = 1;
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
                "代理地址要以 http:// 或 https:// 开头哦。",
            );
            return;
        }
        self.config.proxy = value.clone();
        self.client.set_proxy(&value);
        self.editing_proxy = false;
        self.config_dirty = true;
        self.set_status(StatusKind::Info, "代理设置改好了，保存之后会写进配置文件。");
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
            &format!("缓存时间调成 {} 了，记得保存哦。", format::ttl_label(next)),
        );
    }

    pub fn save_config(&mut self, announce: bool) {
        if let Err(error) = self.config.save(&self.paths) {
            self.set_status(StatusKind::Error, &format!("配置文件没有写进去：{error}"));
            return;
        }
        self.client.set_proxy(&self.config.proxy);
        self.config_dirty = false;
        if announce {
            self.set_status(
                StatusKind::Ok,
                &format!(
                    "配置保存好啦：{}",
                    format::display_path(&self.paths.config_file())
                ),
            );
        }
    }

    pub fn clear_cache(&mut self) {
        match self.cache.clear() {
            Ok(()) => {
                self.refresh_cache_stats();
                self.set_status(StatusKind::Ok, "缓存已经清空啦，下次会重新拉取。");
            }
            Err(error) => self.set_status(StatusKind::Error, &format!("缓存没清掉：{error}")),
        }
    }

    pub fn handle_settings_row(&mut self, row: usize) {
        self.settings_row = row.min(6);
    }

    pub fn handle_paste(&mut self, text: &str) {
        if self.overlay.is_some() {
            return;
        }
        if self.editing_proxy {
            for ch in text.chars().filter(|ch| *ch != '\n' && *ch != '\r') {
                self.proxy_input.insert(ch);
            }
        } else if self.search_focus {
            for ch in text.chars().filter(|ch| *ch != '\n' && *ch != '\r') {
                self.search_input.insert(ch);
            }
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::Moved => {
                if self.overlay.is_some() {
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
                if self.overlay.is_some() {
                    self.overlay = None;
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
            Action::SearchHistory(query) => self.select_search_history(query),
            Action::SubmitSearch => self.submit_search(),
            Action::OpenPage => self.open_repo_page(),
            Action::Back => self.go_back(),
            Action::Refresh => self.refresh_current(),
            Action::OpenCurrent => self.open_current(),
            Action::CopyClone => self.copy_clone(),
            Action::SetTheme(flavor) => self.set_theme(flavor),
            Action::StartProxyEdit => self.start_proxy_edit(),
            Action::Ttl(kind, direction) => self.adjust_ttl(kind, direction),
            Action::ClearCache => self.clear_cache(),
            Action::SaveConfig => self.save_config(true),
            Action::SetSettingsRow(row) => self.handle_settings_row(row),
            Action::OpenAsset { release, asset } => self.open_asset(release, asset),
            Action::OpenReleasePage(index) => self.open_release_page(index),
            Action::SourceZip => self.open_source_zip(),
            Action::Topic(topic) => self.search_topic(topic),
            Action::CloseOverlay => self.overlay = None,
        }
    }

    pub fn quit(&mut self) {
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
        .map_err(|error| format!("没能调起浏览器：{error}"))
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
