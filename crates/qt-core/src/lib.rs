//! Quick Tool 核心共享库
//!
//! 提供公共类型定义、错误处理和加密工具

pub mod types;
pub mod error;
pub mod crypto;

pub use types::*;
pub use error::{Error, Result};