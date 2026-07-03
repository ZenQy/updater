use reqwest::Client;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tracing::{info, warn};

use crate::config::{MAX_CONCURRENT_REQUESTS, Platform, USER_AGENT, UpdateInfo};

/// 版本号正则表达式（懒加载，只编译一次）
static VERSION_REGEX: once_cell::sync::Lazy<regex::Regex> =
    once_cell::sync::Lazy::new(|| regex::Regex::new(r#""softwareVersion": "(.*?)""#).unwrap());

/// 并发检查所有 Liteapks 应用的新版本
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
                            "[LiteApks] {} 有新版本: {} -> {}",
                            name, current_version, new_version
                        );
                    }
                    Some(UpdateInfo {
                        name,
                        current_version,
                        new_version,
                        platform: Platform::LiteApks,
                    })
                }
                Err(e) => {
                    warn!("[LiteApks] 检查 {} 失败: {}", name, e);
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
                warn!("LiteApks 检查任务 panic: {:?}", e);
            }
        }
    }

    updates
}

/// 从 Liteapks 页面中提取最新版本号
///
/// 通过正则表达式从页面 HTML 中匹配 `"softwareVersion": "版本号"` 字段。
/// 正则表达式使用懒加载缓存，只编译一次。
///
/// # 参数
/// - `client`: 复用的 HTTP 客户端
/// - `app_name`: Liteapks 应用页面路径名
///
/// # 返回
/// 最新版本号字符串
async fn get_latest_version(
    client: &Client,
    app_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let _timer = Instant::now();

    let body = client
        .get(format!("https://liteapks.com/{}.html", app_name))
        .header("user-agent", USER_AGENT)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    // 使用缓存的正则表达式（只编译一次）
    let caps = VERSION_REGEX
        .captures(body.as_str())
        .ok_or_else(|| format!("在 {} 页面中未找到版本号", app_name))?;
    Ok(caps[1].to_string())
}
