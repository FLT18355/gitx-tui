use crate::format::base64_decode;
use crate::model::{
    RateLimit, ReadmeContent, Release, Repo, SearchOutcome, SearchResponse, SearchSort,
    TreeResponse, User, Workflow, WorkflowJob, WorkflowJobsResponse, WorkflowListResponse,
    WorkflowRun, WorkflowRunsResponse,
};
use serde::de::DeserializeOwned;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

const API: &str = "https://api.github.com";
const USER_AGENT: &str = "gxt/0.1";

#[derive(Clone)]
pub struct ApiClient {
    agent: ureq::Agent,
    download_agent: ureq::Agent,
    token: Option<String>,
    proxy: String,
}

impl ApiClient {
    pub fn new(proxy: String) -> Self {
        let agent = build_agent(Duration::from_secs(15));
        let download_agent = build_agent(Duration::from_secs(3600));
        let token = detect_token();
        Self {
            agent,
            download_agent,
            token,
            proxy: proxy.trim_end_matches('/').to_string(),
        }
    }

    pub fn without_auth(proxy: String) -> Self {
        Self {
            agent: build_agent(Duration::from_secs(15)),
            download_agent: build_agent(Duration::from_secs(3600)),
            token: None,
            proxy: proxy.trim_end_matches('/').to_string(),
        }
    }

    pub fn token_present(&self) -> bool {
        self.token.is_some()
    }

    pub fn set_proxy(&mut self, proxy: &str) {
        self.proxy = proxy.trim_end_matches('/').to_string();
    }

    pub fn proxy(&self) -> &str {
        &self.proxy
    }

    pub fn user(&self) -> Result<User, String> {
        self.get_json(&format!("{API}/user"), true)
    }

    pub fn repositories(&self) -> Result<Vec<Repo>, String> {
        self.get_pages(
            &format!(
                "{API}/user/repos?per_page=100&sort=updated&direction=desc&affiliation=owner,collaborator,organization_member"
            ),
            true,
        )
    }

    pub fn starred(&self) -> Result<Vec<Repo>, String> {
        self.get_pages(
            &format!("{API}/user/starred?per_page=100&sort=updated&direction=desc"),
            true,
        )
    }

    pub fn repository(&self, full_name: &str, private: bool) -> Result<Repo, String> {
        let url = format!("{API}/repos/{full_name}");
        if private {
            self.get_json(&url, true)
        } else {
            self.get_public_with_fallback(&url)
        }
    }

    pub fn releases(&self, full_name: &str, private: bool) -> Result<Vec<Release>, String> {
        let url = format!("{API}/repos/{full_name}/releases?per_page=10");
        if private {
            self.get_json(&url, true)
        } else {
            self.get_public_with_fallback(&url)
        }
    }

    pub fn rate_limit(&self) -> Result<RateLimit, String> {
        self.get_json(&format!("{API}/rate_limit"), self.token.is_some())
    }

    pub fn update_repository(
        &self,
        full_name: &str,
        patch: &serde_json::Value,
    ) -> Result<Repo, String> {
        let Some(token) = self.token.as_deref() else {
            return Err("还没有登录 GitHub，改不了仓库哦喵~先运行 gh auth login 吧喵~".to_string());
        };
        let url = format!("{API}/repos/{full_name}");
        let response = self
            .agent
            .patch(&url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("Authorization", format!("Bearer {token}"))
            .send_json(patch)
            .map_err(|err| format_transport_error(&err))?;
        let status = response.status().as_u16();
        let mut response = response;
        let body = response.body_mut().read_to_string().unwrap_or_default();
        if (200..300).contains(&status) {
            serde_json::from_str(&body)
                .map_err(|err| format!("GitHub 改完仓库送回来的数据人家看不懂：{err}喵~"))
        } else {
            Err(status_message(status, &body))
        }
    }

    pub fn action_runs(&self, full_name: &str, private: bool) -> Result<Vec<WorkflowRun>, String> {
        let url = format!("{API}/repos/{full_name}/actions/runs?per_page=30");
        let response: WorkflowRunsResponse = if private {
            self.get_json(&url, true)?
        } else {
            self.get_public_with_fallback(&url)?
        };
        Ok(response.workflow_runs)
    }

    pub fn action_jobs(
        &self,
        full_name: &str,
        run_id: u64,
        private: bool,
    ) -> Result<Vec<WorkflowJob>, String> {
        let url = format!("{API}/repos/{full_name}/actions/runs/{run_id}/jobs?per_page=30");
        let response: WorkflowJobsResponse = if private {
            self.get_json(&url, true)?
        } else {
            self.get_public_with_fallback(&url)?
        };
        Ok(response.jobs)
    }

    pub fn workflows(&self, full_name: &str, private: bool) -> Result<Vec<Workflow>, String> {
        let url = format!("{API}/repos/{full_name}/actions/workflows?per_page=100");
        let response: WorkflowListResponse = if private {
            self.get_json(&url, true)?
        } else {
            self.get_public_with_fallback(&url)?
        };
        Ok(response.workflows)
    }

    pub fn dispatch_workflow(
        &self,
        full_name: &str,
        workflow_id: u64,
        git_ref: &str,
        inputs: Option<serde_json::Value>,
    ) -> Result<(), String> {
        let Some(token) = self.token.as_deref() else {
            return Err(
                "还没有登录 GitHub，跑不了工作流哦喵~先运行 gh auth login 吧喵~".to_string(),
            );
        };
        let url = format!("{API}/repos/{full_name}/actions/workflows/{workflow_id}/dispatches");
        let mut payload = serde_json::Map::new();
        payload.insert("ref".to_string(), serde_json::json!(git_ref));
        if let Some(inputs) = inputs {
            payload.insert("inputs".to_string(), inputs);
        }
        let payload = serde_json::Value::Object(payload);
        let response = self
            .agent
            .post(&url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("Authorization", format!("Bearer {token}"))
            .send_json(&payload)
            .map_err(|err| format_transport_error(&err))?;
        let status = response.status().as_u16();
        let mut response = response;
        let body = response.body_mut().read_to_string().unwrap_or_default();
        if (200..300).contains(&status) {
            Ok(())
        } else {
            Err(status_message(status, &body))
        }
    }

    pub fn search(&self, query: &str, sort: SearchSort) -> Result<SearchOutcome, String> {
        let encoded = percent_encode(query.trim());
        let url = format!(
            "{API}/search/repositories?q={encoded}&per_page=100{}",
            sort.api_params()
        );
        let response: SearchResponse = self.get_public_with_fallback(&url)?;
        Ok(SearchOutcome {
            total: response.total_count,
            items: response.items,
        })
    }

    pub fn file_tree(
        &self,
        full_name: &str,
        branch: &str,
        private: bool,
    ) -> Result<TreeResponse, String> {
        let branch = if branch.trim().is_empty() {
            "HEAD"
        } else {
            branch
        };
        let url = format!(
            "{API}/repos/{full_name}/git/trees/{}?recursive=1",
            percent_encode(branch)
        );
        if private {
            self.get_json(&url, true)
        } else {
            self.get_public_with_fallback(&url)
        }
    }

    pub fn readme(&self, full_name: &str, private: bool) -> Result<ReadmeContent, String> {
        let url = format!("{API}/repos/{full_name}/readme");
        let raw: ReadmeApi = if private {
            self.get_json(&url, true)?
        } else {
            self.get_public_with_fallback(&url)?
        };
        if raw.encoding != "base64" {
            return Ok(ReadmeContent {
                name: raw.name,
                path: raw.path,
                html_url: raw.html_url,
                content: raw.content,
            });
        }
        let bytes = base64_decode(&raw.content)
            .ok_or_else(|| "README 的编码人家没看懂，暂时读不出来喵~".to_string())?;
        let content = String::from_utf8(bytes)
            .map_err(|_| "README 不是 UTF-8 文本，人家读不了呢喵~".to_string())?;
        Ok(ReadmeContent {
            name: raw.name,
            path: raw.path,
            html_url: raw.html_url,
            content,
        })
    }

    pub fn set_starred(&self, full_name: &str, starred: bool) -> Result<(), String> {
        let Some(token) = self.token.as_deref() else {
            return Err("还没有登录 GitHub，改不了收藏哦喵~先运行 gh auth login 吧喵~".to_string());
        };
        let url = format!("{API}/user/starred/{full_name}");
        let request = if starred {
            self.agent
                .put(&url)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .header("Authorization", format!("Bearer {token}"))
                .send_empty()
        } else {
            self.agent
                .delete(&url)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .header("Authorization", format!("Bearer {token}"))
                .call()
        };
        let response = request.map_err(|err| format_transport_error(&err))?;
        let status = response.status().as_u16();
        let mut response = response;
        let body = response.body_mut().read_to_string().unwrap_or_default();
        if (200..300).contains(&status) {
            Ok(())
        } else {
            Err(status_message(status, &body))
        }
    }

    pub fn download_asset<F>(
        &self,
        asset_url: &str,
        private: bool,
        destination: &Path,
        mut progress: F,
    ) -> Result<u64, String>
    where
        F: FnMut(u64, Option<u64>),
    {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("下载目录建不出来：{err}喵~检查一下设置里的路径嘛喵~"))?;
        }
        let attempts: Vec<(String, bool)> = if private {
            vec![(asset_url.to_string(), true)]
        } else {
            vec![
                (self.accelerated(asset_url), false),
                (asset_url.to_string(), self.token.is_some()),
            ]
        };
        let mut errors = Vec::new();
        for (url, authenticated) in attempts {
            match self.download_once(&url, authenticated, destination, &mut progress) {
                Ok(size) => return Ok(size),
                Err(error) => errors.push(error),
            }
        }
        Err(format!("下载没有成功：{}喵~", errors.join("；")))
    }

    pub fn accelerated(&self, github_url: &str) -> String {
        join_proxy(&self.proxy, github_url)
    }

    fn get_pages<T: DeserializeOwned>(
        &self,
        base: &str,
        authenticated: bool,
    ) -> Result<Vec<T>, String> {
        let mut all = Vec::new();
        for page in 1..=1000 {
            let separator = if base.contains('?') { '&' } else { '?' };
            let url = format!("{base}{separator}page={page}");
            let chunk: Vec<T> = self.get_json(&url, authenticated)?;
            let count = chunk.len();
            all.extend(chunk);
            if count < 100 {
                break;
            }
        }
        Ok(all)
    }

    fn download_once<F>(
        &self,
        url: &str,
        authenticated: bool,
        destination: &Path,
        progress: &mut F,
    ) -> Result<u64, String>
    where
        F: FnMut(u64, Option<u64>),
    {
        let mut request = self
            .download_agent
            .get(url)
            .header("Accept", "application/octet-stream");
        if authenticated && let Some(token) = self.token.as_deref() {
            request = request.header("Authorization", format!("Bearer {token}"));
        }
        let mut response = request.call().map_err(|err| format_transport_error(&err))?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            let body = response.body_mut().read_to_string().unwrap_or_default();
            return Err(status_message(status, &body));
        }
        let total = response
            .headers()
            .get("content-length")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        let mut part = destination.as_os_str().to_os_string();
        part.push(format!(".part-{}", std::process::id()));
        let part = PathBuf::from(part);
        let mut file = fs::File::create(&part).map_err(|err| {
            let _ = fs::remove_file(&part);
            format!("临时文件写不进去：{err}喵~")
        })?;
        let mut reader = response.body_mut().as_reader();
        let mut buffer = [0_u8; 64 * 1024];
        let mut received = 0_u64;
        let mut last_emit = Instant::now();
        progress(0, total);
        loop {
            let read = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => read,
                Err(err) => {
                    drop(file);
                    let _ = fs::remove_file(&part);
                    return Err(format!("下载到一半断掉了：{err}喵~再点一次试试嘛喵~"));
                }
            };
            if let Err(err) = file.write_all(&buffer[..read]) {
                drop(file);
                let _ = fs::remove_file(&part);
                return Err(format!("文件写不进去：{err}喵~"));
            }
            received += read as u64;
            if last_emit.elapsed() >= Duration::from_millis(120) {
                progress(received, total);
                last_emit = Instant::now();
            }
        }
        if let Err(err) = file.flush() {
            drop(file);
            let _ = fs::remove_file(&part);
            return Err(format!("文件没有写完：{err}喵~"));
        }
        drop(file);
        if let Err(err) = fs::rename(&part, destination) {
            let _ = fs::remove_file(&part);
            return Err(format!("下载好了，但是改名没成功：{err}喵~"));
        }
        progress(received, total);
        Ok(received)
    }

    fn get_public_with_fallback<T: DeserializeOwned>(&self, url: &str) -> Result<T, String> {
        let accelerated = self.accelerated(url);
        match self.get_json(&accelerated, false) {
            Ok(value) => Ok(value),
            Err(proxy_error) => {
                let direct = self.get_json(url, self.token.is_some());
                match direct {
                    Ok(value) => Ok(value),
                    Err(direct_error) => Err(format!(
                        "加速通道没有成功：{proxy_error}；直连也没有成功：{direct_error}喵~"
                    )),
                }
            }
        }
    }

    fn get_json<T: DeserializeOwned>(&self, url: &str, authenticated: bool) -> Result<T, String> {
        if authenticated && self.token.is_none() {
            return Err(
                "还没有可用的 GitHub 凭据，人家读不到你的仓库喵~先在终端运行 gh auth login 哦喵~"
                    .to_string(),
            );
        }
        let body = self.fetch_text(url, authenticated)?;
        serde_json::from_str(&body)
            .map_err(|err| format!("GitHub 送来的数据人家看不懂：{err}喵~稍后再试试嘛喵~"))
    }

    fn fetch_text(&self, url: &str, authenticated: bool) -> Result<String, String> {
        let mut last_error = String::new();
        for attempt in 0..2 {
            if attempt > 0 {
                std::thread::sleep(Duration::from_millis(350 * attempt as u64));
            }
            let mut request = self
                .agent
                .get(url)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28");
            if authenticated && let Some(token) = self.token.as_deref() {
                request = request.header("Authorization", format!("Bearer {token}"));
            }
            let response = match request.call() {
                Ok(response) => response,
                Err(err) => {
                    last_error = format_transport_error(&err);
                    continue;
                }
            };
            let status = response.status().as_u16();
            let mut response = response;
            let body = match response.body_mut().read_to_string() {
                Ok(body) => body,
                Err(err) => {
                    last_error = format_transport_error(&err);
                    continue;
                }
            };
            if (200..300).contains(&status) {
                return Ok(body);
            }
            if status == 429 || status >= 500 {
                last_error = status_message(status, &body);
                continue;
            }
            return Err(status_message(status, &body));
        }
        if last_error.is_empty() {
            last_error = "网络暂时没有响应，再试一次吧喵~".to_string();
        }
        Err(last_error)
    }
}

#[derive(serde::Deserialize)]
struct ReadmeApi {
    #[serde(default)]
    name: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    encoding: String,
}

fn build_agent(timeout: Duration) -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .timeout_connect(Some(Duration::from_secs(10)))
        .http_status_as_error(false)
        .user_agent(USER_AGENT)
        .build();
    config.into()
}

fn detect_token() -> Option<String> {
    if let Ok(output) = Command::new("gh")
        .args(["auth", "token", "--hostname", "github.com"])
        .output()
        && output.status.success()
    {
        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !token.is_empty() {
            return Some(token);
        }
    }
    for key in ["GITHUB_TOKEN", "GH_TOKEN"] {
        if let Ok(token) = std::env::var(key) {
            let token = token.trim().to_string();
            if !token.is_empty() {
                return Some(token);
            }
        }
    }
    None
}

fn join_proxy(base: &str, github_url: &str) -> String {
    let base = base.trim_end_matches('/');
    if base.is_empty() || github_url.starts_with(&format!("{base}/")) {
        github_url.to_string()
    } else {
        format!("{base}/{github_url}")
    }
}

fn percent_encode(input: &str) -> String {
    let mut out = String::new();
    for byte in input.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}

fn status_message(status: u16, body: &str) -> String {
    let api_message = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("message")
                .and_then(|message| message.as_str())
                .map(str::to_string)
        })
        .filter(|message| !message.trim().is_empty());
    match status {
        401 => "GitHub 凭据失效啦，运行 gh auth login 重新登录一下嘛喵~".to_string(),
        404 => "这个仓库人家找不到呢，可能是私有仓库，也可能已经被删掉了喵~".to_string(),
        403 => format!(
            "GitHub 不想给人家开门（403）喵~ {}",
            api_message.unwrap_or_else(|| "可能是访问太频繁啦，等几分钟再试嘛喵~".to_string())
        ),
        429 => "请求有点太密集啦，GitHub 让人家缓一缓喵~过一会儿再刷新嘛喵~".to_string(),
        _ => {
            let detail = api_message.unwrap_or_else(|| {
                let compact = body.split_whitespace().collect::<Vec<_>>().join(" ");
                if compact.is_empty() {
                    "它也没多说呢喵~".to_string()
                } else {
                    compact.chars().take(140).collect()
                }
            });
            format!("GitHub 回了 {status} 呢：{detail} 喵~")
        }
    }
}

fn format_transport_error(error: &ureq::Error) -> String {
    format!("网络没连上呢，人家再试一次：{error}喵~")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_joins_exactly_once() {
        let base = "https://v4.gh-proxy.org";
        let url = "https://api.github.com/repos/ratatui/ratatui";
        let first = join_proxy(base, url);
        assert_eq!(
            first,
            "https://v4.gh-proxy.org/https://api.github.com/repos/ratatui/ratatui"
        );
        assert_eq!(join_proxy(base, &first), first);
    }

    #[test]
    fn encodes_search_queries() {
        assert_eq!(percent_encode("ratatui"), "ratatui");
        assert_eq!(
            percent_encode("topic:cli stars:>100"),
            "topic%3Acli%20stars%3A%3E100"
        );
        assert_eq!(percent_encode("中文"), "%E4%B8%AD%E6%96%87");
    }

    #[test]
    fn proxy_url_for_release_asset() {
        let base = "https://v4.gh-proxy.org";
        let asset = "https://github.com/owner/repo/releases/download/v1/app.tar.gz";
        assert_eq!(
            join_proxy(base, asset),
            "https://v4.gh-proxy.org/https://github.com/owner/repo/releases/download/v1/app.tar.gz"
        );
    }
}
