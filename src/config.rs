use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// HTTP 请求 User-Agent
pub const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/149.0.0.0 Safari/537.36";

/// 并发请求最大数量
pub const MAX_CONCURRENT_REQUESTS: usize = 5;

/// 单个应用信息
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct App {
    /// 应用名称/标识符
    pub name: String,
    /// 当前版本号
    #[serde(default)]
    pub version: String,
}

/// 总的配置文件结构
#[derive(Default, Serialize, Deserialize, Debug)]
pub struct Config {
    /// 来自 GitHub 的应用列表
    pub github: Vec<App>,
    /// 来自 Liteapks 的应用列表
    pub liteapks: Vec<App>,
}

impl Config {
    /// 从指定路径读取并解析 JSON 配置文件
    ///
    /// # 参数
    /// - `path`: 配置文件路径
    ///
    /// # 返回
    /// 解析成功的 `Config` 实例
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(path.as_ref())?;
        let reader = BufReader::new(file);
        Ok(serde_json::from_reader(reader)?)
    }

    /// 将当前配置写入指定路径的 JSON 文件（美化格式）
    ///
    /// # 参数
    /// - `path`: 配置文件路径
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::create(path.as_ref())?;
        Ok(serde_json::to_writer_pretty(file, self)?)
    }
}

/// 应用版本检查结果
#[derive(Debug)]
pub struct UpdateInfo {
    /// 应用名称
    pub name: String,
    /// 当前版本号
    pub current_version: String,
    /// 新版本号
    pub new_version: String,
    /// 来源平台
    pub platform: Platform,
}

/// 应用来源平台
#[derive(Debug)]
pub enum Platform {
    /// GitHub Releases
    GitHub,
    /// Liteapks
    LiteApks,
}
