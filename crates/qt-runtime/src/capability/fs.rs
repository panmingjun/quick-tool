//! 文件系统能力

/// 文件系统能力实现
pub struct FsCapability {
    /// 允许的路径
    allowed_paths: Vec<String>,
}

impl FsCapability {
    pub fn new(allowed_paths: Vec<String>) -> Self {
        Self { allowed_paths }
    }

    /// 检查路径访问权限
    pub fn check_path(&self, path: &str) -> bool {
        self.allowed_paths.iter().any(|allowed| path.starts_with(allowed))
    }
}