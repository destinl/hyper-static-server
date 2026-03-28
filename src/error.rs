/// 统一错误类型定义
///
/// 提供服务器所有可能的错误类型，并映射到对应的 HTTP 状态码。
/// 错误消息经过设计，不会向客户端泄漏文件系统信息。

use thiserror::Error;
use axum::http::StatusCode;

/// 服务器错误类型
#[derive(Debug, Error)]
pub enum ServerError {
    /// 文件或目录不存在
    #[error("File not found")]
    NotFound,

    /// 权限不足或路径遍历尝试
    #[error("Permission denied")]
    PermissionDenied,

    /// 目录遍历攻击尝试
    #[error("Path traversal detected")]
    PathTraversal,

    /// Range 请求格式错误
    #[error("Invalid range request")]
    InvalidRange,

    /// 符号链接逃逸尝试
    #[error("Symlink escape not allowed")]
    SymlinkEscape,

    /// 通用 IO 错误 (内部使用，不泄漏细节)
    #[error("IO error occurred")]
    IoError(String),

    /// 无效请求
    #[error("Bad request: {0}")]
    BadRequest(String),
}

impl From<std::io::Error> for ServerError {
    /// 将 IO 错误转换为服务器错误
    ///
    /// # Safety
    /// 不泄漏具体的 IO 错误信息给客户端，只记录日志
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => ServerError::NotFound,
            std::io::ErrorKind::PermissionDenied => ServerError::PermissionDenied,
            _ => ServerError::IoError(err.to_string()),
        }
    }
}

impl From<ServerError> for StatusCode {
    /// 将服务器错误映射到 HTTP 状态码
    fn from(err: ServerError) -> Self {
        match err {
            ServerError::NotFound => StatusCode::NOT_FOUND,
            ServerError::PermissionDenied | ServerError::PathTraversal | ServerError::SymlinkEscape => {
                StatusCode::FORBIDDEN
            }
            ServerError::InvalidRange | ServerError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ServerError::IoError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// 服务器结果类型别名
pub type ServerResult<T> = Result<T, ServerError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_error_not_found() {
        // Happy Path: NotFound 错误映射到 404
        let err = ServerError::NotFound;
        assert_eq!(StatusCode::from(err), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_server_error_permission_denied() {
        // Happy Path: PermissionDenied 映射到 403
        let err = ServerError::PermissionDenied;
        assert_eq!(StatusCode::from(err), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_server_error_path_traversal() {
        // Security: 路径遍历尝试返回 403
        let err = ServerError::PathTraversal;
        assert_eq!(StatusCode::from(err), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_server_error_symlink_escape() {
        // Security: 符号链接逃逸返回 403
        let err = ServerError::SymlinkEscape;
        assert_eq!(StatusCode::from(err), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_server_error_invalid_range() {
        // Happy Path: InvalidRange 映射到 400
        let err = ServerError::InvalidRange;
        assert_eq!(StatusCode::from(err), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_server_error_io_error() {
        // Happy Path: IoError 映射到 500
        let err = ServerError::IoError("test error".to_string());
        assert_eq!(StatusCode::from(err), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_io_error_conversion_not_found() {
        // Edge Case: IO NotFound 转换为 ServerError::NotFound
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let server_err = ServerError::from(io_err);
        assert!(matches!(server_err, ServerError::NotFound));
    }

    #[test]
    fn test_io_error_conversion_permission_denied() {
        // Edge Case: IO PermissionDenied 转换为 ServerError::PermissionDenied
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let server_err = ServerError::from(io_err);
        assert!(matches!(server_err, ServerError::PermissionDenied));
    }

    #[test]
    fn test_io_error_conversion_other() {
        // Edge Case: 其他 IO 错误转换为 ServerError::IoError
        let io_err = std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "eof");
        let server_err = ServerError::from(io_err);
        assert!(matches!(server_err, ServerError::IoError(_)));
    }

    #[test]
    fn test_error_display_not_leak_path() {
        // Security: 错误消息不泄漏文件路径
        let err = ServerError::NotFound;
        let msg = format!("{}", err);
        assert!(!msg.contains('/'));
        assert!(!msg.contains('\\'));
    }
}
