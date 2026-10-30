# AGENT.md — gitx-tui 续作交接

> 给下一个 Agent：先读本文件，再按「待办」顺序做。当前版本 **0.1.1**，基线提交 `c1e9220`（已 push 到 `origin/main`）；gxt CLI / 本地仓库 TUI / 设置 TUI 可能还在未提交改动里，先 `git status` / `git diff` 看清楚。

## 项目速览

- Rust 2024 + ratatui 0.30 + crossterm 0.29 + ureq 3.5（已开 `json` feature）。
- 单 binary 多入口：`src/main.rs` + `src/cli.rs`；GitHub 页面状态在 `src/app.rs`，渲染在 `src/ui.rs`；本地仓库 TUI 在 `src/local.rs` / `src/local_app.rs` / `src/local_ui.rs`。
- 认证：`gh auth token --hostname github.com`，回退 `GITHUB_TOKEN` / `GH_TOKEN`。
- 公开 API 走 `proxy`（默认 `https://v4.gh-proxy.org`），认证请求直连 `api.github.com`。
- 缓存：`~/.cache/gitx-tui/*.json`，配置：`~/.config/gitx-tui/config.toml`。
- 文案风格：所有用户可见中文尽量以 `喵~` 结尾；不要用 emoji，图标在 `src/icons.rs`。
- 二进制名 `gxt`（`Cargo.toml` 的 `[[bin]]`）；`gxt` 开 GitHub 工作台，`gxt <路径>` 开本地仓库 TUI，`gxt settings` 开独立设置 TUI。
- 页面：`1` 仓库 / `2` 收藏 / `3` 搜索 / `4` 账号额度；设置按钮已从主导航移除，只从 `gxt settings` 进入；仓库详情是 `Page::Repo`，`Esc` 返回。
- 列表分页：`[` `]` 翻页，`g` `G` 首尾页，`PgUp` `PgDn`，顶部有上一页/下一页按钮；分页逻辑在 `App::page_count_for` / `current_page_for` / `set_list_page`，渲染 `draw_list_pagination`。

## 已完成（不要重复做）

- 全界面文案喵化（README / `--help` 同步）。
- 仓库、收藏、搜索列表分页（本地按窗口高度分页；仓库/收藏 API 会先自动取完全部分页）。
- 账号页：资料 + `/rate_limit` 的 core/search/graphql 额度与恢复时间，缓存 key `rate_limit`。
- 自己的仓库编辑：`e` / 「编辑仓库」按钮打开 `Overlay::EditRepo`，可改 description、homepage、topics、private、archived，`Ctrl+S` 走 `PATCH /repos/{owner}/{repo}`，成功后更新列表/详情/缓存。
- 工作流页：仓库页按 `a` 进入 `Page::Actions`，左 run / 右 jobs+steps，`Enter`/`o` 去浏览器，缓存 key `actions_*`，详见「待办 4」。
- 手动运行工作流：Actions 页按 `R`/`x` 或点「运行工作流」，选中工作流 + 分支/标签 + 可选 inputs JSON 后回车走 `POST /repos/{owner}/{repo}/actions/workflows/{id}/dispatches`。
- 缓存管理：设置 TUI「缓存管理」或主界面 `c` 打开 `Overlay::Cache`，按分类清理，`o` 打开缓存目录。
- gxt CLI：`gxt` / `gxt <路径>` / `gxt settings` / `--theme` / `--help` / `--version`，二进制名改为 `gxt`。
- 独立设置 TUI：`App::settings` 只显示设置页，跳过 GitHub 网络和 token 探测；主界面导航不再有「设置」按钮。
- 本地仓库 TUI：`gxt <路径>` 显示分支/改动/提交，支持暂存、取消暂存、提交，文件在 `src/local*`。
- 测试 40 项通过，`cargo clippy --all-targets -- -D warnings` 无警告。

## 待办 1：修编辑页 Enter 没用（✅ 已修）

已改：空字段也会显示光标；Enter 完成当前字段并自动跳到下一行；兼容 `\r` / `\n`；提示文案会随编辑态切换。已通过 `cargo test` / clippy，建议手动再摸一遍。

原始分析：`src/app.rs` 的 `handle_repo_edit_key` 在未编辑状态下 `KeyCode::Enter` 会把 `edit.editing = true`；编辑状态下 Enter 只是 `editing = false`。用户反馈「Enter 没用」，最可能是：

1. 空字段不显示光标（`src/ui.rs` 里 `if editing && !empty` 才 `set_cursor_position`），按 Enter 后看起来没反应。
2. Enter 只是切换编辑态，没有明显的「完成/下一项」反馈。

建议改法（保持简单）：

- `src/ui.rs`：`if editing` 就去设置光标，空字段把光标放在 `value_x`，不要依赖 `!empty`。
- `src/app.rs` 的 `handle_repo_edit_key`：
  - 编辑态 Enter：先把当前字段提交（`editing = false`），然后自动移到下一行（0→1→2→3→4，3/4 不进入文本编辑）；最后一行再按 Enter 可以触发保存或停在归档。
  - 同时接受 `KeyCode::Char('\r') | KeyCode::Char('\n')`，防止某些终端把 Enter 报成字符。
  - 未编辑态 Enter：文本行进入编辑；可见性/归档行 toggle；保持现有 `Ctrl+S` 保存。
- `src/ui.rs` 的编辑弹窗底部提示根据 `edit.editing` 切换：
  - 编辑中：`Enter 完成并下一项   Esc 结束编辑   Ctrl+S 保存`
  - 非编辑：`j/k 选   Enter 编辑/切换   Ctrl+S 保存   Esc 关掉`
- 手动验证：`cargo run` → `e` → Enter 简介 → 输入文字 → Enter → 应看到光标和下一行选中；`Ctrl+S` 只在测试账号/测试仓库上点，避免误改真实仓库。

## 待办 2：install 脚本（✅ 已完成，本地构建）

`install.sh` 不做 release 下载，而是在仓库根目录执行：

```bash
./install.sh                 # cargo build --release --locked
./install.sh --dir /some/bin # 装到指定目录
./install.sh --debug         # 构建 debug 版
```

行为：检查 `cargo` 和 `Cargo.toml` → `cargo build --release --locked` → 把 `target/release/gxt` 装到 `${INSTALL_DIR:-$HOME/.local/bin}` 并命名为 `gxt`（`install -m 755`，失败时 `cp + chmod` 兜底）→ 提示 PATH。release tarball 里不要放 `install.sh`（没有源码时它跑不起来）。

## 待办 3：GitHub Actions 构建并发布 Release（✅ 已完成）

`.github/workflows/release.yml` 已创建并通过 YAML 解析校验；tag `v*` 触发，5 个 target，打包 + SHA256SUMS + Release。下一步只需 push 后打 tag 验证。

原始要求：

- 触发：`push: tags: ['v*']`，`workflow_dispatch` 可选。
- 矩阵：
  - `x86_64-unknown-linux-gnu`（ubuntu-latest）
  - `aarch64-unknown-linux-gnu`（ubuntu-latest + `gcc-aarch64-linux-gnu`，或 `cross`）
  - `x86_64-apple-darwin`（macos-13）
  - `aarch64-apple-darwin`（macos-14）
  - `x86_64-pc-windows-msvc`（windows-latest，zip）
- 步骤：checkout、`dtolnay/rust-toolchain@stable` + target、`Swatinem/rust-cache`、`cargo build --release --target`、打包：
  - unix：`gxt-${VERSION}-${TARGET}.tar.gz`，包内目录 `gxt-${VERSION}-${TARGET}/gxt` + README + LICENSE
  - windows：`gxt-${VERSION}-${TARGET}.zip`
- 生成 `SHA256SUMS`，用 `softprops/action-gh-release@v2` 上传所有资产，`generate_release_notes: true`。
- 版本从 tag 去掉 `v`；检查 `Cargo.toml` 版本与 tag 一致，不一致直接失败。
- 如需要 Linux musl 静态包可加，但先不要扩大矩阵。
- workflow 里不要引入需要付费/私有 token 的 action。

## 待办 4：GitHub Actions 功能（✅ 已完成）

已实现：`Page::Actions`、`WorkflowRun/Job/Step` 模型、`ApiClient::action_runs/action_jobs`、左右双栏页面、缓存、`a` 入口、文档与测试。后续补了 `Workflow` 列表与手动运行弹窗（`R`/`x`，`workflow_dispatch`）。`Esc` 从仓库页进入时回仓库页，从列表页进入时回原列表。原始设计留档：

目标：新增一个专门页面查看当前仓库的 GitHub Actions 运行记录。GitHub 没有「账号级全部 runs」的稳定 API，建议做成 **仓库维度** 的 Actions 页：

- 入口：仓库详情页按 `a` 或点「工作流」按钮进入 `Page::Actions`；`Esc` 返回仓库页；保留 `o` 在浏览器打开当前 run。
- 数据模型（`src/model.rs`）：
  - `WorkflowRun { id, name, display_title, event, status, conclusion, head_branch, run_number, html_url, created_at, updated_at, run_started_at, actor: Owner, workflow_id }`
  - `WorkflowRunsResponse { total_count, workflow_runs: Vec<WorkflowRun> }`
  - `WorkflowJob { id, name, status, conclusion, started_at, completed_at, html_url, steps: Vec<WorkflowStep> }`
  - `WorkflowStep { name, status, conclusion, number }`
  - `WorkflowJobsResponse { total_count, jobs: Vec<WorkflowJob> }`
- API（`src/api.rs`）：
  - `action_runs(full_name, private) -> Result<Vec<WorkflowRun>, String>`：`GET /repos/{owner}/{repo}/actions/runs?per_page=30`，复用 `get_pages` 或只取第一页（页面内再分页）；认证请求直连。
  - `action_jobs(full_name, run_id, private) -> Result<Vec<WorkflowJob>, String>`：`GET /repos/{owner}/{repo}/actions/runs/{run_id}/jobs?per_page=30`。
  - 公开仓库也可走 `get_public_with_fallback`，私有必须带 token。
- App 状态（`src/app.rs`）：
  - `Page::Actions`（不要进主导航，或放第 6 个；先不要动 1-5 的既有键位）。
  - `actions_repo: Option<String>`、`action_runs: Vec<WorkflowRun>`、`action_runs_loading`、`action_runs_fetched_at`、`action_selected`、`action_jobs: Vec<WorkflowJob>`、`action_jobs_loading`、`action_jobs_for`、`action_detail_scroll`。
  - `Msg::ActionRuns { name, result }`、`Msg::ActionJobs { run_id, result }`。
  - 缓存 key：`actions_{key_part(full_name)}`、`actions_jobs_{run_id}`，TTL 用 `detail_ttl_secs`。
  - 刷新 `r`、选择移动 `j/k`、分页复用 `[` `]` 或 `PgUp/PgDn`、Enter 打开 run 的 `html_url`。
- UI（`src/ui.rs`）：
  - 标题「工作流喵~」，副标题「这里是当前仓库的 Actions 运行记录喵~」。
  - 左：run 列表（状态图标：成功/失败/进行中/排队/跳过；显示 `#run_number`、名称、分支、event、相对时间）。
  - 右：选中 run 的 jobs + steps（状态、耗时、结论），空态「还没有运行记录喵~」。
  - 底部提示：`j/k 挑  Enter 去浏览器  r 刷新  Esc 返回仓库喵~`。
  - 不要 emoji；用 `icons::COGS` / `CHECK` / `WARNING` / `CLOCK` 等。
- 其他同步：`README.md` 页面表/键盘表/已知限制、`src/main.rs` help、`hints_for(Page::Actions)`、`Page::title`。
- 测试：至少给 `WorkflowRun` 的 JSON 解析加一个 serde 测试；如果抽了纯函数（状态排序/耗时格式化）再加单测。

## 关键文件与位置

| 文件 | 用途 |
| --- | --- |
| `src/api.rs` | `ApiClient`、`get_pages`、`get_json`、`get_public_with_fallback`、`status_message`、PATCH 示例 |
| `src/app.rs` | `Page` / `Overlay` / `Action` / `Msg` / `App` 状态、`handle_key`、`handle_message`、缓存 |
| `src/ui.rs` | `draw()` 分发、`draw_repo_page`、`draw_list_pagination`、`draw_repo_edit_overlay`、`draw_account_page` |
| `src/model.rs` | 所有 serde 数据结构和测试 |
| `src/config.rs` | 配置字段、TTL、路径 |
| `src/cache.rs` | `Cache::load/save`、`Cached`、`now_epoch` |
| `src/cli.rs` | `gxt` 参数解析、帮助、`settings` / 本地路径分派 |
| `src/local.rs` | 本地 git 读取、porcelain 解析、暂存/取消暂存/提交 |
| `src/local_app.rs` | 本地仓库 TUI 状态、按键、事件循环 |
| `src/local_ui.rs` | 本地仓库 TUI 三栏渲染和提交弹窗 |
| `README.md` | 功能、页面、键盘、安装、已知限制 |

## 常用命令

```bash
cargo fmt
cargo test
cargo clippy --all-targets -- -D warnings
cargo run -- --help
cargo run -- .          # 本地仓库 TUI
cargo run -- settings   # 设置 TUI
cargo build --release   # 产物是 target/release/gxt
```

## 约束

- 不要使用 subagent（用户明确禁止过）。
- 不要全量读 `src/app.rs` / `src/ui.rs`，用 `grep` 定位后分段读。
- 任何 VCS 操作前先 `git status`；仓库不是 jj 仓库（无 `.jj/`）。
- 保持现有文案风格：中文 + `喵~`，不用 emoji。
- 改动后必须跑 `cargo fmt && cargo test && cargo clippy --all-targets -- -D warnings`。
- 发布/推送前确认版本号：`Cargo.toml` / `Cargo.lock` 的包版本与 tag 一致；二进制名是 `gxt`，包名仍是 `gitx-tui`。

## 已知小问题（有空再修）

- 设置页在 72 列附近 TTL 行会挤，必要时改成单列或隐藏 `+/-`。
- 编辑弹窗高度固定偏大，可压缩到 14 行左右。
- 账号页额度刷新按钮与标题同一行，窄宽度可能重叠。
- 私有仓库的 Actions 接口需要 token，未登录时会提示。
- 本地仓库 TUI 只做部分 lazygit：没有 diff 浏览、冲突处理、push/pull、鼠标操作。

## 验收清单

- [ ] 编辑页 Enter 有可见光标，能完成编辑并移到下一项。
- [ ] `install.sh` 在 Linux x86_64 上 dry-run 逻辑正确（无 release 时给出清晰错误）。
- [ ] `release.yml` YAML 通过 `actionlint` 或至少 `python -c 'import yaml...'` 校验。
- [x] Actions 页面能列出 run、切换选择、加载 jobs、Esc 返回。
- [x] `cargo test` 全过，clippy 无警告。
- [x] README / `--help` / 帮助弹窗同步新按键与安装方式。
- [x] `gxt settings` 打开独立设置 TUI，主界面不再有设置导航。
- [x] `gxt <路径>` 打开本地仓库 TUI，暂存/取消暂存/提交可用。
- [x] 构建产物名为 `target/release/gxt`。
