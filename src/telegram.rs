use reqwest::Client;
use serde_json::json;
use std::env;
use tracing::info;

/// Telegram API 发送消息响应结构
#[derive(serde::Deserialize, Debug)]
struct TelegramResponse {
    /// 请求是否成功
    ok: bool,
}

/// 向 Telegram 发送 Markdown 格式的消息
///
/// 从环境变量 `TELEGRAM_TOKEN` 和 `TELEGRAM_TO` 分别读取 Bot Token 和目标 Chat ID。
///
/// # 参数
/// - `client`: 复用的 HTTP 客户端
/// - `text`: 消息内容（支持 Markdown 格式）
///
/// # 返回
/// - `true`: 发送成功
/// - `false`: API 返回失败状态
pub async fn send_message(client: &Client, text: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let token = env::var("TELEGRAM_TOKEN")?;
    let chat_id = env::var("TELEGRAM_TO")?;
    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
    let body = json!({
      "chat_id": chat_id,
      "text": text,
      "parse_mode": "Markdown",
      "link_preview_options": { "is_disabled": true },
    });

    let resp: TelegramResponse = client
        .post(url)
        .json(&body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    info!("Telegram 消息发送成功");
    Ok(resp.ok)
}
