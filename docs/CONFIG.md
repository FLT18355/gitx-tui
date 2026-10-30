# 配置、网络与缓存

## 配置文件

默认路径：

```text
~/.config/gitx-tui/config.toml
```

`gxt settings` 会写入这个文件；也可以手动编辑，重启后生效。

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
| `repos_ttl_secs` | 仓库和收藏列表缓存时间 |
| `detail_ttl_secs` | 仓库详情、文件树、README、发行版和 Actions 缓存时间 |
| `search_ttl_secs` | 搜索结果缓存时间 |
| `search_history` | 最近搜索，最多 8 条 |
| `show_file_tree` | 是否加载仓库文件树 |
| `show_readme` | 是否加载 README |
| `download_dir` | 下载目录，支持 `~/` 开头 |

## 主题

主题文件位于 `themes/`：

```text
themes/latte.toml
themes/frappe.toml
themes/macchiato.toml
themes/mocha.toml
```

优先读取当前目录的 `themes/`，其次读取 `~/.config/gitx-tui/themes/`，最后使用编译进二进制的同一套主题。

## 网络

公开 API（仓库详情、文件树、README、发行版、搜索）先走代理：

```text
https://v4.gh-proxy.org/https://api.github.com/...
```

发行版资产和源码包下载先走：

```text
https://v4.gh-proxy.org/https://github.com/...
```

以下请求直接访问 GitHub：

- `/user`、`/user/repos`、`/user/starred`、`/rate_limit`
- 私有仓库详情与私有仓库 Actions
- 手动触发工作流
- Star 操作
- 仓库编辑

这样登录凭据不会发给第三方代理，公共代理也不会承载私有仓库数据。公开代理失败时会回退到直连；发行版下载失败也会尝试直连。

## 缓存

缓存目录：

```text
~/.cache/gitx-tui
```

数据按更新时间保存为 JSON。`gxt cache` 可以查看统计，`gxt cache clear` 或 `gxt settings` 里的缓存管理可以清理。缓存键包括：

- `repos_*`、`starred_*`
- `search_*`
- `repo_*`、`releases_*`、`tree_*`、`readme_*`
- `actions_*`、`actions_jobs_*`、`actions_workflows_*`
- `user`、`rate_limit`