# 开发与构建

## 环境

- Rust 1.88 或更新版本，edition 2024。
- git 与 GitHub CLI `gh`。
- 终端需要支持真彩色；推荐 Nerd Font。

## 构建与运行

产物名是 `gxt`：

```bash
cargo build --release
./target/release/gxt
```

本地安装：

```bash
./install.sh
./install.sh --dir /some/bin
./install.sh --debug
```

也可以使用 Cargo：

```bash
cargo install --path .
```

## 测试

```bash
cargo fmt
cargo test
cargo clippy --all-targets -- -D warnings
```

## 模块

| 文件 | 用途 |
| --- | --- |
| `src/main.rs` | 入口、TUI 事件循环、CLI 命令分派 |
| `src/cli.rs` | 参数解析与 CLI 帮助 |
| `src/commands.rs` | status/log/tags/push/config/cache/doctor 一次性命令 |
| `src/app.rs` | GitHub 页面状态、设置模式、Actions、议题、通知、缓存和键鼠事件 |
| `src/ui.rs` | GitHub 工作台、议题/通知页与设置 TUI 渲染 |
| `src/api.rs` | GitHub API、gh 凭据、代理、下载与工作流触发 |
| `src/model.rs` | GitHub serde 数据结构 |
| `src/config.rs` | 配置字段、TTL 与路径 |
| `src/cache.rs` | JSON 缓存 |
| `src/local.rs` | 本地仓库数据读取与 porcelain 解析 |
| `src/local_git.rs` | git 命令执行 |
| `src/local_ops.rs` | diff、暂存、提交、分支、标签、push/pull/fetch |
| `src/local_app.rs` | 本地仓库 TUI 状态、按键与后台任务 |
| `src/local_ui.rs` | 本地仓库 TUI 渲染 |
| `src/theme.rs` | Catppuccin 主题加载 |

## 发布

`.github/workflows/release.yml` 在推送 `v*` tag 时构建 Linux/macOS 包并创建 Release。版本号来自 `Cargo.toml`，构建产物为 `gxt`，压缩包名为 `gxt-<version>-<target>.tar.gz`。

发布前确认：

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

并确保 `Cargo.toml` / `Cargo.lock` 的版本与 tag 一致。