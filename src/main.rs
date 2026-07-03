//! # 应用版本更新检查工具
//!
//! 该工具从 `config.json` 中读取应用列表，并发检查以下平台的最新版本：
//!
//! - **GitHub Releases**: 通过 GitHub API 获取最新 release 版本号
//! - **Liteapks**: 通过解析页面 HTML 提取软件版本号
//!
//! 当发现新版本时，通过 Telegram Bot 发送通知，并自动更新 `config.json` 中的版本号。

use reqwest::Client;
use std::time::Duration;

use crate::config::App;

mod config;
mod github;
mod liteapks;
mod telegram;

const CONFIG_PATH: &str = "config.json";

/// 程序入口
///
/// 流程：
/// 1. 读取配置文件
/// 2. 并发检查 GitHub 和 Liteapks 的新版本
/// 3. 合并更新结果，更新配置并生成通知消息
/// 4. 如果有新版本，发送 Telegram 通知
/// 5. 写回配置文件
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    let cfg = config::Config::from_file(CONFIG_PATH)?;

    // 并发检查 GitHub 和 Liteapks
    let (github_updates, liteapks_updates) = tokio::join!(
        github::check_updates(&client, &cfg.github),
        liteapks::check_updates(&client, &cfg.liteapks),
    );

    let all_updates = github_updates
        .into_iter()
        .chain(liteapks_updates)
        .collect::<Vec<_>>();

    let (cfg, message) = build_update_message(&all_updates);
    telegram::send_message(&client, &message).await?;

    cfg.to_file(CONFIG_PATH)?;
    Ok(())
}

/// 根据更新结果构建 Telegram 通知消息，并更新配置中的版本号
///
/// # 参数
/// - `cfg`: 可变配置引用，版本号会被更新为最新值
/// - `updates`: 所有有版本更新的应用列表
///
/// # 返回
/// 格式化的 Telegram 通知消息字符串
fn build_update_message(updates: &[config::UpdateInfo]) -> (config::Config, String) {
    let mut cfg = config::Config::default();
    let mut lines = Vec::new();

    for info in updates {
        match info.platform {
            config::Platform::GitHub => {
                if info.new_version != info.current_version {
                    lines.push(format!(
                        "[🔗{}({})](https://github.com/{}/releases/tag/{})",
                        info.name, info.new_version, info.name, info.new_version
                    ));
                }
                cfg.github.push(App {
                    name: info.name.clone(),
                    version: info.new_version.clone(),
                });
            }
            config::Platform::LiteApks => {
                if info.new_version != info.current_version {
                    lines.push(format!(
                        "[🔗{}({})](https://liteapks.com/{}.html)",
                        info.name, info.new_version, info.name
                    ));
                }
                cfg.liteapks.push(App {
                    name: info.name.clone(),
                    version: info.new_version.clone(),
                });
            }
        }
    }

    (cfg, format!("以下软件可以更新：{}", lines.join("\n")))
}
