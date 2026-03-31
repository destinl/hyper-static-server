/// HTTP 服务器核心模块
///
/// 使用 axum 框架搭建 HTTP 服务，处理路由定义、请求分发和服务器启动。
///
/// # 架构决策
/// - 使用 axum (不是 actix-web): 生态统一，维护稳定
/// - 路由：Trie matching (axum 自动处理)
/// - 静态文件：使用状态层处理

use axum::{
    routing::{get, Router},  // 移除 post 和 delete，除非你确实需要它们
    extract::{Path, State, Request, Multipart},
    response::{Response, IntoResponse, Json},
    http::{Method, StatusCode, header},
};
use serde::Serialize;
use std::{path::{Component, PathBuf}, sync::Arc};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

use crate::error::{ServerError, ServerResult};
use crate::response::{
    self, FileMetadata, RangeValue,
    build_not_modified_response,
    build_full_file_response,
    build_partial_response,
    build_directory_listing,
};
use tower_http::trace::TraceLayer;

/// 为 ServerError 实现 IntoResponse，允许直接从 handler 返回错误
impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ServerError::NotFound => (StatusCode::NOT_FOUND, "Not Found"),
            ServerError::PermissionDenied => (StatusCode::FORBIDDEN, "Permission Denied"),
            ServerError::PathTraversal => (StatusCode::BAD_REQUEST, "Path Traversal Detected"),
            ServerError::InvalidRange => (StatusCode::BAD_REQUEST, "Invalid Range Request"),
            ServerError::SymlinkEscape => (StatusCode::BAD_REQUEST, "Symlink Escape Not Allowed"),
            ServerError::IoError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error"),
            ServerError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.as_str()),
            ServerError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            ServerError::TooManyRequests => (StatusCode::TOO_MANY_REQUESTS, "Too Many Requests"),
            ServerError::PayloadTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "Payload Too Large"),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

/// 服务器配置
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// 绑定主机
    pub host: String,
    /// 监听端口
    pub port: u16,
    /// 服务根目录 (已规范化)
    pub root_dir: PathBuf,
    /// 是否启用 CORS
    pub cors: bool,
    /// 是否跟随符号链接
    pub follow_symlinks: bool,
    /// 是否启用文件上传
    pub allow_upload: bool,
    /// 是否启用文件删除
    pub allow_delete: bool,
}

/// 应用状态 (在所有请求间共享)
#[derive(Debug, Clone)]
pub struct AppState {
    pub config: Arc<ServerConfig>,
}

/// 验证请求路径是否在根目录内 (防止目录遍历攻击)
///
/// # Security
/// 使用 URL 解码、符号链接检查和 root 目录比较来防止越界访问。
fn validate_path_within_root(
    root: &PathBuf,
    requested: &str,
    follow_symlinks: bool,
) -> ServerResult<PathBuf> {
    // 解码 URL 编码的字符
    let decoded = decode_url(requested)?;
    let root_canonical = root
        .canonicalize()
        .map_err(|_| ServerError::IoError("Failed to canonicalize root".into()))?;

    let requested_path = PathBuf::from(&decoded);
    let mut current = root.clone();

    for component in requested_path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => {
                return Err(ServerError::PathTraversal);
            }
            Component::ParentDir => {
                current.pop();
            }
            Component::CurDir => {
                continue;
            }
            Component::Normal(part) => {
                current.push(part);
                if !follow_symlinks {
                    let metadata = std::fs::symlink_metadata(&current)
                        .map_err(|_| ServerError::NotFound)?;
                    if metadata.file_type().is_symlink() {
                        return Err(ServerError::SymlinkEscape);
                    }
                }
            }
        }
    }

    let canonical = current.canonicalize().map_err(|_| ServerError::NotFound)?;
    if !canonical.starts_with(&root_canonical) {
        return Err(ServerError::PathTraversal);
    }

    Ok(canonical)
}

/// URL 解码辅助函数
fn decode_url(s: &str) -> Result<String, ServerError> {
    urlencoding::decode(s)
        .map(|cow| cow.into_owned())
        .map_err(|_| ServerError::BadRequest("Invalid URL encoding".into()))
}

/// 检查并处理 CORS 预检请求
async fn handle_options(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if state.config.cors {
        (
            StatusCode::NO_CONTENT,
            [
                (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
                (header::ACCESS_CONTROL_ALLOW_METHODS, "GET, HEAD, OPTIONS"),
                (header::ACCESS_CONTROL_ALLOW_HEADERS, "Content-Type"),
            ],
        ).into_response()
    } else {
        StatusCode::NO_CONTENT.into_response()
    }
}

/// 添加 CORS 头到响应
fn add_cors_headers(response: Response) -> Response {
    let (mut parts, body) = response.into_parts();
    parts.headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        "*".parse().unwrap(),
    );
    parts.headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        "GET, HEAD, OPTIONS".parse().unwrap(),
    );
    parts.headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        "*".parse().unwrap(),
    );
    Response::from_parts(parts, body)
}

/// 处理静态文件和目录请求
async fn handle_static_file(
    Path(requested_path): Path<String>,
    State(state): State<Arc<AppState>>,
    request: Request,
) -> Result<Response, ServerError> {
    let config = &state.config;

    // 验证路径在根目录内
    let file_path = validate_path_within_root(
        &config.root_dir,
        &requested_path,
        config.follow_symlinks,
    )?;

    // 获取文件元数据
    let metadata = std::fs::metadata(&file_path)?;

    if metadata.is_dir() {
        // 目录请求 - 生成目录列表
        return handle_directory_request(
            &file_path,
            &requested_path,
            config.cors,
            config.allow_upload,
            config.allow_delete,
        );
    }

    // 文件请求 - 处理静态文件 (支持 Range 和缓存)
    handle_file_request(&file_path, config.cors, request).await
}

/// 处理目录列表请求
fn handle_directory_request(
    dir_path: &PathBuf,
    request_path: &str,
    cors: bool,
    allow_upload: bool,
    allow_delete: bool,
) -> Result<Response, ServerError> {
    // 生成目录列表 HTML
    let html = build_directory_listing(dir_path, request_path, allow_upload, allow_delete)?;

    let mut response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CONTENT_LENGTH, html.len().to_string())
        .body(axum::body::Body::from(html))
        .expect("Failed to build directory listing response");

    if cors {
        response = add_cors_headers(response);
    }

    Ok(response)
}

/// 处理文件请求 (支持 Range 和缓存)
async fn handle_file_request(
    file_path: &PathBuf,
    cors: bool,
    request: Request,
) -> Result<Response, ServerError> {
    // 获取文件元数据
    let fs_metadata = std::fs::metadata(file_path)?;
    let metadata = FileMetadata::from_metadata(fs_metadata)?;
    let etag = metadata.generate_etag();

    // 提取请求头
    let headers = request.headers();
    let if_none_match = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok());
    let if_modified_since = headers
        .get(header::IF_MODIFIED_SINCE)
        .and_then(|v| v.to_str().ok());
    let range_header = headers
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok());

    // 检查缓存是否有效 (返回 304)
    if response::is_cache_valid(if_none_match, &etag, if_modified_since, metadata.modified) {
        let last_modified = metadata.generate_last_modified()?;
        let mut response = build_not_modified_response(&etag, &last_modified);
        if cors {
            response = add_cors_headers(response);
        }
        return Ok(response);
    }

    // 检查是否有 Range 请求
    if let Some(range_spec) = range_header {
        match RangeValue::parse(range_spec, metadata.size) {
            Ok(range) => {
                // 有效的 Range 请求，返回 206 Partial Content
                let mut response = build_partial_response(file_path, metadata, range).await?;
                if cors {
                    response = add_cors_headers(response);
                }
                return Ok(response);
            }
            Err(_) => {
                // Range 格式错误，返回 400
                return Err(ServerError::InvalidRange);
            }
        }
    }

    // 普通请求，返回完整文件
    let mut response = build_full_file_response(file_path, metadata).await?;

    if cors {
        response = add_cors_headers(response);
    }

    Ok(response)
}

/// 根路径处理 (重定向到目录列表或 index.html)
async fn handle_root(
    State(state): State<Arc<AppState>>,
    request: Request,
) -> Result<Response, ServerError> {
    // 检查根目录下是否有 index.html
    let index_path = state.config.root_dir.join("index.html");

    if index_path.exists() && index_path.is_file() {
        // 有 index.html，返回它
        handle_file_request(&index_path, state.config.cors, request).await
    } else {
        // 没有 index.html，显示目录列表
        handle_directory_request(
            &state.config.root_dir,
            "/",
            state.config.cors,
            state.config.allow_upload,
            state.config.allow_delete,
        )
    }
}

/// 通配符路径处理 (匹配所有其他路径)
async fn handle_wildcard(
    Path(requested_path): Path<String>,
    State(state): State<Arc<AppState>>,
    request: Request,
) -> Result<Response, ServerError> {
    handle_static_file(Path(requested_path), State(state), request).await
}

/// 上传响应结构
#[derive(Serialize)]
struct UploadResponse {
    success: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    filename: Option<String>,
}

/// 处理文件上传
async fn handle_upload(
    Path(requested_path): Path<String>,
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, ServerError> {
    let config = &state.config;

    // 检查是否允许上传
    if !config.allow_upload {
        return Ok(Json(UploadResponse {
            success: false,
            message: "文件上传已禁用".to_string(),
            filename: None,
        }));
    }

    // 验证上传目录
    let upload_dir = if requested_path.is_empty() || requested_path == "/" {
        config.root_dir.clone()
    } else {
        validate_path_within_root(&config.root_dir, &requested_path, config.follow_symlinks)?
    };

    // 确保目标是目录
    if !upload_dir.is_dir() {
        return Err(ServerError::BadRequest("目标路径不是目录".into()));
    }

    // 处理上传的文件
    let mut uploaded_filename = None;
    
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        ServerError::BadRequest(format!("读取上传数据失败: {}", e))
    })? {
        let filename = field.file_name()
            .ok_or_else(|| ServerError::BadRequest("缺少文件名".into()))?
            .to_string();

        // 安全检查：防止路径遍历
        if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
            return Err(ServerError::BadRequest("无效的文件名".into()));
        }

        let data = field.bytes().await.map_err(|e| {
            ServerError::BadRequest(format!("读取文件数据失败: {}", e))
        })?;

        let file_path = upload_dir.join(&filename);
        
        // 写入文件
        tokio::fs::write(&file_path, &data).await.map_err(|e| {
            ServerError::IoError(format!("写入文件失败: {}", e))
        })?;

        uploaded_filename = Some(filename);
        break; // 只处理第一个文件
    }

    match uploaded_filename {
        Some(name) => Ok(Json(UploadResponse {
            success: true,
            message: "上传成功".to_string(),
            filename: Some(name),
        })),
        None => Err(ServerError::BadRequest("未收到文件".into())),
    }
}

/// 处理根目录上传
async fn handle_root_upload(
    State(state): State<Arc<AppState>>,
    multipart: Multipart,
) -> Result<Json<UploadResponse>, ServerError> {
    handle_upload(Path(String::new()), State(state), multipart).await
}

/// 删除响应结构
#[derive(Serialize)]
struct DeleteResponse {
    success: bool,
    message: String,
}

/// 处理文件删除
async fn handle_delete(
    Path(requested_path): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<DeleteResponse>, ServerError> {
    let config = &state.config;

    // 检查是否允许删除
    if !config.allow_delete {
        return Ok(Json(DeleteResponse {
            success: false,
            message: "文件删除已禁用".to_string(),
        }));
    }

    // 验证路径
    let file_path = validate_path_within_root(&config.root_dir, &requested_path, config.follow_symlinks)?;

    // 检查文件是否存在
    if !file_path.exists() {
        return Err(ServerError::NotFound);
    }

    // 删除文件或目录
    if file_path.is_dir() {
        tokio::fs::remove_dir_all(&file_path).await.map_err(|e| {
            ServerError::IoError(format!("删除目录失败: {}", e))
        })?;
    } else {
        tokio::fs::remove_file(&file_path).await.map_err(|e| {
            ServerError::IoError(format!("删除文件失败: {}", e))
        })?;
    }

    Ok(Json(DeleteResponse {
        success: true,
        message: "删除成功".to_string(),
    }))
}

/// 启动 HTTP 服务器
///
/// # Arguments
/// * `config` - 服务器配置
///
/// # Returns
/// 服务器运行错误 (如果发生)
pub async fn start_server(config: ServerConfig) -> ServerResult<()> {
    // 创建应用状态
    let state = Arc::new(AppState {
        config: Arc::new(config),
    });

    // 定义路由
    // PERF: axum 使用 Trie 路由，O(min(path_len, routes)) 复杂度
    // 理由：比线性匹配快 10x+ 当路由数量大时
    let mut app = Router::new()
        .route(
            "/",
            get(handle_root)
                .head(handle_root)
                .post(handle_root_upload)
                .options(handle_options),
        )
        .route(
            "/*path",
            get(handle_wildcard)
                .head(handle_wildcard)
                .post(handle_upload)
                .delete(handle_delete)
                .options(handle_options),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    if state.config.cors {
        app = app.layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::HEAD, Method::OPTIONS])
                .allow_headers(Any),
        );
    }

    // 绑定地址
    let addr = format!(
        "{}:{}",
        state.config.host,
        state.config.port
    );

    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| ServerError::IoError(e.to_string()))?;

    tracing::info!("Server listening on {}", addr);

    // 启动服务
    axum::serve(listener, app)
        .await
        .map_err(|e| ServerError::IoError(e.to_string()))?;

    Ok(())
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_path_within_root_normal() {
        // Happy Path: 正常路径在根目录内
        let root = std::env::temp_dir();
        let result = validate_path_within_root(&root, ".", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_within_root_subdir() {
        // Happy Path: 子目录路径
        let root = std::env::temp_dir();
        let result = validate_path_within_root(&root, "subdir", false);
        // 可能失败如果子目录不存在，但不应是 PathTraversal
        match result {
            Err(ServerError::PathTraversal) => panic!("Should not be PathTraversal"),
            _ => {}
        }
    }

    #[test]
    fn test_validate_path_traversal_blocked() {
        // Security: 目录遍历被阻止
        let root = std::env::temp_dir();
        let result = validate_path_within_root(&root, "../../etc/passwd", false);

        // 应该被阻止 (PathTraversal 或 NotFound)
        assert!(result.is_err());
        match result.unwrap_err() {
            ServerError::PathTraversal | ServerError::NotFound => {}
            e => panic!("Expected PathTraversal or NotFound, got {:?}", e),
        }
    }

    #[test]
    fn test_decode_url() {
        // Happy Path: 正常解码
        assert_eq!(decode_url("hello%20world").unwrap(), "hello world");
        assert_eq!(decode_url("path%2Fto%2Ffile").unwrap(), "path/to/file");
        assert_eq!(decode_url("test%2Etxt").unwrap(), "test.txt");
        
        // Edge Case: 不需要解码的字符串
        assert_eq!(decode_url("normal").unwrap(), "normal");
        assert_eq!(decode_url("").unwrap(), "");
        
        // Error Case: 无效的百分号编码
        assert!(decode_url("%").is_err());
        assert!(decode_url("%2").is_err());
        assert!(decode_url("%XX").is_err());
        assert!(decode_url("%2X").is_err());
    }

    #[test]
    fn test_server_config_clone() {
        // Happy Path: ServerConfig 可克隆
        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            root_dir: std::env::temp_dir(),
            cors: false,
            follow_symlinks: false,
            allow_upload: false,
            allow_delete: false,
        };
        let _clone = config.clone();
    }

    #[test]
    fn test_app_state_arc() {
        // Happy Path: AppState 可被 Arc 包裹
        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            root_dir: std::env::temp_dir(),
            cors: true,
            follow_symlinks: false,
            allow_upload: true,
            allow_delete: false,
        };
        let _state = Arc::new(AppState {
            config: Arc::new(config),
        });
    }

    #[test]
    fn test_cors_headers_added() {
        // Happy Path: CORS 头被添加
        let response = Response::builder()
            .status(StatusCode::OK)
            .body(axum::body::Body::from("test"))
            .unwrap();

        let response_with_cors = add_cors_headers(response);
        let headers = response_with_cors.headers();

        assert!(headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_some());
        assert!(headers.get(header::ACCESS_CONTROL_ALLOW_METHODS).is_some());
    }

    #[test]
    fn test_directory_listing_generation() {
        // Happy Path: 目录列表生成
        let temp_dir = std::env::temp_dir();
        let result = build_directory_listing(&temp_dir, "/test/", false, false);
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("/test/"));
    }

    #[test]
    fn test_symlink_escape_blocked_without_flag() {
        // Security: 符号链接逃逸在不跟随符号链接时被阻止
        let root = std::env::temp_dir();
        // 创建测试结构
        let test_dir = root.join("hyper_test_symlink");
        let _ = std::fs::create_dir_all(&test_dir);

        // 创建指向外部的符号链接 (Windows 需要管理员权限，可能失败)
        #[cfg(unix)]
        {
            let link_path = test_dir.join("escape_link");
            let _ = std::os::unix::fs::symlink("/etc", &link_path);

            let result = validate_path_within_root(&test_dir, "escape_link/passwd", false);
            assert!(result.is_err());

            let _ = std::fs::remove_dir_all(&test_dir);
        }

        #[cfg(windows)]
        {
            // Windows 符号链接需要特殊处理，跳过
            let _ = std::fs::remove_dir_all(&test_dir);
        }
    }
}
