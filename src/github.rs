use reqwest::Client;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tracing::{info, warn};

use crate::config::{MAX_CONCURRENT_REQUESTS, Platform, USER_AGENT, UpdateInfo};

/// GitHub API 最新 release 响应结构
#[derive(serde::Deserialize)]
struct GithubRelease {
    /// 版本标签名，如 `v1.2.3`
    tag_name: String,
}

/// 并发检查所有 GitHub 应用的新版本
///
/// 每个应用独立通过 `tokio::spawn` 发起异步请求，单个失败不影响其他应用。
/// 同时使用 `Semaphore` 限制最大并发请求数量，防止触发 API 限流。
///
/// # 参数
/// - `client`: 复用的 HTTP 客户端
/// - `apps`: 应用列表（含当前版本号）
///
/// # 返回
/// 有版本更新的应用信息列表
pub async fn check_updates(client: &Client, apps: &[crate::config::App]) -> Vec<UpdateInfo> {
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));
    let mut handles = Vec::new();

    for app in apps {
        let client = client.clone();
        let name = app.name.clone();
        let current_version = app.version.clone();
        // 获取信号量许可（最多 MAX_CONCURRENT_REQUESTS 个并发）
        let permit = semaphore.clone().acquire_owned().await.unwrap();

        let handle = tokio::spawn(async move {
            let _permit = permit; // 保持信号量计数在闭包中有效
            match get_latest_version(&client, &name).await {
                Ok(new_version) => {
                    if new_version != current_version {
                        info!(
                            "[GitHub] {} 有新版本: {} -> {}",
                            name, current_version, new_version
                        );
                    }
                    Some(UpdateInfo {
                        name,
                        current_version,
                        new_version,
                        platform: Platform::GitHub,
                    })
                }
                Err(e) => {
                    warn!("[GitHub] 检查 {} 失败: {}", name, e);
                    None
                }
            }
        });
        handles.push(handle);
    }

    let mut updates = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(Some(info)) => updates.push(info),
            Ok(None) => {}
            Err(e) => {
                warn!("GitHub 检查任务 panic: {:?}", e);
            }
        }
    }

    updates
}

/// 获取指定 GitHub 仓库的最新 release 版本号
///
/// 调用 `GET /repos/{owner}/{repo}/releases/latest` API。
/// 使用 [`ClientBuilder`] 添加超时控制，防止长时间阻塞。
///
/// # 参数
/// - `client`: 复用的 HTTP 客户端
/// - `app_name`: 仓库全名，格式为 `owner/repo`
///
/// # 返回
/// 最新版本标签名（如 `v1.2.3`）
async fn get_latest_version(
    client: &Client,
    app_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let _timer = Instant::now();

    let body: GithubRelease = client
        .get(format!(
            "https://api.github.com/repos/{}/releases/latest",
            app_name
        ))
        .header("user-agent", USER_AGENT)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(body.tag_name)
}
