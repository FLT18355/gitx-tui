# gitx-tui（命令 `gxt`）

终端里的 GitHub 工作台和本地 git TUI，使用 Rust、ratatui 与 crossterm 写成喵~

GitHub 侧覆盖仓库、收藏、搜索、账号额度、仓库详情和 Actions；本地侧用 lazygit 风格查看改动、diff、分支、提交和标签，并支持暂存、提交、推送、拉取、取回和打标签。界面使用 Catppuccin 配色与 Nerd Font 图标，没有 emoji 喵~

## 功能

- GitHub 仓库：账号仓库、收藏、搜索、语言筛选、排序、分页和仓库详情。
- 仓库详情：文件树、发行版下载、README、议题/合并请求入口和自己的仓库编辑。
- Actions：运行记录、jobs、steps、状态与耗时，以及手动触发工作流和 inputs。
- 账号页：资料、仓库统计、GitHub API core/search/GraphQL 额度与恢复时间。
- 本地仓库 TUI：
  - 分支、标签、改动、提交四个列表，支持分页和选择；
  - 选中文件或提交时显示差异；
  - 暂存、取消暂存、全部暂存、全部取消暂存；
  - 提交、创建标签、推送当前分支、推送标签、fetch、pull；
  - 切换分支；
  - 中文输入、横向滚动和显示宽度正确的光标。
- 全局设置 TUI：主题、预览开关、下载目录、代理、缓存时间和缓存管理。
- 主题：Latte、Frappe、Macchiato、Mocha 四套 Catppuccin 配色。
- 缓存：仓库、收藏、搜索、详情、Actions 与工作流列表都会落盘，按 TTL 刷新。
- 代理：公开 API 可走加速代理，认证请求直连 GitHub。

## 界面

- GitHub 工作台：仓库、收藏、搜索、账号四个页面，以及仓库详情和 Actions。
- 仓库详情：文件树、发行版、README 和 Actions 入口。
- 本地仓库 TUI：左侧分支和标签，中间改动，右侧差异，底部提交记录。
- 设置 TUI：所有全局设置集中在一页。

## 文档

- CLI 用法：运行 `gxt -h`，或看 [docs/CLI.md](docs/CLI.md)。
- 键位：看 [docs/KEYBINDINGS.md](docs/KEYBINDINGS.md)。
- 配置、网络与缓存：看 [docs/CONFIG.md](docs/CONFIG.md)。
- 构建、测试与架构：看 [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)。

## 环境

- Rust 2024 / 1.88 或更新版本（从源码构建时）。
- GitHub CLI `gh`，用于读取登录凭据。
- 支持鼠标与真彩色的终端。
- 推荐 Nerd Font；没有时图标可能显示成方块。

## 许可证

MIT