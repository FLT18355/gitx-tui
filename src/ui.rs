use crate::app::{
    Action, App, HitBox, LayoutRects, Overlay, Page, RepoPane, StatusKind, TextInput, TtlKind,
    cache_groups,
};
use crate::format;
use crate::icons;
use crate::model::{Repo, RepoSort, SearchSort};
use crate::theme::{FLAVORS, Theme};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

const MIN_WIDTH: u16 = 72;
const MIN_HEIGHT: u16 = 22;
const CARD_HEIGHT: u16 = 4;
const CARD_STEP: u16 = 5;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let theme = app.theme.clone();
    fill(frame, area, theme.color("base"));

    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        draw_too_small(frame, area, &theme);
        app.hitboxes.clear();
        return;
    }

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(2),
    ])
    .split(area);
    let mut hits = Vec::new();
    app.layout = LayoutRects::default();
    draw_header(frame, app, &theme, chunks[0], &mut hits);
    match app.page {
        Page::Repositories => draw_repositories_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Starred => draw_starred_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Search => draw_search_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Account => draw_account_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Settings => draw_settings_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Repo => draw_repo_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Actions => draw_actions_page(frame, app, &theme, chunks[1], &mut hits),
    }
    draw_footer(frame, app, &theme, chunks[2], &hits);

    if app.overlay.is_some() {
        hits.clear();
    }
    match app.overlay {
        Some(Overlay::Help) => draw_help(frame, app, &theme, area, &mut hits),
        Some(Overlay::Languages) => draw_languages_overlay(frame, app, &theme, area, &mut hits),
        Some(Overlay::EditRepo) => draw_repo_edit_overlay(frame, app, &theme, area, &mut hits),
        Some(Overlay::Cache) => draw_cache_overlay(frame, app, &theme, area, &mut hits),
        Some(Overlay::RunWorkflow) => draw_workflow_overlay(frame, app, &theme, area, &mut hits),
        None => app.layout.overlay = Rect::default(),
    }

    app.hitboxes = hits;
}

fn draw_header(frame: &mut Frame, app: &App, theme: &Theme, area: Rect, hits: &mut Vec<HitBox>) {
    fill(frame, area, theme.color("mantle"));
    let brand = Rect::new(area.x + 2, area.y, area.width.saturating_sub(4), 1);
    let mut title = vec![
        Span::styled(
            format!("{} ", icons::GITHUB),
            Style::default().fg(theme.color("blue")),
        ),
        Span::styled(
            "gxt",
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if area.width > 96 {
        title.push(Span::styled(
            "  终端里的 GitHub 工作台",
            Style::default().fg(theme.color("overlay0")),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(title)), brand);

    if app.settings_only {
        frame.render_widget(
            Paragraph::new("全局设置喵~  Ctrl+S 保存  Esc 退出喵~")
                .style(Style::default().fg(theme.color("subtext0"))),
            Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), 1),
        );
        fill(
            frame,
            Rect::new(area.x, area.y + 2, area.width, 1),
            theme.color("surface0"),
        );
        return;
    }

    let account = format!(
        "{} 账号 {}   {} {}喵~",
        icons::USER,
        app.auth_label(),
        icons::PALETTE,
        Theme::label(&app.theme.flavor)
    );
    if area.width > 118 {
        let account_width = format::text_width(&account) as u16 + 2;
        let right = Rect::new(
            area.x + area.width.saturating_sub(account_width),
            area.y,
            account_width,
            1,
        );
        frame.render_widget(
            Paragraph::new(account).style(Style::default().fg(theme.color("subtext0"))),
            right,
        );
    }

    let active = app
        .page
        .nav_index()
        .unwrap_or_else(|| app.return_page.nav_index().unwrap_or(0));
    let labels = [
        icons::label(icons::REPO, "仓库喵~"),
        icons::label(icons::STAR, "收藏喵~"),
        icons::label(icons::SEARCH, "搜索喵~"),
        icons::label(icons::USER, "账号喵~"),
    ];
    let pages = [
        Page::Repositories,
        Page::Starred,
        Page::Search,
        Page::Account,
    ];
    let mut x = area.x + 2;
    for (index, label) in labels.iter().enumerate() {
        let width = format::text_width(label) as u16 + 4;
        let rect = Rect::new(x, area.y + 1, width, 1);
        let selected = index == active;
        let action = Action::Navigate(pages[index]);
        let hovered = app.hover.as_ref() == Some(&action);
        let bg = if selected {
            theme.color("surface1")
        } else if hovered {
            theme.color("surface0")
        } else {
            theme.color("mantle")
        };
        fill(frame, rect, bg);
        let style = if selected {
            Style::default()
                .fg(theme.color("blue"))
                .add_modifier(Modifier::BOLD)
        } else if hovered {
            Style::default().fg(theme.color("text"))
        } else {
            Style::default().fg(theme.color("subtext0"))
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(format!(" {label} "), style)))
                .alignment(Alignment::Center),
            rect,
        );
        register(hits, rect, action);
        x += width + 1;
    }

    if area.width > 100 {
        let hint = "Tab 切页   / 搜索   ? 帮助喵~";
        let width = format::text_width(hint) as u16;
        let rect = Rect::new(
            area.x + area.width.saturating_sub(width + 2),
            area.y + 1,
            width,
            1,
        );
        frame.render_widget(
            Paragraph::new(hint).style(Style::default().fg(theme.color("overlay0"))),
            rect,
        );
    }
    fill(
        frame,
        Rect::new(area.x, area.y + 2, area.width, 1),
        theme.color("surface0"),
    );
}

fn draw_footer(frame: &mut Frame, app: &App, theme: &Theme, area: Rect, _hits: &[HitBox]) {
    fill(frame, area, theme.color("crust"));
    let status_rect = Rect::new(area.x + 2, area.y, area.width.saturating_sub(26), 1);
    let (text, color) = if let Some(download) = app.download_summary() {
        (
            format!("{}  {}", app.spinner(), download),
            theme.color("peach"),
        )
    } else if let Some(busy) = app.busy_text() {
        (format!("{}  {}", app.spinner(), busy), theme.color("peach"))
    } else {
        let (icon, color) = match app.status.kind {
            StatusKind::Info => (icons::INFO, theme.color("subtext0")),
            StatusKind::Ok => (icons::CHECK, theme.color("green")),
            StatusKind::Error => (icons::WARNING, theme.color("red")),
        };
        (format!("{icon}  {}", app.status.text), color)
    };
    frame.render_widget(
        Paragraph::new(format::truncate(&text, status_rect.width as usize))
            .style(Style::default().fg(color)),
        status_rect,
    );

    if area.width > 68 {
        let info = format!(
            "{} 缓存 {} 个文件 · {}   {} {}喵~",
            icons::DATABASE,
            app.cache_stats.files,
            format::format_size(app.cache_stats.bytes),
            icons::GLOBE,
            proxy_label(app)
        );
        let width = area.width.saturating_sub(2);
        let rect = Rect::new(area.x + 2, area.y + 1, width, 1);
        let hint = hints_for(app.page);
        let info_width = format::text_width(&info) as u16;
        let left_width = width.saturating_sub(info_width + 2);
        frame.render_widget(
            Paragraph::new(format::truncate(hint, left_width as usize))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(rect.x, rect.y, left_width, 1),
        );
        frame.render_widget(
            Paragraph::new(info)
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(rect.x, rect.y, width, 1),
        );
    } else {
        let hint = hints_for(app.page);
        frame.render_widget(
            Paragraph::new(format::truncate(hint, area.width as usize - 4))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), 1),
        );
    }
}

fn hints_for(page: Page) -> &'static str {
    match page {
        Page::Repositories => {
            "j/k 挑    [ ] 翻页    Enter 打开    e 编辑    f 收藏    l 语言    / 找仓库喵~"
        }
        Page::Starred => {
            "j/k 挑    [ ] 翻页    Enter 打开    f 收藏    o 主页    y 复制    r 刷新喵~"
        }
        Page::Search => "输入后回车    Esc 离开输入    j/k 挑    [ ] 翻页    Enter 打开喵~",
        Page::Account => "r 刷新账号和额度    Enter 打开主页    Esc 回仓库喵~",
        Page::Settings => "j/k 选    h/l 调    Enter 操作    Ctrl+S 保存    Esc 离开喵~",
        Page::Repo => "Esc 返回    Tab 换面板    j/k 滚动    a 工作流    e 编辑自己    o 主页喵~",
        Page::Actions => {
            "j/k 挑运行  Enter 去浏览器  R/x 运行工作流  PgUp/PgDn 滚任务  r 刷新  Esc 返回喵~"
        }
    }
}

fn draw_repositories_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    let sidebar_width = if area.width >= 122 {
        24
    } else if area.width >= 96 {
        22
    } else if area.width >= 82 {
        19
    } else {
        0
    };
    let detail_width = if area.width >= 126 {
        44
    } else if area.width >= 104 {
        38
    } else if area.width >= 90 {
        34
    } else {
        0
    };
    let columns = Layout::horizontal([
        Constraint::Length(sidebar_width),
        Constraint::Min(28),
        Constraint::Length(detail_width),
    ])
    .split(area);
    if sidebar_width > 0 {
        draw_sidebar(frame, app, theme, columns[0], hits);
    }
    draw_repositories_main(frame, app, theme, columns[1], hits);
    if detail_width > 0 {
        draw_repository_detail(frame, app, theme, columns[2], hits);
        app.layout.detail = columns[2];
    }
}

fn draw_sidebar(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(theme.color("surface0"))),
        area,
    );
    let width = area.width.saturating_sub(3);
    let mut y = area.y + 1;

    let login = app
        .user
        .as_ref()
        .map(|user| user.login.clone())
        .or_else(|| app.login.clone())
        .unwrap_or_else(|| "未登录喵~".to_string());
    frame.render_widget(
        Paragraph::new(icons::label(icons::USER, "当前账号喵~"))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 2, y, width, 1),
    );
    y += 1;
    frame.render_widget(
        Paragraph::new(format::truncate(&login, width as usize)).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(area.x + 2, y, width, 1),
    );
    y += 1;
    let auth = if app.client.token_present() {
        "gh 已经连上啦喵~"
    } else {
        "先跑一下 gh auth login 哦喵~"
    };
    frame.render_widget(
        Paragraph::new(format::truncate(auth, width as usize))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 2, y, width, 1),
    );
    y += 2;

    let lang_header = Rect::new(area.x, y, area.width.saturating_sub(1), 1);
    let lang_hover = app.hover.as_ref() == Some(&Action::OpenLanguages);
    if lang_hover {
        fill(frame, lang_header, theme.color("surface0"));
    }
    frame.render_widget(
        Paragraph::new(icons::label(icons::CODE, "语言筛选喵~"))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 2, y, width, 1),
    );
    register(hits, lang_header, Action::OpenLanguages);
    y += 1;

    let total = app.repos.len();
    let all_active = app.language_filter.is_none();
    let all_rect = Rect::new(area.x, y, area.width.saturating_sub(1), 1);
    draw_sidebar_item(
        frame,
        theme,
        all_rect,
        "全部喵~",
        total,
        all_active,
        app.hover.as_ref() == Some(&Action::Language(None)),
    );
    register(hits, all_rect, Action::Language(None));
    y += 1;

    let counts = app.language_counts();
    let language_limit = area.y + area.height.saturating_sub(7);
    let mut shown = 0;
    for (language, count) in &counts {
        if y >= language_limit {
            break;
        }
        let active = app
            .language_filter
            .as_deref()
            .map(|value| value.eq_ignore_ascii_case(language))
            .unwrap_or(false);
        let action = Action::Language(Some(language.clone()));
        let hovered = app.hover.as_ref() == Some(&action);
        let rect = Rect::new(area.x, y, area.width.saturating_sub(1), 1);
        draw_sidebar_item(frame, theme, rect, language, *count, active, hovered);
        register(hits, rect, action);
        shown += 1;
        y += 1;
    }
    if shown < counts.len() && y < area.y + area.height.saturating_sub(4) {
        let action = Action::OpenLanguages;
        let hovered = app.hover.as_ref() == Some(&action);
        let rect = Rect::new(area.x, y, area.width.saturating_sub(1), 1);
        if hovered {
            fill(frame, rect, theme.color("surface0"));
        }
        frame.render_widget(
            Paragraph::new(format!("查看全部 {} 种喵~", counts.len()))
                .style(Style::default().fg(theme.color("blue"))),
            Rect::new(area.x + 2, y, width, 1),
        );
        register(hits, rect, action);
    }

    let cache_top = area.y + area.height.saturating_sub(5);
    if cache_top > y {
        frame.render_widget(
            Paragraph::new(icons::label(icons::DATABASE, "本地缓存喵~"))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x + 2, cache_top, width, 1),
        );
        let stats = format!(
            "{} 个文件 · {}喵~",
            app.cache_stats.files,
            format::format_size(app.cache_stats.bytes)
        );
        frame.render_widget(
            Paragraph::new(format::truncate(&stats, width as usize))
                .style(Style::default().fg(theme.color("subtext0"))),
            Rect::new(area.x + 2, cache_top + 1, width, 1),
        );
        frame.render_widget(
            Paragraph::new("到设置页就能管缓存啦喵~")
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x + 2, cache_top + 2, width, 1),
        );
    }
}

fn draw_sidebar_item(
    frame: &mut Frame,
    theme: &Theme,
    rect: Rect,
    label: &str,
    count: usize,
    active: bool,
    hovered: bool,
) {
    if active || hovered {
        fill(frame, rect, theme.color("surface0"));
    }
    if active {
        fill(
            frame,
            Rect::new(rect.x, rect.y, 1, rect.height),
            theme.color("blue"),
        );
    }
    let style = if active {
        Style::default()
            .fg(theme.color("blue"))
            .add_modifier(Modifier::BOLD)
    } else if hovered {
        Style::default().fg(theme.color("text"))
    } else {
        Style::default().fg(theme.color("subtext0"))
    };
    let count_text = count.to_string();
    let left = Rect::new(rect.x + 2, rect.y, rect.width.saturating_sub(4), 1);
    frame.render_widget(
        Paragraph::new(format::truncate(label, left.width as usize)).style(style),
        left,
    );
    frame.render_widget(
        Paragraph::new(count_text)
            .alignment(Alignment::Right)
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(
            rect.x + 2,
            rect.y,
            rect.width.saturating_sub(4),
            rect.height,
        ),
    );
}

fn draw_repositories_main(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    draw_page_title(
        frame,
        app,
        theme,
        inner,
        &icons::label(icons::REPO, "人家的仓库喵~"),
        "人家把账号下的仓库都整理好啦喵~",
        "刷新一下喵~",
        Action::Refresh,
        hits,
    );

    let (public, private, languages, stars) = app.repo_stats();
    let stats = format!(
        "公开 {}   私有 {}   语言 {}   星标合计 {}喵~",
        public,
        private,
        languages,
        format::human_count(stars)
    );
    frame.render_widget(
        Paragraph::new(format::truncate(&stats, inner.width as usize))
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 2, inner.width, 1),
    );

    let chips_y = inner.y + 3;
    draw_sort_chips(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, chips_y, inner.width, 1),
        &icons::label(icons::CLOCK, "排序喵~"),
        &[
            (
                RepoSort::Updated.label(),
                Action::SetRepoSort(RepoSort::Updated),
                app.repo_sort == RepoSort::Updated,
            ),
            (
                RepoSort::Stars.label(),
                Action::SetRepoSort(RepoSort::Stars),
                app.repo_sort == RepoSort::Stars,
            ),
            (
                RepoSort::Name.label(),
                Action::SetRepoSort(RepoSort::Name),
                app.repo_sort == RepoSort::Name,
            ),
        ],
    );

    draw_list_pagination(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, inner.y + 4, inner.width, 1),
    );
    let list = Rect::new(
        inner.x,
        inner.y + 6,
        inner.width,
        inner.height.saturating_sub(6),
    );
    app.layout.list = list;
    draw_cards(frame, app, theme, list, hits, EmptyKind::Repositories);
}

fn draw_starred_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    let summary_width = if area.width >= 118 { 25 } else { 0 };
    let detail_width = if area.width >= 112 {
        38
    } else if area.width >= 94 {
        33
    } else {
        0
    };
    let columns = Layout::horizontal([
        Constraint::Length(summary_width),
        Constraint::Min(28),
        Constraint::Length(detail_width),
    ])
    .split(area);
    if summary_width > 0 {
        draw_starred_summary(frame, app, theme, columns[0]);
    }
    draw_starred_main(frame, app, theme, columns[1], hits);
    if detail_width > 0 {
        draw_repository_detail(frame, app, theme, columns[2], hits);
        app.layout.detail = columns[2];
    }
}

fn draw_starred_summary(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(theme.color("surface0"))),
        area,
    );
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    frame.render_widget(
        Paragraph::new(icons::label(icons::STAR, "星星小账本喵~"))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    let (count, languages, stars) = app.starred_stats();
    frame.render_widget(
        Paragraph::new(count.to_string()).style(
            Style::default()
                .fg(theme.color("blue"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y + 2, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new("个仓库收着星星呢喵~").style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 3, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new(format!("{} 语言  {}喵~", icons::CODE, languages))
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 5, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new(format!(
            "{} 星标  {}喵~",
            icons::STAR,
            format::human_count(stars)
        ))
        .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 6, inner.width, 1),
    );

    let mut y = inner.y + 9;
    frame.render_widget(
        Paragraph::new(icons::label(icons::INFO, "人家的小提示喵~"))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y += 1;
    let note = [
        "收藏列表来自 GitHub喵~",
        "点仓库卡片看详情喵~",
        "双击或按 Enter 就进",
        "仓库页喵~",
    ];
    for line in note {
        frame.render_widget(
            Paragraph::new(format::truncate(line, inner.width as usize))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y += 1;
    }
}

fn draw_starred_main(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    draw_page_title(
        frame,
        app,
        theme,
        inner,
        &icons::label(icons::STAR, "星星收藏喵~"),
        "点过星星的仓库，都收在这里啦喵~",
        "刷新一下喵~",
        Action::Refresh,
        hits,
    );
    let (count, _, _) = app.starred_stats();
    frame.render_widget(
        Paragraph::new(format!("人家这里收着 {} 个仓库呢喵~", count))
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 2, inner.width, 1),
    );
    draw_sort_chips(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, inner.y + 3, inner.width, 1),
        &icons::label(icons::CLOCK, "排序喵~"),
        &[
            (
                RepoSort::Updated.label(),
                Action::SetStarSort(RepoSort::Updated),
                app.star_sort == RepoSort::Updated,
            ),
            (
                RepoSort::Stars.label(),
                Action::SetStarSort(RepoSort::Stars),
                app.star_sort == RepoSort::Stars,
            ),
            (
                RepoSort::Name.label(),
                Action::SetStarSort(RepoSort::Name),
                app.star_sort == RepoSort::Name,
            ),
        ],
    );
    draw_list_pagination(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, inner.y + 4, inner.width, 1),
    );
    let list = Rect::new(
        inner.x,
        inner.y + 6,
        inner.width,
        inner.height.saturating_sub(6),
    );
    app.layout.list = list;
    draw_cards(frame, app, theme, list, hits, EmptyKind::Starred);
}

fn draw_search_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    draw_page_title(
        frame,
        app,
        theme,
        inner,
        &icons::label(icons::SEARCH, "找仓库喵~"),
        "想找什么仓库喵~？告诉人家就好啦喵~",
        "刷新一下喵~",
        Action::Refresh,
        hits,
    );

    let search_rect = Rect::new(inner.x, inner.y + 3, inner.width, 3);
    draw_search_field(frame, app, theme, search_rect, hits);
    app.layout.search = search_rect;

    let sort_y = search_rect.y + 4;
    draw_sort_chips(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, sort_y, inner.width, 1),
        &icons::label(icons::CLOCK, "排序喵~"),
        &[
            (
                SearchSort::BestMatch.label(),
                Action::SetSearchSort(SearchSort::BestMatch),
                app.search_sort == SearchSort::BestMatch,
            ),
            (
                SearchSort::Stars.label(),
                Action::SetSearchSort(SearchSort::Stars),
                app.search_sort == SearchSort::Stars,
            ),
            (
                SearchSort::Updated.label(),
                Action::SetSearchSort(SearchSort::Updated),
                app.search_sort == SearchSort::Updated,
            ),
        ],
    );
    frame.render_widget(
        Paragraph::new(format::truncate(
            &app.search_depth_label(),
            inner.width as usize,
        ))
        .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, sort_y + 1, inner.width, 1),
    );

    draw_list_pagination(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, sort_y + 2, inner.width, 1),
    );
    let content = Rect::new(
        inner.x,
        sort_y + 3,
        inner.width,
        inner.height.saturating_sub(sort_y - inner.y + 3),
    );
    let detail_width = if area.width >= 116 {
        42
    } else if area.width >= 96 {
        35
    } else {
        0
    };
    let columns =
        Layout::horizontal([Constraint::Min(30), Constraint::Length(detail_width)]).split(content);
    app.layout.list = columns[0];
    draw_cards(frame, app, theme, columns[0], hits, EmptyKind::Search);
    if detail_width > 0 {
        draw_repository_detail(frame, app, theme, columns[1], hits);
        app.layout.detail = columns[1];
    }
}

fn draw_account_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let width = area.width.saturating_sub(8).min(112);
    let left = area.x + (area.width.saturating_sub(width)) / 2;
    let inner = Rect::new(left, area.y + 1, width, area.height.saturating_sub(2));
    draw_page_title(
        frame,
        app,
        theme,
        inner,
        &icons::label(icons::USER, "账号喵~"),
        "这里放着你的 GitHub 小档案和 API 额度喵~",
        "刷新喵~",
        Action::Refresh,
        hits,
    );

    let content = Rect::new(
        inner.x,
        inner.y + 3,
        inner.width,
        inner.height.saturating_sub(3),
    );
    if content.height < 8 {
        return;
    }
    let left_width = content.width.saturating_mul(11) / 20;
    let gap = 2;
    let profile_rect = Rect::new(content.x, content.y, left_width, content.height);
    let quota_rect = Rect::new(
        content.x + left_width + gap,
        content.y,
        content.width.saturating_sub(left_width + gap),
        content.height,
    );
    draw_account_profile(frame, app, theme, profile_rect, hits);
    draw_account_quota(frame, app, theme, quota_rect, hits);
}

fn draw_account_profile(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.color("surface0"))),
        area,
    );
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    if inner.height < 6 || inner.width < 12 {
        return;
    }
    let Some(user) = app.user.as_ref() else {
        draw_center_message(
            frame,
            theme,
            area,
            "还没有登录喵~",
            "先在终端跑一下 gh auth login，再来看看嘛喵~",
        );
        return;
    };

    frame.render_widget(
        Paragraph::new(icons::label(icons::USER, "人家的小档案喵~"))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new(format::truncate(user.display_name(), inner.width as usize)).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new(format!("@{}", user.login)).style(Style::default().fg(theme.color("blue"))),
        Rect::new(inner.x, inner.y + 2, inner.width, 1),
    );

    let mut y = inner.y + 4;
    let bio = user
        .bio
        .as_deref()
        .map(format::clean_text)
        .filter(|bio| !bio.trim().is_empty())
        .unwrap_or_else(|| "还没有写简介喵~".to_string());
    for line in format::wrap_text(&bio, inner.width as usize, 3) {
        frame.render_widget(
            Paragraph::new(line).style(Style::default().fg(theme.color("subtext0"))),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y += 1;
    }

    let facts = [
        (icons::COGS, "公司喵~", user.company.clone()),
        (icons::GLOBE, "位置喵~", user.location.clone()),
        (icons::NETWORK, "博客喵~", user.blog.clone()),
        (icons::INFO, "邮箱喵~", user.email.clone()),
        (icons::CLOCK, "加入喵~", Some(user.joined_label())),
        (
            icons::STAR,
            "计划喵~",
            user.plan
                .as_ref()
                .map(|plan| format!("{} · {} 个私有仓库喵~", plan.name, plan.private_repos)),
        ),
    ];
    y += 1;
    for (icon, label, value) in facts {
        if y >= inner.y + inner.height - 7 {
            break;
        }
        let value = value
            .map(|value| format::clean_text(&value))
            .filter(|value| !value.trim().is_empty());
        let Some(value) = value else {
            continue;
        };
        frame.render_widget(
            Paragraph::new(format::truncate(
                &format!("{icon} {label}  {value}"),
                inner.width as usize,
            ))
            .style(Style::default().fg(theme.color("subtext0"))),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y += 1;
    }

    let stats_y = inner.y + inner.height.saturating_sub(5);
    let (public, private, _, _) = app.repo_stats();
    let stats = format!(
        "{} 公开 {}   私有 {}   {} 星标 {}   {} 关注 {}喵~",
        icons::REPO,
        public,
        private,
        icons::STAR,
        app.starred.len(),
        icons::EYE,
        user.followers
    );
    frame.render_widget(
        Paragraph::new(format::truncate(&stats, inner.width as usize))
            .style(Style::default().fg(theme.color("text"))),
        Rect::new(inner.x, stats_y, inner.width, 1),
    );
    let follow = format!(
        "{} 正在关注 {}   {} 公开仓库 {}喵~",
        icons::USER,
        user.following,
        icons::REPO,
        user.public_repos
    );
    frame.render_widget(
        Paragraph::new(format::truncate(&follow, inner.width as usize))
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, stats_y + 1, inner.width, 1),
    );

    let open = icons::label(icons::EXTERNAL, "打开主页喵~");
    let open_width = format::text_width(&open) as u16 + 4;
    if stats_y + 3 < inner.y + inner.height {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(inner.x, stats_y + 3, open_width, 1),
            &open,
            Action::OpenExternal(user.html_url.clone()),
            true,
        );
    }
}

fn draw_account_quota(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.color("surface0"))),
        area,
    );
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    if inner.height < 6 || inner.width < 12 {
        return;
    }
    frame.render_widget(
        Paragraph::new(icons::label(icons::NETWORK, "GitHub API 额度喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    let source = if app.client.token_present() {
        "凭据来自 gh auth token，额度按当前账号算喵~"
    } else {
        "还没登录，看到的是匿名额度喵~"
    };
    frame.render_widget(
        Paragraph::new(format::truncate(source, inner.width as usize))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    let Some(rate) = app.rate_limit.as_ref() else {
        let detail = if app.rate_loading {
            format!("{} 人家正在问 GitHub 喵~...", app.spinner())
        } else {
            "点一下刷新，人家才去问 GitHub 喵~".to_string()
        };
        draw_center_message(frame, theme, area, "额度还没拿来喵~", &detail);
        return;
    };

    let now = crate::cache::now_epoch();
    let mut y = inner.y + 3;
    for (label, info) in rate.items() {
        if y + 4 >= inner.y + inner.height {
            break;
        }
        let percent = if info.limit == 0 {
            0
        } else {
            (info.remaining as f64 / info.limit as f64 * 100.0).round() as u32
        };
        let color = if percent >= 50 {
            theme.color("green")
        } else if percent >= 20 {
            theme.color("yellow")
        } else {
            theme.color("red")
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    label,
                    Style::default()
                        .fg(theme.color("text"))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  {} / {} 剩余", info.remaining, info.limit),
                    Style::default().fg(theme.color("subtext0")),
                ),
                Span::styled(
                    format!("  {percent}%"),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
            ])),
            Rect::new(inner.x, y, inner.width, 1),
        );
        let bar = Rect::new(inner.x, y + 1, inner.width.saturating_sub(2), 1);
        fill(frame, bar, theme.color("surface0"));
        let filled = (bar.width as u32 * percent / 100) as u16;
        if filled > 0 {
            fill(frame, Rect::new(bar.x, bar.y, filled, 1), color);
        }
        frame.render_widget(
            Paragraph::new(format!(
                "已用 {} · {}喵~",
                info.used,
                format::reset_label(info.reset, now)
            ))
            .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(inner.x, y + 2, inner.width, 1),
        );
        y += 4;
    }

    let note = format!("{} 额度接口不消耗额度，刷新随便点喵~", icons::INFO);
    if y < inner.y + inner.height {
        frame.render_widget(
            Paragraph::new(format::truncate(&note, inner.width as usize))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1),
        );
    }
    let refresh = icons::label(icons::REFRESH, "刷新额度喵~");
    let refresh_width = format::text_width(&refresh) as u16 + 4;
    if inner.y + inner.height >= 2 {
        let rect = Rect::new(
            inner.x + inner.width.saturating_sub(refresh_width),
            inner.y,
            refresh_width,
            1,
        );
        draw_button(
            frame,
            app,
            theme,
            hits,
            rect,
            &refresh,
            Action::RefreshRateLimit,
            false,
        );
    }
}

fn draw_search_field(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    let border_color = if app.search_focus {
        theme.color("blue")
    } else {
        theme.color("surface1")
    };
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
        area,
    );
    let button_width = 9;
    let input_width = area.width.saturating_sub(button_width + 5);
    let input_rect = Rect::new(area.x + 2, area.y + 1, input_width, 1);
    let placeholder = "想找什么喵~？输入后按回车嘛";
    let (visible, cursor_column) = if app.search_input.is_empty() {
        (format::truncate(placeholder, input_width as usize), 0_u16)
    } else {
        visible_input(&app.search_input, app.search_focus, input_width)
    };
    let text_style = if app.search_input.is_empty() {
        Style::default().fg(theme.color("overlay0"))
    } else {
        Style::default().fg(theme.color("text"))
    };
    frame.render_widget(Paragraph::new(visible).style(text_style), input_rect);
    let field_hit = Rect::new(area.x, area.y, area.width, area.height);
    register(hits, field_hit, Action::SearchFocus);
    let button_rect = Rect::new(
        area.x + area.width.saturating_sub(button_width + 1),
        area.y + 1,
        button_width,
        1,
    );
    draw_button(
        frame,
        app,
        theme,
        hits,
        button_rect,
        &icons::label(icons::SEARCH, "找找喵~"),
        Action::SubmitSearch,
        true,
    );
    if app.search_focus {
        let x = input_rect
            .x
            .saturating_add(cursor_column.min(input_width.saturating_sub(1)));
        frame.set_cursor_position(Position::new(x, input_rect.y));
    }
}

fn draw_list_pagination(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    hits: &mut Vec<HitBox>,
    area: Rect,
) {
    if area.height == 0 || area.width < 26 {
        return;
    }
    let prev = icons::label(icons::BACK, "上一页喵~");
    let next = icons::label(icons::EXTERNAL, "下一页喵~");
    let prev_width = format::text_width(&prev) as u16 + 4;
    let next_width = format::text_width(&next) as u16 + 4;
    let buttons_width = prev_width + next_width + 2;
    let label_width = area.width.saturating_sub(buttons_width + 1);
    frame.render_widget(
        Paragraph::new(format::truncate(&app.page_label(), label_width as usize))
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(area.x, area.y, label_width, 1),
    );
    let prev_rect = Rect::new(area.x + area.width - buttons_width, area.y, prev_width, 1);
    draw_button(
        frame,
        app,
        theme,
        hits,
        prev_rect,
        &prev,
        Action::PrevListPage,
        false,
    );
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(prev_rect.x + prev_width + 2, area.y, next_width, 1),
        &next,
        Action::NextListPage,
        true,
    );
}

fn draw_cards(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
    empty: EmptyKind,
) {
    if area.height < CARD_HEIGHT {
        return;
    }
    let indices = app.visible_indices();
    let viewport = (area.height / CARD_STEP).max(1) as usize;
    app.viewport_cards = viewport;
    let selected = app.selected_index();

    if indices.is_empty() {
        app.list_offset = 0;
        draw_empty_state(frame, app, theme, area, hits, empty);
        return;
    }

    let page = (selected / viewport).min((indices.len() - 1) / viewport);
    app.list_offset = page * viewport;

    let hovered = app.hover.clone();
    for (row, (slot, index)) in indices
        .iter()
        .enumerate()
        .skip(app.list_offset)
        .take(viewport)
        .enumerate()
    {
        let card = Rect::new(
            area.x,
            area.y + row as u16 * CARD_STEP,
            area.width.saturating_sub(1),
            CARD_HEIGHT,
        );
        let action = Action::SelectVisible(slot);
        let is_selected = slot == selected;
        let is_hovered = hovered.as_ref() == Some(&action);
        let repo = app.raw_repo(*index);
        draw_repo_card(frame, theme, card, &repo, is_selected, is_hovered);
        register(hits, card, action);
    }

    if indices.len() > viewport {
        let scroll_rect = Rect::new(
            area.x + area.width.saturating_sub(1),
            area.y,
            1,
            area.height,
        );
        let track = Style::default().fg(theme.color("surface0"));
        for y in 0..scroll_rect.height {
            frame.render_widget(
                Paragraph::new("|").style(track),
                Rect::new(scroll_rect.x, scroll_rect.y + y, 1, 1),
            );
        }
        let ratio = viewport as f32 / indices.len() as f32;
        let thumb_height = (area.height as f32 * ratio).max(1.0) as u16;
        let max_offset = indices.len().saturating_sub(viewport).max(1);
        let offset = (app.list_offset as f32 / max_offset as f32
            * (area.height.saturating_sub(thumb_height)) as f32) as u16;
        for y in 0..thumb_height {
            frame.render_widget(
                Paragraph::new("|").style(Style::default().fg(theme.color("overlay0"))),
                Rect::new(scroll_rect.x, scroll_rect.y + offset + y, 1, 1),
            );
        }
    }
}

#[derive(Clone, Copy)]
enum EmptyKind {
    Repositories,
    Starred,
    Search,
}

fn draw_empty_state(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
    kind: EmptyKind,
) {
    if matches!(kind, EmptyKind::Search) {
        draw_search_history(frame, app, theme, area, hits);
        return;
    }
    let loading = match kind {
        EmptyKind::Repositories => app.repos_loading,
        EmptyKind::Starred => app.starred_loading,
        EmptyKind::Search => app.search_loading,
    };
    let (title, detail) = if loading {
        ("人家正在搬数据喵~...", "第一次同步会慢一点点，等一等哦喵~")
    } else if !app.client.token_present() {
        (
            "还没有登录 GitHub喵~",
            "在终端跑一下 gh auth login，再重新打开人家嘛喵~",
        )
    } else {
        match kind {
            EmptyKind::Repositories => (
                "这里还没有仓库呢喵~",
                "先去 GitHub 建一个，或者按 r 刷新看看嘛喵~",
            ),
            EmptyKind::Starred => (
                "还没有收藏任何仓库喵~",
                "去 GitHub 点一颗星星，这里就会热闹起来啦喵~",
            ),
            EmptyKind::Search => ("没有搜到仓库喵~", "换个关键词再试试嘛喵~"),
        }
    };
    draw_center_message(frame, theme, area, title, detail);
}

fn draw_search_history(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    if app.search_submitted.is_none() {
        let title = if app.search_loading {
            "人家正在找仓库喵~..."
        } else {
            "还没有开始搜索呢喵~"
        };
        let mut y = area.y + 2;
        frame.render_widget(
            Paragraph::new(title).style(
                Style::default()
                    .fg(theme.color("subtext1"))
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(area.x + 1, y, area.width.saturating_sub(2), 1),
        );
        y += 1;
        frame.render_widget(
            Paragraph::new("在上面输入关键词，按回车人家就去找啦喵~")
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x + 1, y, area.width.saturating_sub(2), 1),
        );
        y += 3;
        if !app.config.search_history.is_empty() {
            frame.render_widget(
                Paragraph::new("人家最近找过喵~").style(
                    Style::default()
                        .fg(theme.color("subtext0"))
                        .add_modifier(Modifier::BOLD),
                ),
                Rect::new(area.x + 1, y, area.width.saturating_sub(2), 1),
            );
            y += 1;
            let history = app.config.search_history.clone();
            for query in history {
                if y >= area.y + area.height {
                    break;
                }
                let rect = Rect::new(area.x, y, area.width.saturating_sub(1), 1);
                let action = Action::SearchHistory(query.clone());
                let hovered = app.hover.as_ref() == Some(&action);
                if hovered {
                    fill(frame, rect, theme.color("surface0"));
                }
                frame.render_widget(
                    Paragraph::new(format!("  {query}"))
                        .style(Style::default().fg(theme.color("subtext1"))),
                    rect,
                );
                register(hits, rect, action);
                y += 1;
            }
        }
        return;
    }
    if app.search_loading {
        draw_center_message(
            frame,
            theme,
            area,
            "人家正在找仓库喵~...",
            "搜索结果一会儿就回来啦喵~",
        );
        return;
    }
    if app.search_results.is_empty() {
        draw_center_message(
            frame,
            theme,
            area,
            "没有搜到仓库喵~",
            "换个关键词，或者去掉一些限定条件试试嘛喵~",
        );
    }
}

fn draw_repo_card(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    repo: &Repo,
    selected: bool,
    hovered: bool,
) {
    let bg = if selected || hovered {
        theme.color("surface0")
    } else {
        theme.color("mantle")
    };
    fill(frame, area, bg);
    if selected {
        fill(
            frame,
            Rect::new(area.x, area.y, 1, area.height),
            theme.color("blue"),
        );
    }
    if hovered && !selected {
        fill(
            frame,
            Rect::new(area.x, area.y, 1, area.height),
            theme.color("surface2"),
        );
    }
    let left = area.x + 2;
    let width = area.width.saturating_sub(4);
    let mut title = vec![
        Span::styled(
            format!("{} ", icons::REPO),
            Style::default().fg(theme.color("blue")),
        ),
        Span::styled(
            repo.owner_name().to_string(),
            Style::default().fg(theme.color("overlay1")),
        ),
        Span::styled("/", Style::default().fg(theme.color("overlay0"))),
        Span::styled(
            repo.name.clone(),
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if repo.archived {
        title.push(Span::styled(
            format!("  {} 归档", icons::ARCHIVE),
            Style::default().fg(theme.color("peach")),
        ));
    }
    let (visibility_icon, visibility_text) = if repo.private {
        (icons::LOCK, "私有喵~")
    } else {
        (icons::GLOBE, "公开喵~")
    };
    let right = format!(
        "{} {}   {} {}",
        visibility_icon,
        visibility_text,
        icons::CLOCK,
        format::relative_time(repo.updated_at())
    );
    let right_width = format::text_width(&right) as u16 + 1;
    frame.render_widget(
        Paragraph::new(Line::from(title)),
        Rect::new(left, area.y, width.saturating_sub(right_width), 1),
    );
    frame.render_widget(
        Paragraph::new(format::truncate(&right, width as usize))
            .alignment(Alignment::Right)
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(left, area.y, width, 1),
    );

    let description =
        format::truncate(&format::clean_text(repo.description_text()), width as usize);
    frame.render_widget(
        Paragraph::new(description).style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(left, area.y + 1, width, 1),
    );

    let mut meta = vec![
        Span::styled(
            repo.language_label().to_string(),
            Style::default().fg(language_color(theme, repo.language_label())),
        ),
        Span::styled(
            format!("   {} ", icons::STAR),
            Style::default().fg(theme.color("overlay0")),
        ),
        Span::styled(
            format::human_count(repo.stargazers_count),
            Style::default().fg(theme.color("subtext1")),
        ),
        Span::styled(
            format!("   {} ", icons::FORK),
            Style::default().fg(theme.color("overlay0")),
        ),
        Span::styled(
            format::human_count(repo.forks_count),
            Style::default().fg(theme.color("subtext1")),
        ),
        Span::styled(
            format!("   {} ", icons::ISSUE),
            Style::default().fg(theme.color("overlay0")),
        ),
        Span::styled(
            format::human_count(repo.open_issues_count),
            Style::default().fg(theme.color("subtext1")),
        ),
    ];
    if repo.private {
        meta.push(Span::styled(
            format!("   {} 私有", icons::LOCK),
            Style::default().fg(theme.color("mauve")),
        ));
    }
    frame.render_widget(
        Paragraph::new(Line::from(meta)),
        Rect::new(left, area.y + 2, width, 1),
    );

    if !repo.topics.is_empty() {
        let topics = repo
            .topics
            .iter()
            .take(5)
            .map(|topic| format!("#{}", format::clean_text(topic)))
            .collect::<Vec<_>>()
            .join("  ");
        frame.render_widget(
            Paragraph::new(format::truncate(&topics, width as usize))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(left, area.y + 3, width, 1),
        );
    }
}

fn draw_repository_detail(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(theme.color("surface0"))),
        area,
    );
    let Some(selected) = app.selected_repo() else {
        draw_center_message(
            frame,
            theme,
            area,
            "挑一个仓库看看喵~",
            "点左边卡片，人家就把详情摆在这里喵~",
        );
        return;
    };
    let repo = app
        .detail
        .clone()
        .filter(|detail| detail.display_full_name() == selected.display_full_name())
        .unwrap_or(selected);
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    if inner.height < 8 {
        return;
    }
    frame.render_widget(
        Paragraph::new(icons::label(icons::REPO, "仓库小档案喵~"))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new(format::truncate(
            &format!("{} {}", icons::REPO, repo.display_full_name()),
            inner.width as usize,
        ))
        .style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    let mut y = inner.y + 2;
    let description = format::wrap_text(
        &format::clean_text(repo.description_text()),
        inner.width as usize,
        3,
    );
    for line in description {
        frame.render_widget(
            Paragraph::new(line).style(Style::default().fg(theme.color("subtext0"))),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y += 1;
    }
    if y > inner.y + 5 {
        y = inner.y + 5;
    }

    let visibility_icon = if repo.private {
        icons::LOCK
    } else {
        icons::GLOBE
    };
    let badges = format!(
        "{visibility_icon} {}   {} {}   {} {}",
        repo.visibility(),
        icons::CODE,
        repo.language_label(),
        icons::LICENSE,
        repo.license_label()
    );
    frame.render_widget(
        Paragraph::new(format::truncate(&badges, inner.width as usize))
            .style(Style::default().fg(theme.color("sapphire"))),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y += 2;
    if y + 3 < inner.y + inner.height {
        let stats = [
            (
                format!("{} 星标喵~", icons::STAR),
                format::human_count(repo.stargazers_count),
                format!("{} 复刻喵~", icons::FORK),
                format::human_count(repo.forks_count),
            ),
            (
                format!("{} 议题喵~", icons::ISSUE),
                format::human_count(repo.open_issues_count),
                format!("{} 关注喵~", icons::EYE),
                format::human_count(repo.watchers_count),
            ),
            (
                format!("{} 更新喵~", icons::CLOCK),
                format::relative_time(repo.updated_at()),
                format!("{} 分支喵~", icons::BRANCH),
                repo.default_branch.clone(),
            ),
        ];
        for (left_label, left_value, right_label, right_value) in stats {
            let line = Line::from(vec![
                Span::styled(
                    format!("{left_label} "),
                    Style::default().fg(theme.color("overlay0")),
                ),
                Span::styled(
                    format::truncate(&left_value, 10),
                    Style::default().fg(theme.color("text")),
                ),
                Span::styled(
                    format!("   {right_label} "),
                    Style::default().fg(theme.color("overlay0")),
                ),
                Span::styled(
                    format::truncate(&right_value, 12),
                    Style::default().fg(theme.color("text")),
                ),
            ]);
            frame.render_widget(Paragraph::new(line), Rect::new(inner.x, y, inner.width, 1));
            y += 1;
        }
    }

    let buttons_y = y + 1;
    if buttons_y < inner.y + inner.height {
        let mut x = inner.x;
        let mut buttons = vec![
            (
                icons::label(icons::EXTERNAL, "去主页喵~"),
                Action::OpenCurrent,
                false,
            ),
            (
                icons::label(icons::COPY, "复制喵~"),
                Action::CopyClone,
                false,
            ),
            (icons::label(icons::TAG, "打开喵~"), Action::OpenPage, true),
        ];
        if app.is_own_repo(&repo) {
            buttons.push((
                icons::label(icons::COGS, "编辑喵~"),
                Action::OpenRepoEdit,
                false,
            ));
        }
        for (label, action, primary) in buttons {
            let width = format::text_width(&label) as u16 + 4;
            if x + width > inner.x + inner.width {
                break;
            }
            draw_button(
                frame,
                app,
                theme,
                hits,
                Rect::new(x, buttons_y, width, 1),
                &label,
                action,
                primary,
            );
            x += width + 1;
        }
    }

    let releases_y = buttons_y + 2;
    if releases_y >= inner.y + inner.height {
        return;
    }
    fill(
        frame,
        Rect::new(inner.x, releases_y, inner.width, 1),
        theme.color("surface0"),
    );
    frame.render_widget(
        Paragraph::new(icons::label(icons::TAG, "发行版喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, releases_y + 1, inner.width, 1),
    );
    let loading_note = app
        .releases_loading
        .as_ref()
        .is_some_and(|name| name == &repo.display_full_name());
    if loading_note {
        frame.render_widget(
            Paragraph::new(format!("{} 人家正在取喵~...", app.spinner()))
                .style(Style::default().fg(theme.color("peach"))),
            Rect::new(inner.x, releases_y + 2, inner.width, 1),
        );
        return;
    }
    if app.releases.is_empty() {
        frame.render_widget(
            Paragraph::new("这个仓库还没有发行版呢喵~")
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(inner.x, releases_y + 2, inner.width, 1),
        );
        return;
    }
    let release_area = Rect::new(
        inner.x,
        releases_y + 2,
        inner.width,
        (inner.y + inner.height).saturating_sub(releases_y + 2),
    );
    let mut lines: Vec<Line> = Vec::new();
    for release in &app.releases {
        lines.push(Line::from(vec![
            Span::styled(
                release.tag_name.clone(),
                Style::default()
                    .fg(theme.color("blue"))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}", format::relative_time(release.date())),
                Style::default().fg(theme.color("overlay0")),
            ),
        ]));
        lines.push(Line::from(Span::styled(
            format::truncate(
                &format::clean_text(release.title()),
                release_area.width as usize,
            ),
            Style::default().fg(theme.color("subtext0")),
        )));
        lines.push(Line::from(Span::styled(
            format!("资产 {} 个喵~", release.assets.len()),
            Style::default().fg(theme.color("overlay0")),
        )));
        lines.push(Line::default());
    }
    app.detail_scroll_max = lines.len().saturating_sub(release_area.height as usize) as u16;
    app.detail_scroll = app.detail_scroll.min(app.detail_scroll_max);
    frame.render_widget(
        Paragraph::new(Text::from(lines)).scroll((app.detail_scroll, 0)),
        release_area,
    );
    if app.client.proxy().contains("v4.gh-proxy.org") {
        frame.render_widget(
            Paragraph::new("发行版资产走 v4 加速通道喵~")
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(
                inner.x,
                inner.y + inner.height.saturating_sub(1),
                inner.width,
                1,
            ),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_page_title(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    inner: Rect,
    title: &str,
    subtitle: &str,
    refresh_label: &str,
    refresh_action: Action,
    hits: &mut Vec<HitBox>,
) {
    frame.render_widget(
        Paragraph::new(title).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width.saturating_sub(12), 1),
    );
    frame.render_widget(
        Paragraph::new(format::truncate(subtitle, inner.width as usize))
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    let width = format::text_width(refresh_label) as u16 + 4;
    if inner.width > width + 4 {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(inner.x + inner.width - width, inner.y, width, 1),
            refresh_label,
            refresh_action,
            false,
        );
    }
}

fn draw_sort_chips(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    hits: &mut Vec<HitBox>,
    area: Rect,
    label: &str,
    chips: &[(&str, Action, bool)],
) {
    let label_width = format::text_width(label) as u16;
    frame.render_widget(
        Paragraph::new(label).style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x, area.y, label_width + 1, 1),
    );
    let mut x = area.x + label_width + 2;
    for (text, action, active) in chips {
        let width = format::text_width(text) as u16 + 3;
        if x + width > area.x + area.width {
            break;
        }
        let rect = Rect::new(x, area.y, width, 1);
        let hovered = app.hover.as_ref() == Some(action);
        let bg = if *active {
            theme.color("surface1")
        } else if hovered {
            theme.color("surface0")
        } else {
            theme.color("base")
        };
        fill(frame, rect, bg);
        let style = if *active {
            Style::default()
                .fg(theme.color("blue"))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.color("subtext0"))
        };
        frame.render_widget(
            Paragraph::new(format!(" {text}")).style(style),
            Rect::new(rect.x, rect.y, rect.width, 1),
        );
        register(hits, rect, action.clone());
        x += width + 1;
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_repo_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let Some(base_repo) = app.page_repo.clone() else {
        draw_center_message(
            frame,
            theme,
            area,
            "仓库不见啦喵~",
            "按 Esc 回列表再选一次嘛喵~",
        );
        return;
    };
    let repo = app
        .detail
        .clone()
        .filter(|detail| detail.display_full_name() == base_repo.display_full_name())
        .unwrap_or(base_repo);
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    if inner.height < 12 {
        draw_center_message(
            frame,
            theme,
            inner,
            "这里有点挤喵~",
            "把终端拉大一点，人家才摆得下仓库页喵~",
        );
        return;
    }

    let back_label = icons::label(icons::BACK, "返回喵~");
    let back_width = format::text_width(&back_label) as u16 + 2;
    let back_rect = Rect::new(inner.x, inner.y, back_width, 1);
    let back_hover = app.hover.as_ref() == Some(&Action::Back);
    frame.render_widget(
        Paragraph::new(back_label).style(if back_hover {
            Style::default().fg(theme.color("blue"))
        } else {
            Style::default().fg(theme.color("subtext0"))
        }),
        back_rect,
    );
    register(hits, back_rect, Action::Back);
    let crumb = format!(
        "{} / {} / {}",
        app.return_page.title(),
        repo.owner_name(),
        repo.name
    );
    frame.render_widget(
        Paragraph::new(format::truncate(
            &crumb,
            inner.width.saturating_sub(back_width + 14) as usize,
        ))
        .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(
            inner.x + back_width + 2,
            inner.y,
            inner.width.saturating_sub(back_width + 14),
            1,
        ),
    );
    let refresh_label = icons::label(icons::REFRESH, "刷新一下喵~");
    let refresh_width = format::text_width(&refresh_label) as u16 + 4;
    let refresh_x = inner.x + inner.width.saturating_sub(refresh_width);
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(refresh_x, inner.y, refresh_width, 1),
        &refresh_label,
        Action::Refresh,
        false,
    );
    if app.is_own_repo(&repo) {
        let edit_label = icons::label(icons::COGS, "编辑仓库喵~");
        let edit_width = format::text_width(&edit_label) as u16 + 4;
        let edit_x = refresh_x.saturating_sub(edit_width + 1);
        if edit_x > inner.x + back_width {
            draw_button(
                frame,
                app,
                theme,
                hits,
                Rect::new(edit_x, inner.y, edit_width, 1),
                &edit_label,
                Action::OpenRepoEdit,
                true,
            );
        }
    }

    let starred = app.is_current_starred();
    let star_label = if app.star_loading.is_some() {
        format!("{} 处理中喵~", icons::STAR)
    } else if starred {
        format!("{} 已收藏喵~", icons::STAR)
    } else {
        format!("{} 收藏喵~", icons::STAR_EMPTY)
    };
    let star_width = format::text_width(&star_label) as u16 + 4;
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(
            inner.x + inner.width.saturating_sub(star_width),
            inner.y + 1,
            star_width,
            1,
        ),
        &star_label,
        Action::ToggleStar,
        !starred,
    );

    let title = format!("{} {}", icons::REPO, repo.display_full_name());
    frame.render_widget(
        Paragraph::new(format::truncate(
            &title,
            inner.width.saturating_sub(star_width + 3) as usize,
        ))
        .style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(
            inner.x,
            inner.y + 1,
            inner.width.saturating_sub(star_width + 3),
            1,
        ),
    );
    frame.render_widget(
        Paragraph::new(format::truncate(
            &format::clean_text(repo.description_text()),
            inner.width as usize,
        ))
        .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 2, inner.width, 1),
    );
    let visibility_icon = if repo.private {
        icons::LOCK
    } else {
        icons::GLOBE
    };
    let mut facts = vec![
        format!("{visibility_icon} {}", repo.visibility()),
        format!("{} {}", icons::BRANCH, repo.default_branch),
        format!("{} {}", icons::LICENSE, repo.license_label()),
        format!(
            "{} {}",
            icons::DATABASE,
            format::format_size(repo.size.saturating_mul(1024))
        ),
    ];
    for topic in repo.topics.iter().take(3) {
        facts.push(format!("#{}", format::clean_text(topic)));
    }
    frame.render_widget(
        Paragraph::new(format::truncate(&facts.join("   "), inner.width as usize))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y + 3, inner.width, 1),
    );
    let stats = format!(
        "{} {}   {} {}   {} {}   {} {}   {} {}",
        icons::STAR,
        format::human_count(repo.stargazers_count),
        icons::FORK,
        format::human_count(repo.forks_count),
        icons::EYE,
        format::human_count(repo.watchers_count),
        icons::ISSUE,
        format::human_count(repo.open_issues_count),
        icons::CLOCK,
        format::relative_time(repo.updated_at())
    );
    frame.render_widget(
        Paragraph::new(stats).style(Style::default().fg(theme.color("text"))),
        Rect::new(inner.x, inner.y + 4, inner.width, 1),
    );

    let actions_y = inner.y + 5;
    let mut action_x = inner.x;
    let buttons = [
        (
            icons::label(icons::EXTERNAL, "去主页喵~"),
            Action::OpenCurrent,
            false,
        ),
        (
            icons::label(icons::COPY, "复制克隆喵~"),
            Action::CopyClone,
            false,
        ),
        (
            icons::label(icons::DOWNLOAD, "下载源码喵~"),
            Action::DownloadSource,
            false,
        ),
        (
            icons::label(icons::ISSUE, "议题喵~"),
            Action::OpenExternal(format!("{}/issues", repo.html_url.trim_end_matches('/'))),
            false,
        ),
        (
            icons::label(icons::PULL, "合并请求喵~"),
            Action::OpenExternal(format!("{}/pulls", repo.html_url.trim_end_matches('/'))),
            false,
        ),
        (
            icons::label(icons::COGS, "工作流喵~"),
            Action::OpenActions,
            false,
        ),
    ];
    for (label, action, primary) in buttons {
        let button_width = format::text_width(&label) as u16 + 4;
        if action_x + button_width > inner.x + inner.width {
            break;
        }
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(action_x, actions_y, button_width, 1),
            &label,
            action,
            primary,
        );
        action_x += button_width + 1;
    }
    let divider_y = inner.y + 6;
    fill(
        frame,
        Rect::new(inner.x, divider_y, inner.width, 1),
        theme.color("surface0"),
    );
    let content = Rect::new(
        inner.x,
        divider_y + 1,
        inner.width,
        (inner.y + inner.height).saturating_sub(divider_y + 1),
    );
    if content.height < 4 {
        return;
    }

    let show_readme = app.config.show_readme && content.height >= 9;
    let readme_height = (content.height * 2 / 5).max(4);
    let (main_area, readme_area) = if show_readme {
        let split = Layout::vertical([
            Constraint::Min(4),
            Constraint::Length(readme_height.min(content.height.saturating_sub(4))),
        ])
        .split(content);
        (split[0], split[1])
    } else {
        (content, Rect::default())
    };
    let columns = Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(main_area);
    let tree_rect = columns[0];
    let releases_rect = columns[1];
    app.layout.page_tree = tree_rect;
    app.layout.page_releases = releases_rect;
    app.layout.page_readme = readme_area;
    draw_file_tree_panel(frame, app, theme, &repo, tree_rect, hits);
    draw_releases_panel(frame, app, theme, &repo, releases_rect, hits);
    if show_readme {
        draw_readme_panel(frame, app, theme, readme_area, hits);
    } else {
        app.readme_scroll = 0;
        app.readme_scroll_max = 0;
    }
}

fn workflow_status_icon(status: &str, conclusion: Option<&str>) -> (&'static str, &'static str) {
    match conclusion {
        Some("success") => (icons::CHECK, "green"),
        Some("failure" | "timed_out" | "cancelled" | "startup_failure" | "action_required") => {
            (icons::WARNING, "red")
        }
        Some("skipped" | "neutral" | "stale") => (icons::INFO, "overlay0"),
        _ => match status {
            "in_progress" => (icons::CLOCK, "peach"),
            "queued" | "waiting" | "requested" | "pending" => (icons::CLOCK, "overlay0"),
            _ => (icons::INFO, "blue"),
        },
    }
}

fn draw_actions_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    if inner.height < 8 {
        draw_center_message(
            frame,
            theme,
            inner,
            "这里有点挤喵~",
            "把终端拉大一点，人家才摆得下工作流页喵~",
        );
        return;
    }

    let back_label = icons::label(icons::BACK, "返回喵~");
    let back_width = format::text_width(&back_label) as u16 + 2;
    let back_rect = Rect::new(inner.x, inner.y, back_width, 1);
    let back_hover = app.hover.as_ref() == Some(&Action::Back);
    frame.render_widget(
        Paragraph::new(back_label).style(if back_hover {
            Style::default().fg(theme.color("blue"))
        } else {
            Style::default().fg(theme.color("subtext0"))
        }),
        back_rect,
    );
    register(hits, back_rect, Action::Back);

    let refresh_label = icons::label(icons::REFRESH, "刷新喵~");
    let refresh_width = format::text_width(&refresh_label) as u16 + 4;
    let refresh_x = inner.x + inner.width.saturating_sub(refresh_width);
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(refresh_x, inner.y, refresh_width, 1),
        &refresh_label,
        Action::Refresh,
        false,
    );

    let run_label = icons::label(icons::PLAY, "运行工作流喵~");
    let run_width = format::text_width(&run_label) as u16 + 4;
    let run_x = refresh_x.saturating_sub(run_width + 1);
    let title_x = inner.x + back_width + 2;
    let title_end = if run_x > title_x {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(run_x, inner.y, run_width, 1),
            &run_label,
            Action::OpenWorkflowRunner,
            true,
        );
        run_x
    } else {
        refresh_x
    };

    let repo = app
        .action_repo
        .clone()
        .unwrap_or_else(|| "还没有选仓库".to_string());
    let title = format!("{} 工作流喵~  {repo}", icons::COGS);
    let title_width = title_end.saturating_sub(title_x + 1) as usize;
    frame.render_widget(
        Paragraph::new(format::truncate(&title, title_width)).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(title_x, inner.y, title_width as u16, 1),
    );
    frame.render_widget(
        Paragraph::new("这里是当前仓库的 Actions 运行记录喵~")
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    fill(
        frame,
        Rect::new(inner.x, inner.y + 2, inner.width, 1),
        theme.color("surface0"),
    );

    let content = Rect::new(
        inner.x,
        inner.y + 3,
        inner.width,
        inner.height.saturating_sub(3),
    );
    if content.height < 4 {
        return;
    }
    let columns = Layout::horizontal([
        Constraint::Percentage(44),
        Constraint::Length(1),
        Constraint::Min(24),
    ])
    .split(content);
    draw_action_runs_panel(frame, app, theme, columns[0], hits);
    draw_action_jobs_panel(frame, app, theme, columns[2], hits);
}

fn draw_action_runs_panel(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    if area.width < 8 || area.height < 3 {
        return;
    }
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Paragraph::new(icons::label(icons::COGS, "运行记录喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
    );
    frame.render_widget(
        Paragraph::new(format!("{} 条喵~", app.action_runs.len()))
            .alignment(Alignment::Right)
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
    );
    fill(
        frame,
        Rect::new(area.x, area.y + 1, area.width, 1),
        theme.color("surface0"),
    );
    let body = Rect::new(
        area.x + 1,
        area.y + 2,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    );
    if app.action_runs.is_empty() {
        let (title, detail) = if app.action_runs_loading {
            ("人家正在翻工作流记录喵~...", "第一次同步要等一小会儿哦喵~")
        } else {
            (
                "还没有运行记录喵~",
                "这个仓库还没跑过 Actions，或者人家没权限看喵~",
            )
        };
        draw_center_message(frame, theme, body, title, detail);
        return;
    }

    let rows = (body.height / 2).max(1) as usize;
    let selected = app.action_selected.min(app.action_runs.len() - 1);
    let start = selected.saturating_sub(rows - 1);
    let end = (start + rows).min(app.action_runs.len());
    for (offset, index) in (start..end).enumerate() {
        let y = body.y + offset as u16 * 2;
        if y + 1 >= body.y + body.height {
            break;
        }
        let rect = Rect::new(body.x, y, body.width, 2);
        if index == selected {
            fill(frame, rect, theme.color("surface0"));
        }
        let run = &app.action_runs[index];
        let (icon, color) = workflow_status_icon(&run.status, run.conclusion.as_deref());
        let title_width = body.width.saturating_sub(5) as usize;
        let title = Line::from(vec![
            Span::styled(format!("{icon} "), Style::default().fg(theme.color(color))),
            Span::styled(
                format!("#{} ", run.run_number),
                Style::default()
                    .fg(theme.color("blue"))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format::truncate(&format::clean_text(run.title()), title_width),
                Style::default().fg(theme.color("text")),
            ),
        ]);
        frame.render_widget(
            Paragraph::new(title),
            Rect::new(body.x + 1, y, body.width.saturating_sub(1), 1),
        );
        let event = if run.event.trim().is_empty() {
            "手动"
        } else {
            run.event.as_str()
        };
        let branch = if run.branch().is_empty() {
            "无分支"
        } else {
            run.branch()
        };
        let meta = format!(
            "   {} · {} · {} · {}",
            format::clean_text(event),
            format::clean_text(branch),
            run.status_label(),
            format::relative_time(&run.created_at)
        );
        frame.render_widget(
            Paragraph::new(format::truncate(&meta, body.width as usize))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(body.x + 1, y + 1, body.width.saturating_sub(1), 1),
        );
        register(hits, rect, Action::SelectActionRun(index));
    }
}

fn draw_action_jobs_panel(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    if area.width < 8 || area.height < 3 {
        return;
    }
    fill(frame, area, theme.color("mantle"));
    let Some(run) = app.selected_action_run() else {
        frame.render_widget(
            Paragraph::new(icons::label(icons::CODE, "任务与步骤喵~")).style(
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
        );
        draw_center_message(
            frame,
            theme,
            Rect::new(
                area.x + 1,
                area.y + 2,
                area.width.saturating_sub(2),
                area.height.saturating_sub(2),
            ),
            "先在左边挑一条运行记录喵~",
            "挑好之后人家会把任务和步骤列出来喵~",
        );
        return;
    };
    frame.render_widget(
        Paragraph::new(icons::label(icons::CODE, "任务与步骤喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
    );
    frame.render_widget(
        Paragraph::new(format::truncate(
            &format!("#{} {}", run.run_number, format::clean_text(run.title())),
            area.width.saturating_sub(2) as usize,
        ))
        .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), 1),
    );
    fill(
        frame,
        Rect::new(area.x, area.y + 2, area.width, 1),
        theme.color("surface0"),
    );
    let body = Rect::new(
        area.x + 1,
        area.y + 3,
        area.width.saturating_sub(2),
        area.height.saturating_sub(3),
    );
    if app.action_jobs.is_empty() {
        let (title, detail) = if app.action_jobs_loading == Some(run.id) {
            ("人家正在读任务详情喵~...", "任务多的时候要等一小会儿哦喵~")
        } else {
            (
                "还没有任务详情喵~",
                "这条运行可能还在排队，或者接口没给任务喵~",
            )
        };
        draw_center_message(frame, theme, body, title, detail);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    let mut actions: Vec<(usize, Action)> = Vec::new();
    for job in &app.action_jobs {
        let (icon, color) = workflow_status_icon(&job.status, job.conclusion.as_deref());
        let duration = job
            .started_at
            .as_deref()
            .and_then(|start| format::duration_label(start, job.completed_at.as_deref()))
            .unwrap_or_else(|| "未开始".to_string());
        let name = format::truncate(
            &format::clean_text(&job.name),
            body.width.saturating_sub(20) as usize,
        );
        lines.push(Line::from(vec![
            Span::styled(format!("{icon} "), Style::default().fg(theme.color(color))),
            Span::styled(name, Style::default().fg(theme.color("text"))),
            Span::styled(
                format!("  {}  {}", job.status_label(), duration),
                Style::default().fg(theme.color("overlay0")),
            ),
        ]));
        if !job.html_url.is_empty() {
            actions.push((lines.len() - 1, Action::OpenExternal(job.html_url.clone())));
        }
        for step in &job.steps {
            let (step_icon, step_color) =
                workflow_status_icon(&step.status, step.conclusion.as_deref());
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {step_icon} "),
                    Style::default().fg(theme.color(step_color)),
                ),
                Span::styled(
                    format::truncate(
                        &format::clean_text(&step.name),
                        body.width.saturating_sub(16) as usize,
                    ),
                    Style::default().fg(theme.color("subtext0")),
                ),
                Span::styled(
                    format!("  {}", step.status_label()),
                    Style::default().fg(theme.color("overlay0")),
                ),
            ]));
        }
        lines.push(Line::default());
    }
    lines.push(Line::from(Span::styled(
        "点任务行或按 Enter 去浏览器看完整日志喵~",
        Style::default().fg(theme.color("overlay0")),
    )));
    app.action_jobs_scroll_max = lines.len().saturating_sub(body.height as usize) as u16;
    app.action_jobs_scroll = app.action_jobs_scroll.min(app.action_jobs_scroll_max);
    render_scrolled(
        frame,
        body,
        lines,
        actions,
        app.action_jobs_scroll,
        theme,
        hits,
    );
}

fn draw_cache_overlay(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    let groups = cache_groups();
    let width = area.width.saturating_sub(12).min(72);
    let height = (groups.len() as u16 + 7).min(area.height.saturating_sub(4));
    if width < 30 || height < 6 {
        return;
    }
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, rect);
    fill(frame, rect, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.color("blue"))),
        rect,
    );
    let inner = Rect::new(
        rect.x + 2,
        rect.y + 1,
        rect.width.saturating_sub(4),
        rect.height.saturating_sub(2),
    );
    frame.render_widget(
        Paragraph::new(icons::label(icons::DATABASE, "缓存管理喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new(format!(
            "共 {} 个文件 · {}喵~",
            app.cache_stats.files,
            format::format_size(app.cache_stats.bytes)
        ))
        .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    let list_top = inner.y + 3;
    let bottom = inner.y + inner.height.saturating_sub(1);
    for (index, group) in groups.iter().enumerate() {
        let y = list_top + index as u16;
        if y >= bottom {
            break;
        }
        let stats = app.cache_group_stats(index);
        let selected = index == app.cache_row;
        let row = Rect::new(inner.x, y, inner.width, 1);
        if selected {
            fill(frame, row, theme.color("surface0"));
        }
        let label = format!(
            "{} {:<10}  {:>3} 个文件 · {}",
            icons::DATABASE,
            group.label,
            stats.files,
            format::format_size(stats.bytes)
        );
        frame.render_widget(
            Paragraph::new(format::truncate(&label, inner.width as usize)).style(if selected {
                Style::default()
                    .fg(theme.color("blue"))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.color("subtext0"))
            }),
            row,
        );
        register(hits, row, Action::SelectCacheRow(index));
    }

    let mut x = inner.x;
    let clear_label = icons::label(icons::TRASH, "清掉选中喵~");
    let clear_width = format::text_width(&clear_label) as u16 + 4;
    if x + clear_width <= inner.x + inner.width {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(x, bottom, clear_width, 1),
            &clear_label,
            Action::ClearCacheGroup(app.cache_row),
            false,
        );
        x += clear_width + 1;
    }
    let open_label = icons::label(icons::FOLDER_OPEN, "缓存目录喵~");
    let open_width = format::text_width(&open_label) as u16 + 4;
    if x + open_width <= inner.x + inner.width {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(x, bottom, open_width, 1),
            &open_label,
            Action::OpenCacheDir,
            false,
        );
        x += open_width + 1;
    }
    let close_label = "关掉喵~";
    let close_width = format::text_width(close_label) as u16 + 4;
    if x + close_width <= inner.x + inner.width {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(x, bottom, close_width, 1),
            close_label,
            Action::CloseOverlay,
            false,
        );
    }
    app.layout.overlay = rect;
}

#[allow(clippy::too_many_arguments)]
fn draw_workflow_field(
    frame: &mut Frame,
    theme: &Theme,
    hits: &mut Vec<HitBox>,
    x: u16,
    y: u16,
    width: u16,
    label: &str,
    input: &TextInput,
    editing: bool,
    placeholder: &str,
    action: Action,
) {
    let label_width = format::text_width(label) as u16;
    frame.render_widget(
        Paragraph::new(format::truncate(label, width as usize))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(x, y, width, 1),
    );
    let input_x = x + label_width.saturating_add(2);
    let input_width = width.saturating_sub(label_width.saturating_add(2));
    let input_rect = Rect::new(input_x, y, input_width, 1);
    if input_width > 0 {
        fill(
            frame,
            input_rect,
            if editing {
                theme.color("surface1")
            } else {
                theme.color("surface0")
            },
        );
        let empty = input.is_empty();
        let shown = if empty {
            placeholder.to_string()
        } else {
            input.value.clone()
        };
        frame.render_widget(
            Paragraph::new(format::truncate(&shown, input_width as usize)).style(if empty {
                Style::default().fg(theme.color("overlay0"))
            } else {
                Style::default().fg(theme.color("text"))
            }),
            input_rect,
        );
        if editing {
            let cursor_x = if empty {
                input_x
            } else {
                input_x + (input.cursor as u16).min(input_width.saturating_sub(1))
            };
            frame.set_cursor_position(Position::new(cursor_x, y));
        }
    }
    register(hits, Rect::new(x, y, width, 1), action);
}

fn draw_workflow_overlay(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    let width = area.width.saturating_sub(10).min(80);
    let height = area.height.saturating_sub(6).min(26);
    if width < 40 || height < 14 {
        return;
    }
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, rect);
    fill(frame, rect, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.color("blue"))),
        rect,
    );
    let inner = Rect::new(
        rect.x + 2,
        rect.y + 1,
        rect.width.saturating_sub(4),
        rect.height.saturating_sub(2),
    );
    frame.render_widget(
        Paragraph::new(icons::label(icons::PLAY, "运行工作流喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    let repo = app
        .action_repo
        .clone()
        .unwrap_or_else(|| "还没有选仓库".to_string());
    frame.render_widget(
        Paragraph::new(format::truncate(
            &format!("{repo} · 只有支持 workflow_dispatch 的工作流才能手动运行喵~"),
            inner.width as usize,
        ))
        .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    let list_top = inner.y + 3;
    let list_height = inner.height.saturating_sub(11).max(2);
    let list_area = Rect::new(inner.x, list_top, inner.width, list_height);
    if app.action_workflows.is_empty() {
        let (title, detail) = if app.action_workflows_loading {
            (
                "人家正在取工作流列表喵~...",
                "仓库里的 YAML 有点多的话要等一小会儿喵~",
            )
        } else {
            (
                "这个仓库还没有工作流喵~",
                "先往 .github/workflows 里放一个 workflow_dispatch 吧喵~",
            )
        };
        draw_center_message(frame, theme, list_area, title, detail);
    } else {
        let rows = list_height as usize;
        let selected = app.workflow_selected.min(app.action_workflows.len() - 1);
        let start = selected.saturating_sub(rows.saturating_sub(1));
        let end = (start + rows).min(app.action_workflows.len());
        for (offset, index) in (start..end).enumerate() {
            let y = list_area.y + offset as u16;
            let row = Rect::new(list_area.x, y, list_area.width, 1);
            let workflow = &app.action_workflows[index];
            if index == selected {
                fill(frame, row, theme.color("surface0"));
            }
            let name = if workflow.name.trim().is_empty() {
                workflow.file_name()
            } else {
                workflow.name.as_str()
            };
            let label = format!(
                "{} {}  {}  {}",
                icons::COGS,
                format::clean_text(name),
                workflow.state_label(),
                format::clean_text(&workflow.path)
            );
            let style = if index == selected {
                Style::default()
                    .fg(theme.color("blue"))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.color("subtext0"))
            };
            frame.render_widget(
                Paragraph::new(format::truncate(&label, list_area.width as usize)).style(style),
                row,
            );
            register(hits, row, Action::SelectWorkflow(index));
        }
    }

    let ref_y = list_top + list_height + 1;
    draw_workflow_field(
        frame,
        theme,
        hits,
        inner.x,
        ref_y,
        inner.width,
        "分支/标签喵~",
        &app.workflow_ref,
        app.workflow_ref_editing,
        "默认分支喵~",
        Action::EditWorkflowRef,
    );
    draw_workflow_field(
        frame,
        theme,
        hits,
        inner.x,
        ref_y + 1,
        inner.width,
        "inputs JSON喵~",
        &app.workflow_inputs,
        app.workflow_inputs_editing,
        "{}",
        Action::EditWorkflowInputs,
    );

    if let Some(error) = app.workflow_error.as_deref() {
        frame.render_widget(
            Paragraph::new(format::truncate(error, inner.width as usize))
                .style(Style::default().fg(theme.color("red"))),
            Rect::new(inner.x, ref_y + 2, inner.width, 1),
        );
    }
    frame.render_widget(
        Paragraph::new(format::truncate(
            "j/k 挑  Tab 换输入框  Enter 运行  r 刷新列表  o 开工作流  Esc 关掉喵~",
            inner.width as usize,
        ))
        .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, ref_y + 3, inner.width, 1),
    );

    let bottom = inner.y + inner.height.saturating_sub(1);
    let mut x = inner.x;
    let run_label = if app.workflow_running {
        format!("{} 提交中喵~", app.spinner())
    } else {
        icons::label(icons::PLAY, "运行喵~")
    };
    let run_width = format::text_width(&run_label) as u16 + 4;
    if x + run_width <= inner.x + inner.width {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(x, bottom, run_width, 1),
            &run_label,
            Action::RunWorkflow,
            true,
        );
        x += run_width + 1;
    }
    let refresh_label = icons::label(icons::REFRESH, "刷新列表喵~");
    let refresh_width = format::text_width(&refresh_label) as u16 + 4;
    if x + refresh_width <= inner.x + inner.width {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(x, bottom, refresh_width, 1),
            &refresh_label,
            Action::RefreshWorkflows,
            false,
        );
        x += refresh_width + 1;
    }
    let close_label = "关掉喵~";
    let close_width = format::text_width(close_label) as u16 + 4;
    if x + close_width <= inner.x + inner.width {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(x, bottom, close_width, 1),
            close_label,
            Action::CloseOverlay,
            false,
        );
    }
    app.layout.overlay = rect;
}

fn draw_file_tree_panel(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    repo: &Repo,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    if area.width < 8 || area.height < 3 {
        return;
    }
    fill(frame, area, theme.color("mantle"));
    register(hits, area, Action::FocusRepoPane(RepoPane::Tree));
    let focused = app.repo_pane == RepoPane::Tree;
    let title = icons::label(icons::FOLDER_OPEN, "文件树喵~");
    frame.render_widget(
        Paragraph::new(format!(
            "{}   {} {}",
            title,
            icons::BRANCH,
            format::truncate(&repo.default_branch, 18)
        ))
        .style(if focused {
            Style::default()
                .fg(theme.color("blue"))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.color("text"))
        }),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
    );
    fill(
        frame,
        Rect::new(area.x, area.y + 1, area.width, 1),
        theme.color("surface0"),
    );
    let body = Rect::new(
        area.x + 1,
        area.y + 2,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    );
    if !app.config.show_file_tree {
        draw_center_message(
            frame,
            theme,
            body,
            "文件树预览没开喵~",
            "到设置页里手动打开，人家才会去取文件树哦喵~",
        );
        app.tree_scroll = 0;
        app.tree_scroll_max = 0;
        return;
    }
    if let Some(loading) = app.tree_loading.as_ref()
        && loading == &repo.display_full_name()
    {
        frame.render_widget(
            Paragraph::new(format!("{} 人家正在翻文件树喵~...", app.spinner()))
                .style(Style::default().fg(theme.color("peach"))),
            Rect::new(body.x, body.y, body.width, 1),
        );
        return;
    }
    let Some(tree) = app.tree.as_ref() else {
        draw_center_message(
            frame,
            theme,
            body,
            "文件树还没到喵~",
            "点一下刷新，或者稍后再看看嘛喵~",
        );
        return;
    };
    let mut entries: Vec<_> = tree
        .tree
        .iter()
        .filter(|entry| entry.kind == "blob" || entry.kind == "tree")
        .collect();
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    if entries.is_empty() {
        draw_center_message(
            frame,
            theme,
            body,
            "仓库里空空的喵~",
            "这个分支下面还没有文件呢喵~",
        );
        return;
    }
    let mut lines: Vec<Line> = Vec::new();
    let mut actions = Vec::new();
    for entry in entries.iter().take(1200) {
        let depth = entry.path.matches('/').count().min(10);
        let name = format::clean_text(entry.path.rsplit('/').next().unwrap_or(&entry.path));
        let directory = entry.kind == "tree";
        let icon = if directory {
            icons::FOLDER_OPEN
        } else {
            file_icon(&name)
        };
        let indent = "  ".repeat(depth);
        let suffix = if !directory {
            entry
                .size
                .map(|size| format!("  {}", format::format_size(size)))
                .unwrap_or_default()
        } else {
            String::new()
        };
        let action = Action::OpenTreePath {
            path: entry.path.clone(),
            directory,
        };
        actions.push((lines.len(), action));
        lines.push(Line::from(vec![
            Span::styled(indent, Style::default().fg(theme.color("surface2"))),
            Span::styled(
                format!("{icon} "),
                Style::default().fg(if directory {
                    theme.color("blue")
                } else {
                    theme.color("overlay1")
                }),
            ),
            Span::styled(
                format::truncate(
                    &format!("{name}{suffix}"),
                    body.width.saturating_sub(3) as usize,
                ),
                Style::default().fg(if directory {
                    theme.color("subtext1")
                } else {
                    theme.color("subtext0")
                }),
            ),
        ]));
    }
    if tree.truncated || entries.len() > 1200 {
        lines.push(Line::from(Span::styled(
            "文件太多啦，只展示了一部分喵~",
            Style::default().fg(theme.color("peach")),
        )));
    }
    app.tree_scroll_max = lines.len().saturating_sub(body.height as usize) as u16;
    app.tree_scroll = app.tree_scroll.min(app.tree_scroll_max);
    render_scrolled(frame, body, lines, actions, app.tree_scroll, theme, hits);
}

fn draw_releases_panel(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    repo: &Repo,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    if area.width < 8 || area.height < 3 {
        return;
    }
    fill(frame, area, theme.color("mantle"));
    register(hits, area, Action::FocusRepoPane(RepoPane::Releases));
    let focused = app.repo_pane == RepoPane::Releases;
    frame.render_widget(
        Paragraph::new(icons::label(icons::TAG, "发行版喵~")).style(if focused {
            Style::default()
                .fg(theme.color("blue"))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.color("text"))
        }),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
    );
    let dir_label = format!("{} {}", icons::DOWNLOAD, app.download_path_label());
    frame.render_widget(
        Paragraph::new(format::truncate(
            &dir_label,
            area.width.saturating_sub(2) as usize,
        ))
        .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), 1),
    );
    fill(
        frame,
        Rect::new(area.x, area.y + 2, area.width, 1),
        theme.color("surface0"),
    );
    let body = Rect::new(
        area.x + 1,
        area.y + 3,
        area.width.saturating_sub(2),
        area.height.saturating_sub(4),
    );
    if let Some(loading) = app.releases_loading.as_ref()
        && loading == &repo.display_full_name()
    {
        frame.render_widget(
            Paragraph::new(format!("{} 人家正在取发行版喵~...", app.spinner()))
                .style(Style::default().fg(theme.color("peach"))),
            Rect::new(body.x, body.y, body.width, 1),
        );
        return;
    }
    if app.releases.is_empty() {
        draw_center_message(
            frame,
            theme,
            body,
            "还没有发行版喵~",
            "等作者发一个，或者按 r 再刷新一下嘛喵~",
        );
        return;
    }
    let mut lines: Vec<Line> = Vec::new();
    let mut actions = Vec::new();
    for (index, release) in app.releases.iter().enumerate() {
        let mut title = vec![Span::styled(
            release.tag_name.clone(),
            Style::default()
                .fg(theme.color("blue"))
                .add_modifier(Modifier::BOLD),
        )];
        if index == 0 {
            title.push(Span::styled(
                "  最新",
                Style::default().fg(theme.color("green")),
            ));
        }
        if release.prerelease {
            title.push(Span::styled(
                "  预发布",
                Style::default().fg(theme.color("peach")),
            ));
        }
        title.push(Span::styled(
            format!("   {}", format::relative_time(release.date())),
            Style::default().fg(theme.color("overlay0")),
        ));
        actions.push((lines.len(), Action::OpenReleasePage(index)));
        lines.push(Line::from(title));
        lines.push(Line::from(Span::styled(
            format::truncate(&format::clean_text(release.title()), body.width as usize),
            Style::default().fg(theme.color("subtext0")),
        )));
        let excerpt = release
            .body
            .as_deref()
            .map(plain_excerpt)
            .map(|text| format::clean_text(&text))
            .unwrap_or_default();
        if !excerpt.is_empty() {
            lines.push(Line::from(Span::styled(
                format::truncate(&excerpt, body.width as usize),
                Style::default().fg(theme.color("overlay0")),
            )));
        }
        if release.assets.is_empty() {
            lines.push(Line::from(Span::styled(
                "  没有二进制资产",
                Style::default().fg(theme.color("overlay0")),
            )));
        } else {
            let expanded = app.expanded_releases.contains(&release.tag_name);
            let shown = if expanded {
                release.assets.len()
            } else {
                release.assets.len().min(5)
            };
            for (asset_index, asset) in release.assets.iter().take(shown).enumerate() {
                let line = Line::from(vec![
                    Span::styled(
                        format!("  {} ", icons::DOWNLOAD),
                        Style::default().fg(theme.color("blue")),
                    ),
                    Span::styled(
                        format::truncate(
                            &format::clean_text(&asset.name),
                            body.width.saturating_sub(20) as usize,
                        ),
                        Style::default().fg(theme.color("subtext1")),
                    ),
                    Span::styled(
                        format!("  {}", format::format_size(asset.size)),
                        Style::default().fg(theme.color("overlay0")),
                    ),
                ]);
                actions.push((
                    lines.len(),
                    Action::DownloadAsset {
                        release: index,
                        asset: asset_index,
                    },
                ));
                lines.push(line);
            }
            if release.assets.len() > 5 {
                let (text, color) = if expanded {
                    ("  收起资产喵~".to_string(), theme.color("overlay0"))
                } else {
                    (
                        format!(
                            "  还有 {} 个资产，点这里全部展开喵~",
                            release.assets.len() - 5
                        ),
                        theme.color("blue"),
                    )
                };
                actions.push((lines.len(), Action::ToggleReleaseAssets(index)));
                lines.push(Line::from(Span::styled(
                    text,
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )));
            }
        }
        lines.push(Line::default());
    }
    lines.push(Line::from(Span::styled(
        "点资产就会下载到上面的目录里，同名也不会覆盖哦喵~",
        Style::default().fg(theme.color("overlay0")),
    )));
    app.page_scroll_max = lines.len().saturating_sub(body.height as usize) as u16;
    app.page_scroll = app.page_scroll.min(app.page_scroll_max);
    render_scrolled(frame, body, lines, actions, app.page_scroll, theme, hits);
}

fn draw_readme_panel(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    if area.width < 8 || area.height < 3 {
        return;
    }
    fill(frame, area, theme.color("mantle"));
    register(hits, area, Action::FocusRepoPane(RepoPane::Readme));
    let focused = app.repo_pane == RepoPane::Readme;
    let repo_name = app
        .page_repo
        .as_ref()
        .map(|repo| repo.display_full_name())
        .unwrap_or_default();
    frame.render_widget(
        Paragraph::new(icons::label(icons::BOOK, "README")).style(if focused {
            Style::default()
                .fg(theme.color("blue"))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.color("text"))
        }),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
    );
    if let Some(readme) = app.readme.as_ref() {
        frame.render_widget(
            Paragraph::new(format::truncate(&format::clean_text(&readme.path), 30))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x + 12, area.y, area.width.saturating_sub(28), 1),
        );
        let open_label = icons::label(icons::EXTERNAL, "去 GitHub 看喵~");
        let open_width = format::text_width(&open_label) as u16 + 2;
        let open_rect = Rect::new(
            area.x + area.width.saturating_sub(open_width),
            area.y,
            open_width,
            1,
        );
        let hovered = app.hover == Some(Action::OpenExternal(readme.html_url.clone()));
        frame.render_widget(
            Paragraph::new(open_label).style(if hovered {
                Style::default().fg(theme.color("blue"))
            } else {
                Style::default().fg(theme.color("overlay0"))
            }),
            open_rect,
        );
        register(
            hits,
            open_rect,
            Action::OpenExternal(readme.html_url.clone()),
        );
    }
    fill(
        frame,
        Rect::new(area.x, area.y + 1, area.width, 1),
        theme.color("surface0"),
    );
    let body = Rect::new(
        area.x + 1,
        area.y + 2,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    );
    if let Some(loading) = app.readme_loading.as_ref()
        && loading == &repo_name
    {
        frame.render_widget(
            Paragraph::new(format!("{} 人家正在读 README 喵~...", app.spinner()))
                .style(Style::default().fg(theme.color("peach"))),
            Rect::new(body.x, body.y, body.width, 1),
        );
        return;
    }
    let Some(readme) = app.readme.as_ref() else {
        draw_center_message(
            frame,
            theme,
            body,
            "README 还没到喵~",
            "点一下刷新，或者稍后再看看嘛喵~",
        );
        return;
    };
    let lines = markdown_lines(&readme.content, body.width as usize, theme);
    app.readme_scroll_max = lines.len().saturating_sub(body.height as usize) as u16;
    app.readme_scroll = app.readme_scroll.min(app.readme_scroll_max);
    frame.render_widget(
        Paragraph::new(Text::from(lines)).scroll((app.readme_scroll, 0)),
        body,
    );
}

fn file_icon(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".md") || lower.ends_with(".markdown") {
        icons::BOOK
    } else if lower.ends_with(".rs")
        || lower.ends_with(".py")
        || lower.ends_with(".js")
        || lower.ends_with(".ts")
        || lower.ends_with(".go")
        || lower.ends_with(".toml")
        || lower.ends_with(".json")
        || lower.ends_with(".yaml")
        || lower.ends_with(".yml")
    {
        icons::CODE_FILE
    } else {
        icons::FILE
    }
}

fn markdown_lines(text: &str, width: usize, theme: &Theme) -> Vec<Line<'static>> {
    let width = width.max(10);
    let mut out: Vec<Line<'static>> = Vec::new();
    let mut in_code = false;
    for raw in text.lines() {
        let trimmed = raw.trim_end();
        if trimmed.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if trimmed.trim_start().starts_with("<!--") {
            continue;
        }
        let clean = format::clean_text(trimmed);
        if clean.is_empty() {
            if out.last().is_some_and(|line| line.width() > 0) {
                out.push(Line::default());
            }
            continue;
        }
        if in_code {
            for line in format::wrap_text(&clean, width, 4) {
                out.push(Line::from(Span::styled(
                    line,
                    Style::default().fg(theme.color("overlay1")),
                )));
            }
            continue;
        }
        let heading = clean.starts_with('#');
        let list = clean.starts_with("- ") || clean.starts_with("* ") || clean.starts_with("+ ");
        let plain = if heading {
            strip_markdown_inline(clean.trim_start_matches('#').trim())
        } else {
            strip_markdown_inline(&clean)
        };
        let prefix = if heading {
            "".to_string()
        } else if list {
            "  • ".to_string()
        } else {
            "".to_string()
        };
        let text = if list {
            plain
                .trim_start_matches(['-', '*', '+'])
                .trim_start()
                .to_string()
        } else {
            plain
        };
        let style = if heading {
            Style::default()
                .fg(theme.color("blue"))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.color("subtext0"))
        };
        for line in format::wrap_text(&format!("{prefix}{text}"), width, 3) {
            out.push(Line::from(Span::styled(line, style)));
        }
        if heading {
            out.push(Line::default());
        }
        if out.len() > 1600 {
            break;
        }
    }
    if out.is_empty() {
        out.push(Line::from("这个 README 空空的，人家什么也没读到喵~"));
    }
    out
}

fn strip_markdown_images(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '!' && chars.get(index + 1) == Some(&'[') {
            let mut cursor = index + 2;
            let mut depth = 1;
            while cursor < chars.len() && depth > 0 {
                match chars[cursor] {
                    '[' => depth += 1,
                    ']' => depth -= 1,
                    _ => {}
                }
                cursor += 1;
            }
            if chars.get(cursor) == Some(&'(') {
                cursor += 1;
                let mut paren = 1;
                while cursor < chars.len() && paren > 0 {
                    match chars[cursor] {
                        '(' => paren += 1,
                        ')' => paren -= 1,
                        _ => {}
                    }
                    cursor += 1;
                }
            }
            index = cursor;
            continue;
        }
        out.push(chars[index]);
        index += 1;
    }
    out
}

fn strip_markdown_inline(text: &str) -> String {
    let text = strip_markdown_images(text);
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '!' if chars.peek() == Some(&'[') => {
                chars.next();
                let mut alt = String::new();
                let mut depth = 1;
                for inner in chars.by_ref() {
                    if inner == '[' {
                        depth += 1;
                    } else if inner == ']' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    alt.push(inner);
                }
                if chars.peek() == Some(&'(') {
                    chars.next();
                    for inner in chars.by_ref() {
                        if inner == ')' {
                            break;
                        }
                    }
                }
                out.push_str(&alt);
            }
            '[' => {
                let mut label = String::new();
                let mut found = false;
                for inner in chars.by_ref() {
                    if inner == ']' {
                        found = true;
                        break;
                    }
                    label.push(inner);
                }
                if found && chars.peek() == Some(&'(') {
                    chars.next();
                    for inner in chars.by_ref() {
                        if inner == ')' {
                            break;
                        }
                    }
                }
                out.push_str(&label);
            }
            '*' | '_' | '`' | '~' => {}
            '<' => {
                for inner in chars.by_ref() {
                    if inner == '>' {
                        break;
                    }
                }
            }
            _ => out.push(ch),
        }
    }
    let cleaned = out.trim().to_string();
    format::clean_text(&cleaned)
}

fn draw_settings_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let width = area.width.saturating_sub(8).min(100);
    let left = area.x + (area.width.saturating_sub(width)) / 2;
    let inner = Rect::new(left, area.y + 1, width, area.height.saturating_sub(2));
    app.layout.settings = inner;
    frame.render_widget(
        Paragraph::new(icons::label(icons::SETTINGS, "设置喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new("想改什么就点一点，改完记得让人家保存哦喵~")
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    let actions_y = inner.y + inner.height.saturating_sub(2);
    let mut y = inner.y + 2;

    frame.render_widget(
        Paragraph::new(format!(
            "{} 配置文件  {}喵~",
            icons::TERMINAL,
            format::truncate(
                &app.config_file_label(),
                inner.width.saturating_sub(14) as usize
            )
        ))
        .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y += 2;

    if y + 5 < actions_y {
        let theme_header = icons::label(icons::PALETTE, "主题喵~");
        let theme_offset = format::text_width(&theme_header) as u16 + 2;
        frame.render_widget(
            Paragraph::new(theme_header).style(
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(inner.x, y, inner.width, 1),
        );
        frame.render_widget(
            Paragraph::new("Catppuccin 四套配色，点一下就能换啦喵~")
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(
                inner.x + theme_offset,
                y,
                inner.width.saturating_sub(theme_offset),
                1,
            ),
        );
        y += 1;
        let gap = 1_u16;
        let card_width = (inner.width.saturating_sub(gap * 3)) / 4;
        for (index, flavor) in FLAVORS.iter().enumerate() {
            let rect = Rect::new(
                inner.x + index as u16 * (card_width + gap),
                y,
                card_width,
                4,
            );
            if rect.x + rect.width > inner.x + inner.width {
                break;
            }
            draw_theme_card(frame, app, theme, rect, flavor, hits);
        }
        y += 5;
    }

    if y + 6 < actions_y {
        let gap = 2_u16;
        let column_width = (inner.width.saturating_sub(gap)) / 2;
        let left_column = Rect::new(inner.x, y, column_width, actions_y - y);
        let right_column = Rect::new(
            inner.x + column_width + gap,
            y,
            inner.width.saturating_sub(column_width + gap),
            actions_y - y,
        );

        let preview_header = icons::label(icons::BOOK, "仓库预览喵~");
        let preview_offset = format::text_width(&preview_header) as u16 + 2;
        frame.render_widget(
            Paragraph::new(preview_header).style(
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(left_column.x, y, left_column.width, 1),
        );
        frame.render_widget(
            Paragraph::new("默认关闭，人家不会偷偷去取喵~")
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(
                left_column.x + preview_offset,
                y,
                left_column.width.saturating_sub(preview_offset),
                1,
            ),
        );
        draw_toggle_row(
            frame,
            app,
            theme,
            hits,
            Rect::new(left_column.x, y + 1, left_column.width, 1),
            1,
            "文件树预览喵~",
            app.config.show_file_tree,
            Action::ToggleFileTree,
        );
        draw_toggle_row(
            frame,
            app,
            theme,
            hits,
            Rect::new(left_column.x, y + 2, left_column.width, 1),
            2,
            "README 预览喵~",
            app.config.show_readme,
            Action::ToggleReadme,
        );
        frame.render_widget(
            Paragraph::new(icons::label(icons::DOWNLOAD, "下载喵~")).style(
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(left_column.x, y + 4, left_column.width, 1),
        );
        draw_download_dir_row(
            frame,
            app,
            theme,
            hits,
            Rect::new(left_column.x, y + 5, left_column.width, 1),
        );
        let open_dir = icons::label(icons::DOWNLOAD_DIR, "打开下载目录喵~");
        let open_width = format::text_width(&open_dir) as u16 + 4;
        if y + 6 < actions_y {
            draw_button(
                frame,
                app,
                theme,
                hits,
                Rect::new(left_column.x, y + 6, open_width, 1),
                &open_dir,
                Action::OpenDownloadDir,
                false,
            );
            frame.render_widget(
                Paragraph::new("同名文件会自动改名，不会覆盖旧的哦喵~")
                    .style(Style::default().fg(theme.color("overlay0"))),
                Rect::new(left_column.x, y + 7, left_column.width.saturating_sub(2), 1),
            );
        }

        let network_header = icons::label(icons::NETWORK, "网络喵~");
        let network_offset = format::text_width(&network_header) as u16 + 2;
        frame.render_widget(
            Paragraph::new(network_header).style(
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(right_column.x, y, right_column.width, 1),
        );
        frame.render_widget(
            Paragraph::new("公开接口先走代理，认证请求直连 GitHub 哦喵~")
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(
                right_column.x + network_offset,
                y,
                right_column.width.saturating_sub(network_offset),
                1,
            ),
        );
        draw_proxy_row(
            frame,
            app,
            theme,
            hits,
            Rect::new(right_column.x, y + 1, right_column.width, 1),
        );
        let cache_header = icons::label(icons::DATABASE, "缓存喵~");
        let cache_offset = format::text_width(&cache_header) as u16 + 2;
        frame.render_widget(
            Paragraph::new(cache_header).style(
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(right_column.x, y + 3, right_column.width, 1),
        );
        frame.render_widget(
            Paragraph::new(format!(
                "{} 个文件 · {}喵~",
                app.cache_stats.files,
                format::format_size(app.cache_stats.bytes)
            ))
            .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(
                right_column.x + cache_offset,
                y + 3,
                right_column.width.saturating_sub(cache_offset),
                1,
            ),
        );
        for (index, (label, kind)) in [
            ("仓库列表喵~", TtlKind::Repositories),
            ("详情与发行版喵~", TtlKind::Detail),
            ("搜索结果喵~", TtlKind::Search),
        ]
        .into_iter()
        .enumerate()
        {
            if y + 4 + index as u16 >= actions_y {
                break;
            }
            draw_ttl_row(
                frame,
                app,
                theme,
                hits,
                Rect::new(right_column.x, y + 4 + index as u16, right_column.width, 1),
                label,
                kind,
                5 + index,
            );
        }
    }

    let mut x = inner.x;
    let save_action = Action::SaveConfig;
    let save_hover = app.hover.as_ref() == Some(&save_action);
    let save_width = 18;
    fill(
        frame,
        Rect::new(x, actions_y, save_width, 1),
        if save_hover {
            theme.color("sapphire")
        } else {
            theme.color("blue")
        },
    );
    frame.render_widget(
        Paragraph::new(format!("{} 保存设置喵~", icons::SAVE))
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(theme.color("base"))
                    .add_modifier(Modifier::BOLD),
            ),
        Rect::new(x, actions_y, save_width, 1),
    );
    register(hits, Rect::new(x, actions_y, save_width, 1), save_action);
    x += save_width + 1;
    let clear_action = Action::ClearCache;
    let clear_hover = app.hover.as_ref() == Some(&clear_action);
    let clear_width = 16;
    fill(
        frame,
        Rect::new(x, actions_y, clear_width, 1),
        if clear_hover {
            theme.color("surface1")
        } else {
            theme.color("surface0")
        },
    );
    frame.render_widget(
        Paragraph::new(format!("{} 清掉缓存喵~", icons::TRASH))
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.color("red"))),
        Rect::new(x, actions_y, clear_width, 1),
    );
    register(hits, Rect::new(x, actions_y, clear_width, 1), clear_action);
    x += clear_width + 1;
    let manage_label = icons::label(icons::DATABASE, "缓存管理喵~");
    let manage_width = format::text_width(&manage_label) as u16 + 4;
    if x + manage_width <= inner.x + inner.width {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(x, actions_y, manage_width, 1),
            &manage_label,
            Action::OpenCacheManager,
            false,
        );
    }
    if app.config_dirty {
        frame.render_widget(
            Paragraph::new("有改动还没保存呢喵~")
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("peach"))),
            Rect::new(inner.x, actions_y, inner.width, 1),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_toggle_row(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    hits: &mut Vec<HitBox>,
    rect: Rect,
    row: usize,
    label: &str,
    enabled: bool,
    action: Action,
) {
    if app.settings_row == row {
        fill(frame, rect, theme.color("surface0"));
    }
    let icon = if enabled {
        icons::CHECK_SQUARE
    } else {
        icons::SQUARE
    };
    frame.render_widget(
        Paragraph::new(format!("{icon} {label}")).style(if enabled {
            Style::default()
                .fg(theme.color("green"))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.color("subtext1"))
        }),
        Rect::new(rect.x + 1, rect.y, rect.width.saturating_sub(12), 1),
    );
    frame.render_widget(
        Paragraph::new(if enabled {
            "已开启喵~"
        } else {
            "已关闭喵~"
        })
        .alignment(Alignment::Right)
        .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(rect.x, rect.y, rect.width.saturating_sub(1), 1),
    );
    register(hits, rect, action);
}

fn draw_download_dir_row(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    hits: &mut Vec<HitBox>,
    rect: Rect,
) {
    if app.settings_row == 3 {
        fill(frame, rect, theme.color("surface0"));
    }
    frame.render_widget(
        Paragraph::new("下载目录喵~").style(Style::default().fg(theme.color("subtext1"))),
        Rect::new(rect.x + 1, rect.y, 12, 1),
    );
    if app.editing_download_dir {
        let input_width = rect.width.saturating_sub(14);
        let (visible, cursor_column) = visible_input(&app.download_input, true, input_width);
        frame.render_widget(
            Paragraph::new(visible).style(Style::default().fg(theme.color("text"))),
            Rect::new(rect.x + 13, rect.y, input_width, 1),
        );
        let x = rect
            .x
            .saturating_add(13)
            .saturating_add(cursor_column.min(input_width.saturating_sub(1)));
        frame.set_cursor_position(Position::new(x, rect.y));
        return;
    }
    let path = format::truncate(
        &app.download_path_label(),
        rect.width.saturating_sub(22) as usize,
    );
    frame.render_widget(
        Paragraph::new(path).style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(rect.x + 13, rect.y, rect.width.saturating_sub(23), 1),
    );
    let edit_action = Action::StartDownloadDirEdit;
    let edit_width = 8;
    let edit_rect = Rect::new(
        rect.x + rect.width.saturating_sub(edit_width),
        rect.y,
        edit_width,
        1,
    );
    draw_button(
        frame,
        app,
        theme,
        hits,
        edit_rect,
        "编辑喵~",
        edit_action,
        false,
    );
    register(hits, rect, Action::SetSettingsRow(3));
}

fn draw_theme_card(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    area: Rect,
    flavor: &str,
    hits: &mut Vec<HitBox>,
) {
    let selected = app.theme.flavor == flavor;
    let action = Action::SetTheme(flavor.to_string());
    let hovered = app.hover.as_ref() == Some(&action);
    let preview = Theme::load(&app.paths, flavor);
    let bg = if selected {
        theme.color("surface0")
    } else {
        theme.color("mantle")
    };
    fill(frame, area, bg);
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if selected {
                theme.color("blue")
            } else if hovered {
                theme.color("surface2")
            } else {
                theme.color("surface1")
            })),
        area,
    );
    let label = Theme::label(flavor);
    frame.render_widget(
        Paragraph::new(label).style(
            Style::default()
                .fg(if selected {
                    theme.color("blue")
                } else {
                    theme.color("text")
                })
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), 1),
    );
    let colors = [
        "base", "surface1", "blue", "mauve", "green", "peach", "red", "text",
    ];
    let mut swatches = Vec::new();
    for name in colors {
        swatches.push(Span::styled("  ", Style::default().bg(preview.color(name))));
    }
    frame.render_widget(
        Paragraph::new(Line::from(swatches)),
        Rect::new(area.x + 2, area.y + 2, area.width.saturating_sub(4), 1),
    );
    register(hits, area, action);
}

fn draw_proxy_row(frame: &mut Frame, app: &App, theme: &Theme, hits: &mut Vec<HitBox>, rect: Rect) {
    if app.settings_row == 4 {
        fill(frame, rect, theme.color("surface0"));
    }
    frame.render_widget(
        Paragraph::new("API 代理喵~").style(Style::default().fg(theme.color("subtext1"))),
        Rect::new(rect.x + 1, rect.y, 12, 1),
    );
    if app.editing_proxy {
        let input_width = rect.width.saturating_sub(14);
        let (visible, cursor_column) = visible_input(&app.proxy_input, true, input_width);
        frame.render_widget(
            Paragraph::new(visible).style(Style::default().fg(theme.color("text"))),
            Rect::new(rect.x + 13, rect.y, input_width, 1),
        );
        let x = rect
            .x
            .saturating_add(13)
            .saturating_add(cursor_column.min(input_width.saturating_sub(1)));
        frame.set_cursor_position(Position::new(x, rect.y));
        return;
    }
    frame.render_widget(
        Paragraph::new(format::truncate(
            app.client.proxy(),
            rect.width.saturating_sub(23) as usize,
        ))
        .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(rect.x + 13, rect.y, rect.width.saturating_sub(23), 1),
    );
    let edit_rect = Rect::new(rect.x + rect.width.saturating_sub(9), rect.y, 8, 1);
    draw_button(
        frame,
        app,
        theme,
        hits,
        edit_rect,
        "编辑喵~",
        Action::StartProxyEdit,
        false,
    );
    register(hits, rect, Action::SetSettingsRow(4));
}

#[allow(clippy::too_many_arguments)]
fn draw_ttl_row(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    hits: &mut Vec<HitBox>,
    area: Rect,
    label: &str,
    kind: TtlKind,
    row_index: usize,
) {
    register(hits, area, Action::SetSettingsRow(row_index));
    if app.settings_row == row_index {
        fill(frame, area, theme.color("surface0"));
    }
    frame.render_widget(
        Paragraph::new(label).style(Style::default().fg(theme.color("subtext1"))),
        Rect::new(area.x + 1, area.y, 16, 1),
    );
    let value = match kind {
        TtlKind::Repositories => app.config.repos_ttl_secs,
        TtlKind::Detail => app.config.detail_ttl_secs,
        TtlKind::Search => app.config.search_ttl_secs,
    };
    frame.render_widget(
        Paragraph::new(format::ttl_label(value)).style(Style::default().fg(theme.color("text"))),
        Rect::new(area.x + 17, area.y, 12, 1),
    );
    let minus = Action::Ttl(kind, -1);
    let plus = Action::Ttl(kind, 1);
    let minus_rect = Rect::new(area.x + area.width.saturating_sub(17), area.y, 5, 1);
    let plus_rect = Rect::new(area.x + area.width.saturating_sub(9), area.y, 5, 1);
    for (rect, label, action) in [(minus_rect, "-", minus), (plus_rect, "+", plus)] {
        let hovered = app.hover.as_ref() == Some(&action);
        fill(
            frame,
            rect,
            if hovered {
                theme.color("surface1")
            } else {
                theme.color("mantle")
            },
        );
        frame.render_widget(
            Paragraph::new(label)
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.color("text"))),
            rect,
        );
        register(hits, rect, action);
    }
}

fn render_scrolled(
    frame: &mut Frame,
    area: Rect,
    lines: Vec<Line>,
    actions: Vec<(usize, Action)>,
    scroll: u16,
    theme: &Theme,
    hits: &mut Vec<HitBox>,
) {
    if area.height == 0 {
        return;
    }
    frame.render_widget(Paragraph::new(Text::from(lines)).scroll((scroll, 0)), area);
    let scroll = scroll as usize;
    for (line_index, action) in actions {
        if line_index < scroll || line_index >= scroll + area.height as usize {
            continue;
        }
        let y = area.y + (line_index - scroll) as u16;
        let rect = Rect::new(area.x, y, area.width, 1);
        let hovered = false;
        let _ = hovered;
        register(hits, rect, action);
    }
    let _ = theme;
}

fn plain_excerpt(body: &str) -> String {
    let mut lines = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("<!--")
            || trimmed.starts_with("[//]:")
            || trimmed.starts_with("**Full Changelog**")
        {
            continue;
        }
        let cleaned = trimmed
            .trim_start_matches('#')
            .trim_start_matches('-')
            .trim_start_matches('*')
            .trim();
        if !cleaned.is_empty() {
            lines.push(cleaned.replace('`', ""));
        }
        if lines.len() >= 4 {
            break;
        }
    }
    lines.join(" ")
}

#[allow(clippy::too_many_arguments)]
fn draw_button(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    hits: &mut Vec<HitBox>,
    rect: Rect,
    label: &str,
    action: Action,
    primary: bool,
) {
    let hovered = app.hover.as_ref() == Some(&action);
    let background = if primary {
        if hovered {
            theme.color("sapphire")
        } else {
            theme.color("blue")
        }
    } else if hovered {
        theme.color("surface1")
    } else {
        theme.color("surface0")
    };
    let foreground = if primary {
        theme.color("base")
    } else if hovered {
        theme.color("text")
    } else {
        theme.color("subtext1")
    };
    fill(frame, rect, background);
    frame.render_widget(
        Paragraph::new(label)
            .alignment(Alignment::Center)
            .style(Style::default().fg(foreground).add_modifier(Modifier::BOLD)),
        rect,
    );
    register(hits, rect, action);
}

fn draw_center_message(frame: &mut Frame, theme: &Theme, area: Rect, title: &str, detail: &str) {
    let mut lines = vec![
        Line::from(Span::styled(
            title,
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            detail,
            Style::default().fg(theme.color("overlay0")),
        )),
    ];
    if area.height < 6 {
        lines.truncate(1);
    }
    let height = lines.len() as u16;
    let rect = Rect::new(
        area.x + 2,
        area.y + area.height.saturating_sub(height) / 2,
        area.width.saturating_sub(4),
        height.min(area.height),
    );
    frame.render_widget(
        Paragraph::new(Text::from(lines)).alignment(Alignment::Center),
        rect,
    );
}

fn draw_repo_edit_overlay(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    let Some(edit) = app.repo_edit.clone() else {
        return;
    };
    let width = area.width.saturating_sub(12).min(72);
    let height = area.height.saturating_sub(8).min(18);
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    app.layout.overlay = rect;
    frame.render_widget(Clear, rect);
    fill(frame, rect, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.color("blue"))),
        rect,
    );
    let inner = Rect::new(
        rect.x + 2,
        rect.y + 1,
        rect.width.saturating_sub(4),
        rect.height.saturating_sub(2),
    );
    frame.render_widget(
        Paragraph::new(icons::label(icons::COGS, "改改自己的仓库喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width.saturating_sub(12), 1),
    );
    frame.render_widget(
        Paragraph::new(format::truncate(
            &format!("{} 只会改你自己的仓库喵~", edit.full_name),
            inner.width as usize,
        ))
        .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    let mut y = inner.y + 3;
    for (row, label) in ["简介", "主页", "主题", "可见性", "归档"]
        .iter()
        .enumerate()
    {
        if y >= inner.y + inner.height - 3 {
            break;
        }
        let active = edit.row == row;
        let editing = active && edit.editing;
        if active {
            fill(
                frame,
                Rect::new(inner.x, y, inner.width, 1),
                theme.color("surface0"),
            );
        }
        frame.render_widget(
            Paragraph::new(format!("{label}喵~")).style(if active {
                Style::default()
                    .fg(theme.color("blue"))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.color("subtext0"))
            }),
            Rect::new(inner.x + 1, y, 11, 1),
        );
        let value_x = inner.x + 12;
        let value_width = inner.width.saturating_sub(12);
        let (value, cursor) = match row {
            0 => visible_input(&edit.description, editing, value_width),
            1 => visible_input(&edit.homepage, editing, value_width),
            2 => visible_input(&edit.topics, editing, value_width),
            3 => (
                if edit.private {
                    "私有 · 只有自己能看到喵~".to_string()
                } else {
                    "公开 · 大家都看得到喵~".to_string()
                },
                0,
            ),
            _ => (
                if edit.archived {
                    "已归档 · 只读状态喵~".to_string()
                } else {
                    "正常 · 还能继续折腾喵~".to_string()
                },
                0,
            ),
        };
        let empty = match row {
            0 => edit.description.is_empty(),
            1 => edit.homepage.is_empty(),
            2 => edit.topics.is_empty(),
            _ => false,
        };
        let placeholder = match row {
            0 => "还没写简介喵~",
            1 => "没有主页喵~",
            2 => "还没有主题喵~",
            _ => "",
        };
        let shown = if empty {
            placeholder.to_string()
        } else {
            value
        };
        frame.render_widget(
            Paragraph::new(format::truncate(&shown, value_width as usize)).style(if empty {
                Style::default().fg(theme.color("overlay0"))
            } else {
                Style::default().fg(theme.color("text"))
            }),
            Rect::new(value_x, y, value_width, 1),
        );
        if editing {
            let cursor_x = if empty {
                value_x
            } else {
                value_x.saturating_add(cursor.min(value_width.saturating_sub(1)))
            };
            frame.set_cursor_position(Position::new(cursor_x, y));
        }
        register(
            hits,
            Rect::new(inner.x, y, inner.width, 1),
            Action::EditRepoRow(row),
        );
        y += 1;
    }

    if let Some(error) = edit.error.as_deref() {
        frame.render_widget(
            Paragraph::new(format::truncate(error, inner.width as usize))
                .style(Style::default().fg(theme.color("red"))),
            Rect::new(inner.x, y + 1, inner.width, 1),
        );
    }

    let actions_y = inner.y + inner.height.saturating_sub(2);
    let save_label = if edit.saving {
        format!("{} 保存中喵~", app.spinner())
    } else {
        icons::label(icons::SAVE, "保存喵~")
    };
    let save_width = format::text_width(&save_label) as u16 + 4;
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, actions_y, save_width, 1),
        &save_label,
        Action::SaveRepoEdit,
        true,
    );
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x + save_width + 2, actions_y, 12, 1),
        "取消喵~",
        Action::CloseOverlay,
        false,
    );
    if actions_y > inner.y + 2 {
        let hint = if edit.editing {
            "输入中：Enter 完成并下一项   Esc 结束编辑   Ctrl+S 保存喵~"
        } else {
            "j/k 选   Enter 编辑/切换   Ctrl+S 保存   Esc 关掉喵~"
        };
        frame.render_widget(
            Paragraph::new(hint)
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(inner.x, actions_y - 2, inner.width, 1),
        );
    }
}

fn draw_languages_overlay(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    let choices = app.language_choices();
    let width = area.width.saturating_sub(12).min(58);
    let height = area.height.saturating_sub(8).clamp(10, 24);
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    app.layout.overlay = rect;
    frame.render_widget(Clear, rect);
    fill(frame, rect, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.color("blue"))),
        rect,
    );
    let inner = Rect::new(
        rect.x + 2,
        rect.y + 1,
        rect.width.saturating_sub(4),
        rect.height.saturating_sub(2),
    );
    frame.render_widget(
        Paragraph::new(icons::label(icons::CODE, "挑一个语言喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width.saturating_sub(10), 1),
    );
    let close = Rect::new(inner.x + inner.width.saturating_sub(9), inner.y, 9, 1);
    draw_button(
        frame,
        app,
        theme,
        hits,
        close,
        "关掉喵~",
        Action::CloseOverlay,
        false,
    );
    frame.render_widget(
        Paragraph::new(format!("一共 {} 项，人家都给你列出来啦喵~", choices.len()))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    let list = Rect::new(
        inner.x,
        inner.y + 3,
        inner.width,
        inner.height.saturating_sub(4),
    );
    app.language_viewport = list.height as usize;
    let max_offset = choices.len().saturating_sub(app.language_viewport.max(1));
    app.language_offset = app.language_offset.min(max_offset);
    for (row, (value, label, count)) in choices
        .iter()
        .skip(app.language_offset)
        .take(app.language_viewport)
        .enumerate()
    {
        let index = app.language_offset + row;
        let rect = Rect::new(list.x, list.y + row as u16, list.width, 1);
        let active = value.as_deref() == app.language_filter.as_deref();
        let selected = index == app.language_selected;
        if active || selected {
            fill(
                frame,
                rect,
                if active {
                    theme.color("surface1")
                } else {
                    theme.color("surface0")
                },
            );
        }
        if active {
            fill(frame, Rect::new(rect.x, rect.y, 1, 1), theme.color("blue"));
        }
        frame.render_widget(
            Paragraph::new(format!(
                "{} {}",
                if active { icons::CHECK } else { icons::SQUARE },
                label
            ))
            .style(if active {
                Style::default()
                    .fg(theme.color("blue"))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.color("subtext1"))
            }),
            Rect::new(rect.x + 1, rect.y, rect.width.saturating_sub(10), 1),
        );
        frame.render_widget(
            Paragraph::new(count.to_string())
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(rect.x, rect.y, rect.width.saturating_sub(2), 1),
        );
        register(hits, rect, Action::Language(value.clone()));
    }
    frame.render_widget(
        Paragraph::new("j/k 挑选   Enter 决定   Esc 关掉喵~")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(
            inner.x,
            inner.y + inner.height.saturating_sub(1),
            inner.width,
            1,
        ),
    );
}

fn draw_help(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect, hits: &mut Vec<HitBox>) {
    let width = area.width.saturating_sub(10).min(76);
    let height = area.height.saturating_sub(6).min(22);
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, rect);
    fill(frame, rect, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.color("blue"))),
        rect,
    );
    let inner = Rect::new(
        rect.x + 2,
        rect.y + 1,
        rect.width.saturating_sub(4),
        rect.height.saturating_sub(2),
    );
    let lines = [
        ("换页喵~", "1 仓库  2 收藏  3 搜索  4 账号  Tab 切页喵~"),
        (
            "挑挑喵~",
            "j / k 上下挑  [ ] 翻页  g / G 首尾页  PgUp/PgDn 翻页喵~",
        ),
        ("看仓库喵~", "Enter 进仓库页  o 去浏览器  y 复制克隆地址喵~"),
        (
            "改自己喵~",
            "自己的仓库按 e 改简介、主页、主题、可见性和归档喵~",
        ),
        ("找找喵~", "/ 去找仓库  Enter 开找  Esc 离开输入喵~"),
        ("账号喵~", "4 看账号信息，还能查 GitHub API 额度喵~"),
        (
            "工作流喵~",
            "仓库页按 a 看 Actions，R/x 跑工作流，Tab 填分支和 inputs 喵~",
        ),
        ("缓存喵~", "按 c 分类清理缓存，o 打开缓存目录喵~"),
        ("设置喵~", "退出后敲 gxt settings，用专门的设置 TUI 调喵~"),
        ("鼠标喵~", "点一下挑，点两下进仓库页，滚轮随便滚喵~"),
        ("回去喵~", "Esc 回列表  ? 关掉说明  q 退出人家喵~"),
    ];
    let mut y = inner.y;
    frame.render_widget(
        Paragraph::new(icons::label(icons::INFO, "人家能帮你做什么喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y += 2;
    for (label, text) in lines {
        if y >= inner.y + inner.height {
            break;
        }
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("{label:<6}"),
                    Style::default().fg(theme.color("blue")),
                ),
                Span::styled(text, Style::default().fg(theme.color("subtext0"))),
            ])),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y += 1;
    }
    let close = Rect::new(inner.x, inner.y + inner.height.saturating_sub(1), 10, 1);
    draw_button(
        frame,
        app,
        theme,
        hits,
        close,
        "关掉喵~",
        Action::CloseOverlay,
        false,
    );
    app.layout.overlay = rect;
}

fn draw_too_small(frame: &mut Frame, area: Rect, theme: &Theme) {
    fill(frame, area, theme.color("base"));
    let lines = vec![
        Line::from(Span::styled(
            "终端再大一点嘛喵~",
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("至少要 {MIN_WIDTH} x {MIN_HEIGHT}，现在有点挤喵~"),
            Style::default().fg(theme.color("overlay0")),
        )),
        Line::from(Span::styled(
            "按 Ctrl+C 可以退出喵~",
            Style::default().fg(theme.color("overlay0")),
        )),
    ];
    let height = lines.len() as u16;
    let rect = Rect::new(
        area.x + 2,
        area.y + area.height.saturating_sub(height) / 2,
        area.width.saturating_sub(4),
        height.min(area.height),
    );
    frame.render_widget(
        Paragraph::new(Text::from(lines)).alignment(Alignment::Center),
        rect,
    );
}

fn register(hits: &mut Vec<HitBox>, rect: Rect, action: Action) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }
    hits.push(HitBox { rect, action });
}

fn fill(frame: &mut Frame, rect: Rect, color: Color) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }
    frame.render_widget(Block::new().style(Style::default().bg(color)), rect);
}

fn language_color(theme: &Theme, language: &str) -> Color {
    const NAMES: [&str; 10] = [
        "blue", "mauve", "green", "peach", "teal", "yellow", "sapphire", "lavender", "pink", "sky",
    ];
    if language == "未标注喵~" {
        return theme.color("overlay0");
    }
    let hash = language.bytes().fold(0usize, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(byte as usize)
    });
    theme.color(NAMES[hash % NAMES.len()])
}

fn proxy_label(app: &App) -> String {
    app.client
        .proxy()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_string()
}

fn visible_input(input: &TextInput, focused: bool, max_width: u16) -> (String, u16) {
    if focused {
        input.visible_window(max_width as usize)
    } else {
        (format::truncate(&input.value, max_width.max(1) as usize), 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_nested_markdown_images() {
        let line = "[![GitHub last commit](https://img.example/a)](https://github.com/x)";
        assert_eq!(strip_markdown_inline(line), "");
        assert!(strip_markdown_images("![badge](https://img) text").contains(" text"));
        assert_eq!(
            strip_markdown_inline("**bold** and `code`"),
            "bold and code"
        );
    }
}
