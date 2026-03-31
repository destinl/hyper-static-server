/// hyper-static-server 库入口
///
/// 高性能静态文件服务器，基于 tokio + axum 构建。
///
/// # 功能特性
/// - 静态文件服务 (200 OK)
/// - 目录列表生成 (HTML autoindex)
/// - ETag 和 Last-Modified 缓存支持
/// - Range 请求支持 (206 Partial Content)
/// - CORS 配置
/// - 目录遍历防护
///
/// # 使用示例
/// ```rust,no_run
/// use hyper_static_server::{ServerConfig, server};
/// use std::path::PathBuf;
///
/// #[tokio::main]
/// async fn main() {
///     let config = ServerConfig {
///         host: "127.0.0.1".to_string(),
///         port: 3000,
///         root_dir: PathBuf::from("./public"),
///         cors: false,
///         follow_symlinks: false,
///     };
///     server::start_server(config).await.unwrap();
/// }
/// ```

pub mod error;
pub mod mime;
pub mod response;
pub mod json_formatter;
pub mod server;

// 重新导出常用类型
pub use error::{ServerError, ServerResult};
pub use server::{ServerConfig, start_server};
pub use mime::detect_mime_type;
pub use response::FileMetadata;
pub use json_formatter::{format_json, JsonFormattingResult};
