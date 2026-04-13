//! HTTP 请求 API

/// HTTP 请求接口
pub trait HttpApi {
    /// 发送 GET 请求
    fn get(&self, url: &str) -> qt_core::Result<Vec<u8>>;

    /// 发送 POST 请求
    fn post(&self, url: &str, body: &[u8]) -> qt_core::Result<Vec<u8>>;
}