# gitx-tui 开发命令
set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

version := `grep '^version' Cargo.toml | head -1 | cut -d '"' -f2`

default:
    @just --list

# 编译 debug 版
build:
    cargo build

# 编译 release 版
release:
    cargo build --release

# 运行 GitHub 工作台
run *args:
    cargo run -- {{args}}

# 打开 Markdown 预览器
md *args:
    cargo run -- md {{args}}

# 打开个人发行版发布 TUI
release-tui:
    cargo run -- release

# 打开设置 TUI
settings:
    cargo run -- settings

# 打开本地仓库 TUI
local path=".":
    cargo run -- {{path}}

# 格式化代码
fmt:
    cargo fmt

# 检查格式，不改文件
fmt-check:
    cargo fmt --check

# Clippy，警告即错误
lint:
    cargo clippy --all-targets -- -D warnings

# 跑测试
test:
    cargo test

# 发布前全套检查
check: fmt-check lint test

# 安装到 ~/.cargo/bin
install:
    cargo install --path .

# 用项目脚本安装到指定目录
install-local *args:
    ./install.sh {{args}}

# 清理构建产物
clean:
    cargo clean

# 给当前版本打 tag
tag:
    git tag -a "v{{version}}" -m "v{{version}}"