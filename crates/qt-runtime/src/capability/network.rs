//! 网络能力

/// 网络能力实现
pub struct NetworkCapability {
    /// 允许的域名
    allowed_domains: Vec<String>,
}

impl NetworkCapability {
    pub fn new(allowed_domains: Vec<String>) -> Self {
        Self { allowed_domains }
    }

    /// 检查域名访问权限
    pub fn check_domain(&self, url: &str) -> bool {
        if self.allowed_domains.is_empty() {
            return true;
        }
        self.allowed_domains.iter().any(|domain| url.contains(domain))
    }
}