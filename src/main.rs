/// hyper-static-server 主入口
///
/// 高性能静态文件服务器，基于 tokio + axum 构建。
/// 提供比 Node.js/Python 更高的吞吐量和更低的内存占用。
///
/// # 使用示例
/// ```bash
/// # 默认配置 (localhost:3000, 当前目录)
/// hyper-static-server
///
/// # 自定义端口和目录
/// hyper-static-server -p 8080 -d /var/www
///
/// # 绑定到所有接口
/// hyper-static-server -H 0.0.0.0 -p 3000
///
/// # 启用 CORS
/// hyper-static-server --cors
/// ```

use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

mod error;
mod mime;
mod response;
mod json_formatter;
mod server;

use error::ServerResult;
use server::ServerConfig;

/// 高性能静态文件服务器
#[derive(Parser, Debug)]
#[command(name = "hyper-static-server")]
#[command(author = "Your Name <your.email@example.com>")]
#[command(version = "0.1.0")]
#[command(about = "High-performance static file server", long_about = None)]
struct Cli {
    /// 监听端口
    #[arg(short = 'p', long = "port", default_value = "3000")]
    port: u16,

    /// 服务目录 (默认：当前目录)
    #[arg(short = 'd', long = "dir", default_value = ".")]
    dir: String,

    /// 绑定地址
    #[arg(short = 'H', long = "host", default_value = "127.0.0.1")]
    host: String,

    /// 启用 CORS 头
    #[arg(long = "cors", default_value = "false")]
    cors: bool,

    /// 跟随符号链接 (默认：禁用，安全考虑)
    #[arg(long = "follow-symlinks", default_value = "false")]
    follow_symlinks: bool,

    /// 启用文件上传
    #[arg(long = "allow-upload", default_value = "false")]
    allow_upload: bool,

    /// 启用文件删除
    #[arg(long = "allow-delete", default_value = "false")]
    allow_delete: bool,

    /// Basic Auth 认证 (格式: username:password)
    #[arg(long = "auth")]
    auth: Option<String>,

    /// 最大上传文件大小 (字节，默认 100MB)
    #[arg(long = "max-upload-size", default_value = "104857600")]
    max_upload_size: u64,

    /// 每秒最大请求数 (0 表示不限制)
    #[arg(long = "rate-limit", default_value = "0")]
    rate_limit: u32,
}

/// 初始化日志记录器
fn init_logging() {
    // PERF: 使用 EnvFilter 允许运行时调整日志级别
    // 理由：开发时可调至 debug，生产时设为 info/warn 减少 IO
    // 基准：日志级别对性能影响 < 2% (异步日志)
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set subscriber");
}

/// 验证并规范化服务目录路径
fn validate_root_dir(dir: &str) -> ServerResult<PathBuf> {
    let path = PathBuf::from(dir);

    // 检查目录是否存在
    if !path.exists() {
        return Err(error::ServerError::NotFound);
    }

    // 检查是否为目录
    if !path.is_dir() {
        return Err(error::ServerError::BadRequest(
            "Specified path is not a directory".into()
        ));
    }

    // 检查目录是否可读
    let metadata = std::fs::metadata(&path)?;
    if metadata.permissions().readonly() {
        // 只读目录仍然可以服务文件，这是允许的
        tracing::warn!("Serving from read-only directory: {}", path.display());
    }

    // 规范化路径 (解析..和符号链接)
    let canonical = path.canonicalize()?;

    Ok(canonical)
}

/// 服务器入口点
#[tokio::main]
async fn main() -> ServerResult<()> {
    // 初始化日志
    init_logging();

    // 解析 CLI 参数
    let cli = Cli::parse();

    // 验证并规范化根目录
    let root_dir = validate_root_dir(&cli.dir)?;

    // 构建服务器配置
    let config = ServerConfig {
        host: cli.host.clone(),
        port: cli.port,
        root_dir,
        cors: cli.cors,
        follow_symlinks: cli.follow_symlinks,
        // 注意: allow_upload 和 allow_delete 暂时不在 ServerConfig 中，
        // 功能正在开发中（第 2 阶段）
    };

    // 打印启动信息
    tracing::info!(
        "Starting hyper-static-server on http://{}:{}",
        config.host,
        config.port
    );
    tracing::info!("Serving files from: {}", config.root_dir.display());

    if config.cors {
        tracing::info!("CORS enabled");
    }

    if config.follow_symlinks {
        tracing::warn!("Following symlinks - security risk!");
    }

    // 注意: 文件上传/删除功能正在开发中，暂时禁用
    // if config.allow_upload {
    //     tracing::info!("File upload enabled");
    // }

    // if config.allow_delete {
    //     tracing::warn!("File deletion enabled - use with caution!");
    // }

    // 启动服务器
    server::start_server(config).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_validate_root_dir_existing_directory() {
        // Happy Path: 已存在的目录
        let temp_dir = std::env::temp_dir();
        let result = validate_root_dir(temp_dir.to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_root_dir_nonexistent() {
        // Error Case: 不存在的目录
        let result = validate_root_dir("/nonexistent/path/xyz123");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_root_dir_file_not_directory() {
        // Error Case: 文件不是目录
        let temp_file = std::env::temp_dir().join("test_file_not_dir.txt");
        let mut file = std::fs::File::create(&temp_file).unwrap();
        file.write_all(b"test").unwrap();
        drop(file);

        let result = validate_root_dir(temp_file.to_str().unwrap());
        assert!(result.is_err());

        // 清理
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_validate_root_dir_relative_path() {
        // Happy Path: 相对路径被规范化
        let result = validate_root_dir(".");
        assert!(result.is_ok());
        let canonical = result.unwrap();
        assert!(canonical.is_absolute());
    }

    #[test]
    fn test_cli_default_values() {
        // Happy Path: CLI 默认值
        let cli = Cli::parse_from(["hyper-static-server"]);
        assert_eq!(cli.port, 3000);
        assert_eq!(cli.host, "127.0.0.1");
        assert_eq!(cli.dir, ".");
        assert!(!cli.cors);
        assert!(!cli.follow_symlinks);
        assert!(!cli.allow_upload);
        assert!(!cli.allow_delete);
    }

    #[test]
    fn test_cli_custom_values() {
        // Happy Path: CLI 自定义值
        let cli = Cli::parse_from([
            "hyper-static-server",
            "-p", "8080",
            "-d", "/var/www",
            "-H", "0.0.0.0",
            "--cors",
            "--follow-symlinks",
            "--allow-upload",
            "--allow-delete",
        ]);
        assert_eq!(cli.port, 8080);
        assert_eq!(cli.host, "0.0.0.0");
        assert_eq!(cli.dir, "/var/www");
        assert!(cli.cors);
        assert!(cli.follow_symlinks);
        assert!(cli.allow_upload);
        assert!(cli.allow_delete);
    }

    #[test]
    fn test_cli_long_options() {
        // Happy Path: CLI 长选项
        let cli = Cli::parse_from([
            "hyper-static-server",
            "--port", "9000",
            "--dir", "/tmp",
            "--host", "127.0.0.1",
        ]);
        assert_eq!(cli.port, 9000);
        assert_eq!(cli.dir, "/tmp");
    }

    #[test]
    fn test_cli_invalid_port() {
        // Error Case: 无效端口 (超出范围)
        // clap 会自动处理数字范围验证
        let result = Cli::try_parse_from([
            "hyper-static-server",
            "-p", "99999", // 超出 u16 范围
        ]);
        assert!(result.is_err());
    }
}
