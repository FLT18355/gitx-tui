# CLI 用法

`gxt` 是 gitx-tui 编译出的二进制名。完整参数以 `gxt -h` 为准，本文给出常用组合。

## 命令总览

| 命令 | 作用 |
| --- | --- |
| `gxt` | 打开 GitHub 工作台 |
| `gxt <路径>` | 打开本地仓库 TUI |
| `gxt settings` | 打开全局设置 TUI |
| `gxt status [路径]` | 打印本地仓库状态 |
| `gxt log [路径] [-n <条数>]` | 打印最近提交 |
| `gxt tags [路径]` | 打印标签 |
| `gxt push [路径]` | 推送当前分支 |
| `gxt config [show\|path]` | 查看配置 |
| `gxt cache [stats\|path\|clear]` | 查看或清理缓存 |
| `gxt doctor` | 检查 git、GitHub 登录、配置和缓存 |

路径参数省略时使用当前目录。

## 选项

| 选项 | 说明 |
| --- | --- |
| `--theme <latte\|frappe\|macchiato\|mocha>` | 指定主题 |
| `-n`, `--limit <条数>` | `log` 显示条数，默认 20 |
| `-h`, `--help` | 显示 CLI 帮助 |
| `-V`, `--version` | 显示版本 |

## 示例

```bash
gxt
gxt ~/code/demo
gxt settings
gxt status .
gxt log . -n 50
gxt tags .
gxt push .
gxt config show
gxt cache clear
gxt doctor
```

## 退出码

| 退出码 | 含义 |
| --- | --- |
| `0` | 正常结束 |
| `1` | 运行命令失败，例如不是 git 仓库或 git 操作失败 |
| `2` | 参数错误 |

TUI 键位见 [KEYBINDINGS.md](KEYBINDINGS.md)。