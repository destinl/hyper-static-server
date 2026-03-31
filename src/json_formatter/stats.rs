/// JSON 统计和分析功能
///
/// 提供数组计数、字段计数、性能分析等功能

use serde_json::Value;
use super::format::format_json;

/// 统计 JSON 字符串中所有的数组个数
///
/// # Arguments
/// * `input` - 输入的 JSON 字符串
///
/// # Returns
/// 数组总数（包括嵌套数组）
///
/// # Examples
/// ```
/// use hyper_static_server::json_formatter::count_arrays;
/// let count = count_arrays(r#"{"items":[1,2,3],"tags":["a","b"]}"#);
/// assert_eq!(count, 2);
/// ```
///
/// # Note
/// 虽然在后端直接使用中可能不会调用这个函数，
/// 但它提供了公共 API 供测试和前端集成使用
#[allow(dead_code)]
pub fn count_arrays(input: &str) -> usize {
    let result = format_json(input);
    if result.is_valid {
        result.array_stats.len()
    } else {
        0
    }
}

/// 获取顶层 JSON 对象的字段数
///
/// # Arguments
/// * `input` - 输入的 JSON 字符串
///
/// # Returns
/// 顶层字段数（仅计数第一层）
///
/// # Examples
/// ```
/// use hyper_static_server::json_formatter::get_field_count;
/// let count = get_field_count(r#"{"a":1,"b":2,"c":3}"#);
/// assert_eq!(count, 3);
/// ```
///
/// # Note
/// 虽然在后端直接使用中可能不会调用这个函数，
/// 但它提供了公共 API 供测试和前端集成使用
#[allow(dead_code)]
pub fn get_field_count(input: &str) -> usize {
    if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(input) {
        return map.len();
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_json_with_arrays() {
        let input = r#"{"items":[1,2,3],"name":"test"}"#;
        let array_count = count_arrays(input);
        assert_eq!(array_count, 1);
    }

    #[test]
    fn test_count_arrays_single_level() {
        let input = r#"{"arr1":[],"arr2":[],"obj":{"arr3":[]}}"#;
        let count = count_arrays(input);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_count_arrays_zero() {
        let input = r#"{"a":1,"b":"text","c":null}"#;
        let count = count_arrays(input);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_count_arrays_nested() {
        let input = r#"{"matrix":[[1,2],[3,4]]}"#;
        let count = count_arrays(input);
        // 应该计数所有的 [ ] 对：外层1个 + 内层2个 = 3个
        assert_eq!(count, 3);
    }

    #[test]
    fn test_get_field_count() {
        let input = r#"{"a":1,"b":2,"c":3}"#;
        let count = get_field_count(input);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_get_field_count_nested() {
        let input = r#"{"outer":{"inner":1},"top":2}"#;
        let count = get_field_count(input);
        assert_eq!(count, 2);
    }
}
