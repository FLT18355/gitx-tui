# gitx-tui

终端里的 GitHub 工作台，使用 ratatui 和 crossterm 写成。仓库、收藏、搜索、设置和仓库详情分别占一页，配上 Catppuccin 配色、卡片列表、Nerd Font 图标和鼠标操作，尽量做得像桌面应用。

界面里没有 emoji，图标都是 Nerd Font 字形。终端字体建议使用 Nerd Font，不然图标会显示成方块。

## 功能

- 我的仓库：通过 GitHub API 读取账号下的公开与私有仓库，支持语言筛选、排序、卡片选择、详情查看和仓库页。
- 收藏仓库：读取当前账号的 Starred 列表，支持排序和详情。
- 搜索仓库：按关键词搜索 GitHub 仓库，支持最佳匹配、星标优先和最近更新排序。
- 仓库详情：展示统计、主题标签、克隆地址、发行版和发行版资产。资产下载地址使用 `v4.gh-proxy.org` 加速。
- 设置页：主题、代理地址、缓存有效期都在设置页里改，并写入配置文件。
- 缓存：仓库列表、收藏、搜索、仓库详情和发行版都存在本地，按有效期自动刷新。
- 鼠标：单击选择、双击打开仓库页、滚轮滚动、点击标签、按钮、主题卡片和设置项。
- 主题：Latte、Frappe、Macchiato、Mocha 四套 Catppuccin 主题，每套一个 TOML。

## 环境要求

- Rust 1.88 或更新版本
- GitHub CLI `gh`
- 已执行 `gh auth login`
- 终端支持鼠标与真彩色
- Nerd Font 字体，用于显示图标

程序通过 `gh auth token --hostname github.com` 读取登录凭据，也支持 `GITHUB_TOKEN` 或 `GH_TOKEN` 环境变量。没有登录时仍然可以搜索公开仓库和查看公开仓库详情。

## 构建与运行

```bash
cargo build --release
./target/release/gitx-tui
```

也可以临时指定主题：

```bash
./target/release/gitx-tui --theme latte
```

安装到 Cargo 的 bin 目录：

```bash
cargo install --path .
```

## 页面与操作

| 页面 | 内容 |
| --- | --- |
| 仓库 | 账号仓库列表、语言筛选、排序、详情和发行版 |
| 收藏 | Starred 仓库列表、统计、排序和详情 |
| 搜索 | 搜索框、排序、历史记录、结果列表和详情 |
| 设置 | 配置文件路径、主题、代理、缓存时间和清理缓存 |
| 仓库详情 | 统计、关于、主题、克隆地址、发行版与资产下载 |

### 键盘

| 按键 | 作用 |
| --- | --- |
| `1` `2` `3` `4` | 切换仓库、收藏、搜索、设置 |
| `Tab` | 切换到下一页 |
| `j` `k` | 上下移动 |
| `g` `G` | 跳到首项或末项 |
| `Enter` | 打开仓库页；设置页里执行当前项 |
| `Esc` | 离开输入、关闭帮助或返回列表 |
| `/` | 聚焦搜索 |
| `o` | 用浏览器打开当前仓库 |
| `y` | 复制当前仓库的克隆地址 |
| `r` | 刷新当前页 |
| `h` `l` | 在设置页调整当前项 |
| `Ctrl+S` | 保存设置 |
| `?` | 打开操作说明 |
| `q` | 退出 |

在搜索框里输入时，字母会正常进入输入框。搜索框里按 `Enter` 开始搜索，按 `Esc` 离开输入框。

### 鼠标

- 点击顶部标签切换页面。
- 点击仓库卡片选中，双击打开仓库页。
- 点击排序标签、语言筛选、主题卡片和按钮。
- 滚轮滚动列表、详情和发行版区域。

## 配置文件

项目根目录的 `config.toml` 是随程序发布的默认配置。第一次运行时，程序会把它写到：

```text
~/.config/gitx-tui/config.toml
```

设置页里的改动会写入这个文件。也可以直接编辑，重启后生效。

```toml
theme = "mocha"
proxy = "https://v4.gh-proxy.org"
repos_ttl_secs = 300
detail_ttl_secs = 900
search_ttl_secs = 600
search_history = []
```

| 字段 | 说明 |
| --- | --- |
| `theme` | `latte`、`frappe`、`macchiato`、`mocha` |
| `proxy` | 公开 API 与发行版下载的加速地址 |
| `repos_ttl_secs` | 仓库和收藏列表的缓存时间，单位秒 |
| `detail_ttl_secs` | 仓库详情和发行版的缓存时间，单位秒 |
| `search_ttl_secs` | 搜索结果的缓存时间，单位秒 |
| `search_history` | 最近搜索，最多保留 8 条 |

## 主题

主题文件放在 `themes/` 下：

```text
themes/latte.toml
themes/frappe.toml
themes/macchiato.toml
themes/mocha.toml
```

四套都是 Catppuccin 官方色板，包含 base、mantle、crust、surface、overlay、text 以及强调色。程序优先读取当前目录的 `themes/`，其次读取 `~/.config/gitx-tui/themes/`，找不到时使用编译进二进制的同一套主题。

## 网络与缓存

公开 API（仓库详情、发行版、搜索）会先请求：

```text
https://v4.gh-proxy.org/https://api.github.com/...
```

发行版资产链接也会改写成：

```text
https://v4.gh-proxy.org/https://github.com/owner/repo/releases/download/...
```

认证数据（`/user`、`/user/repos`、`/user/starred`）与私有仓库详情直接访问 GitHub API。这样处理有两个原因：

1. `gh` 的登录凭据不会发送给第三方代理。
2. 公共代理不会转发 `Authorization`，也不适合承载私有仓库数据。

如果公开代理暂时不可用或额度用尽，程序会回退到直连公开 API，避免页面一直空着。

缓存目录：

```text
~/.cache/gitx-tui
```

数据按更新时间保存为 JSON。设置页可以清理缓存，也可以在配置里调整各缓存的有效期。

## 目录结构

```text
src/api.rs      GitHub API、gh 凭据、代理与直连回退
src/app.rs      页面状态、异步消息、键鼠事件、缓存读写
src/cache.rs    JSON 缓存
src/config.rs   配置加载与保存
src/icons.rs    Nerd Font 图标
src/model.rs    GitHub 数据结构
src/theme.rs    Catppuccin 主题加载
src/ui.rs       全部页面渲染与命中区域
themes/         四套主题 TOML
config.toml     随项目发布的默认配置
```

## 测试

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

## 已知限制

- 仓库列表和收藏列表每次最多读取 100 个。
- 搜索一次展示前 40 个结果。
- 目前只读，不会修改星标、创建仓库或发起 Pull Request。
- 发行版资产通过代理交给浏览器打开，是否成功取决于终端和系统是否配置了打开链接的程序。
