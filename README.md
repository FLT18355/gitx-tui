# gitx-tui

终端里的 GitHub 工作台，使用 ratatui 和 crossterm 写成喵~仓库、收藏、搜索、账号、设置和仓库详情分别占一页，配上 Catppuccin 配色、卡片列表、Nerd Font 图标和鼠标操作，尽量做得像桌面应用喵~

界面里没有 emoji，图标都是 Nerd Font 字形喵~终端字体建议使用 Nerd Font，不然图标会显示成方块喵~

## 功能

- 人家的仓库：读取账号下的公开与私有仓库，支持全部语言筛选、排序、详情和仓库页喵~
- 列表分页：仓库、收藏、搜索列表都会按当前窗口大小分页，`[` `]`、`g` `G`、`PgUp` `PgDn` 或底部按钮都能翻页喵~
- 星星收藏：读取完整的 Starred 列表，超过 100 个会自动把接口翻完，界面上再分页展示喵~
- 找仓库：按关键词搜索 GitHub 仓库，支持最佳匹配、星标优先和最近更新排序，一次最多拿 100 个结果再分页喵~
- 账号页：看自己的 GitHub 小档案，还能查核心接口、搜索接口和 GraphQL 的额度与恢复时间喵~额度来自 `/rate_limit`，查它本身不消耗额度喵~
- 仓库页：
  - 左边是文件树预览，点文件或目录会去 GitHub 打开喵~
  - 右边是发行版和资产，点资产就会下载到本地下载目录喵~
  - 最下面是 README 预览，简单的 Markdown 会整理成可读文本喵~
  - 文件树和 README 默认关闭，需要到设置页手动打开喵~
- 本地下载：
  - 发行版资产和源码包都会下载到配置文件里的下载目录喵~
  - 默认目录是 `~/Downloads/gitx-tui`喵~
  - 同名文件会自动加序号，不会覆盖已经下载好的东西喵~
- 收藏操作：在仓库页可以点赞或取消收藏，结果会同步到收藏页和缓存喵~
- 自己的仓库：预览到 owner 是自己的仓库时，按 `e` 或点「编辑仓库」就能改简介、主页、主题、可见性和归档状态喵~保存会真实写回 GitHub喵~
- 常用入口：主页、克隆地址、源码下载、议题、合并请求和工作流喵~
- 工作流页：在仓库页按 `a` 或点「工作流」看当前仓库的 Actions 运行记录，左边挑 run，右边看 jobs 和 steps，`Enter` / `o` 去浏览器看日志喵~
- 设置页：主题、预览开关、下载目录、代理地址和缓存时间都在这里改，并写入配置文件喵~
- 缓存：仓库列表、收藏、搜索、仓库详情、文件树、README 和工作流都存在本地，按有效期自动刷新喵~设置页或 `c` 可以分类清理喵~
- 鼠标：单击选择、双击打开仓库页、滚轮滚动、点击标签、按钮、文件树、发行版资产和设置项喵~
- 主题：Latte、Frappe、Macchiato、Mocha 四套 Catppuccin 主题，每套一个 TOML喵~

## 环境要求

- Rust 1.88 或更新版本
- GitHub CLI `gh`
- 已执行 `gh auth login`
- 终端支持鼠标与真彩色
- Nerd Font 字体，用于显示图标

程序通过 `gh auth token --hostname github.com` 读取登录凭据，也支持 `GITHUB_TOKEN` 或 `GH_TOKEN` 环境变量喵~没有登录时仍然可以搜索公开仓库和查看公开仓库详情喵~

## 构建与运行

```bash
cargo build --release
./target/release/gitx-tui
```

也可以临时指定主题：

```bash
./target/release/gitx-tui --theme latte
```

在仓库根目录跑本地的安装脚本，会 `cargo build --release` 后装到 `~/.local/bin` 喵~

```bash
./install.sh
./install.sh --dir /some/bin   # 换安装目录
./install.sh --debug           # 构建 debug 版，编译快一点
```

也可以交给 Cargo：

```bash
cargo install --path .
```

发布 tag `v*` 时，GitHub Actions 会自动构建 Linux/macOS/Windows 包并创建 Release喵~

## 页面与操作

| 页面 | 内容 |
| --- | --- |
| 仓库 | 账号仓库列表、全部语言筛选、排序、详情和发行版 |
| 收藏 | 完整的 Starred 列表、统计、排序和详情 |
| 搜索 | 搜索框、排序、历史记录、结果列表和详情 |
| 账号 | 账号资料、仓库统计、GitHub API 核心/搜索/GraphQL 额度和恢复时间 |
| 设置 | 配置文件路径、主题、预览开关、下载目录、代理、缓存管理 |
| 仓库详情 | 统计、文件树、右侧发行版下载和底部 README；自己的仓库可以编辑喵~ |
| 工作流 | 当前仓库的 Actions 运行记录、任务与步骤状态，按 `Enter` 去浏览器喵~ |

### 键盘

| 按键 | 作用 |
| --- | --- |
| `1` `2` `3` `4` `5` | 切换仓库、收藏、搜索、账号、设置 |
| `Tab` | 在仓库页切换文件树、发行版、README 面板；其他页面切页 |
| `j` `k` | 上下移动或滚动当前面板 |
| `[` `]` | 上一页 / 下一页 |
| `g` `G` | 第一页 / 最后一页（仓库页里是面板顶部 / 底部） |
| `PgUp` `PgDn` | 列表翻页；仓库页里滚动面板 |
| `Enter` | 打开仓库页；设置页里执行当前项；账号页打开主页；工作流页打开 run；确认语言 |
| `Esc` | 离开输入、关闭弹窗或返回列表 |
| `/` | 聚焦搜索 |
| `a` | 打开当前仓库的 Actions 工作流页 |
| `c` | 打开缓存管理，按分类清理 |
| `e` | 编辑自己的仓库（简介、主页、主题、可见性、归档） |
| `f` | 收藏或取消收藏当前仓库 |
| `l` | 在仓库页打开完整语言筛选 |
| `o` | 用浏览器打开当前仓库 |
| `y` | 复制当前仓库的克隆地址 |
| `r` | 刷新当前页、账号额度或仓库预览 |
| `h` `l` | 在设置页调整当前项 |
| `Ctrl+S` | 保存设置；编辑仓库时保存改动 |
| `?` | 打开操作说明 |
| `q` | 退出 |

在搜索框里输入时，字母会正常进入输入框喵~搜索框里按 `Enter` 开始搜索，按 `Esc` 离开输入框喵~

### 鼠标

- 点击顶部标签切换页面喵~
- 点击仓库卡片选中，双击打开仓库页喵~
- 点击排序标签、语言筛选、主题卡片和按钮喵~
- 点击列表底部的上一页 / 下一页按钮翻页喵~
- 点击自己的仓库页里的「编辑仓库」可以打开编辑弹窗喵~
- 点击仓库页的「工作流」进入 Actions 页，点击运行记录切换 jobs，点击任务行去浏览器喵~
- 点击缓存管理里的分类可以选中，再点「清掉选中」清理喵~
- 点击文件树里的文件或目录会去 GitHub 打开喵~
- 点击发行版资产会开始本地下载喵~
- 滚轮滚动列表、文件树、发行版和 README 区域喵~

## 配置文件

项目根目录的 `config.toml` 是随程序发布的默认配置喵~第一次运行时，程序会把它写到：

```text
~/.config/gitx-tui/config.toml
```

设置页里的改动会写入这个文件喵~也可以直接编辑，重启后生效喵~

```toml
theme = "mocha"
proxy = "https://v4.gh-proxy.org"
repos_ttl_secs = 300
detail_ttl_secs = 900
search_ttl_secs = 600
search_history = []
show_file_tree = false
show_readme = false
download_dir = "~/Downloads/gitx-tui"
```

| 字段 | 说明 |
| --- | --- |
| `theme` | `latte`、`frappe`、`macchiato`、`mocha` |
| `proxy` | 公开 API 与发行版下载的加速地址 |
| `repos_ttl_secs` | 仓库和收藏列表的缓存时间，单位秒 |
| `detail_ttl_secs` | 仓库详情、文件树、README、发行版和 Actions 的缓存时间，单位秒 |
| `search_ttl_secs` | 搜索结果的缓存时间，单位秒 |
| `search_history` | 最近搜索，最多保留 8 条 |
| `show_file_tree` | 是否在仓库页加载文件树，默认关闭 |
| `show_readme` | 是否在仓库页底部加载 README，默认关闭 |
| `download_dir` | 本地下载目录，支持 `~/` 开头 |

`show_file_tree` 和 `show_readme` 打开之后，程序才会去请求对应的 GitHub API喵~关着的时候不会偷偷拉取喵~

## 主题

主题文件放在 `themes/` 下：

```text
themes/latte.toml
themes/frappe.toml
themes/macchiato.toml
themes/mocha.toml
```

四套都是 Catppuccin 官方色板，包含 base、mantle、crust、surface、overlay、text 以及强调色喵~程序优先读取当前目录的 `themes/`，其次读取 `~/.config/gitx-tui/themes/`，找不到时使用编译进二进制的同一套主题喵~

## 网络与缓存

公开 API（仓库详情、文件树、README、发行版、搜索）会先请求：

```text
https://v4.gh-proxy.org/https://api.github.com/...
```

发行版资产和源码包的下载也会先走：

```text
https://v4.gh-proxy.org/https://github.com/...
```

认证数据（`/user`、`/user/repos`、`/user/starred`、`/rate_limit`）、私有仓库详情、私有仓库的 Actions 接口、Star 操作和仓库编辑（`PATCH /repos/{owner}/{repo}`）直接访问 GitHub API喵~这样处理有两个原因：

1. `gh` 的登录凭据不会发送给第三方代理喵~
2. 公共代理不会转发 `Authorization`，也不适合承载私有仓库数据喵~

如果公开代理暂时不可用或额度用尽，程序会回退到直连公开 API喵~发行版下载也会在代理失败后尝试直连喵~

缓存目录：

```text
~/.cache/gitx-tui
```

数据按更新时间保存为 JSON喵~设置页可以清理缓存，也可以在配置里调整各缓存的有效期喵~

## 目录结构

```text
src/api.rs      GitHub API、gh 凭据、代理、直连回退和本地下载
src/app.rs      页面状态、分页、账号额度、仓库编辑、工作流、缓存管理和异步消息
src/cache.rs    JSON 缓存
src/config.rs   配置加载与保存
src/icons.rs    Nerd Font 图标
src/model.rs    GitHub 数据结构
src/theme.rs    Catppuccin 主题加载
src/ui.rs       全部页面渲染、账号页、编辑弹窗、文件树、README 和命中区域
themes/         四套主题 TOML
config.toml     随项目发布的默认配置
```

## 测试

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

## 已知限制

- 仓库列表和收藏会把接口分页全部取完后再按窗口分页展示，仓库特别多时第一次同步会慢一点点喵~
- 搜索一次最多取 100 个结果，界面再按窗口分页；GitHub 搜索 API 本身最多只能翻到 1000 个结果喵~
- 账号页的额度来自 `/rate_limit`，只读查询；仓库编辑会真的改动 GitHub 上的仓库设置喵~
- 仓库编辑只覆盖简介、主页、主题、可见性和归档，不改仓库名、默认分支和文件内容喵~
- 文件树会把仓库当前分支的树递归取下来，界面最多展示 1200 项喵~
- README 采用轻量 Markdown 处理，不是完整渲染器喵~
- 同一时间只下载一个发行版资产，下载完再点下一份喵~
- 工作流页每个仓库只取最近 30 条运行记录，不是账号级全部记录喵~
- 私有仓库的 Actions 记录需要 `gh` 登录凭据，没登录时会提示失败喵~
- Star 操作会真实修改 GitHub 账号状态，点之前要想一下哦喵~
