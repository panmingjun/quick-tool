//! Quick Tool 客户端
//!
//! 基于 Slint 的跨平台快捷工具客户端

mod app;
mod config;
mod hotkey;
mod ui;
mod window;

use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Quick Tool 客户端启动参数
#[derive(Parser)]
#[command(name = "qt-client")]
#[command(about = "Quick Tool 客户端", long_about = None)]
struct Cli {
    /// 离线启动模式（不连接服务端，数据暂存退出后同步）
    #[arg(short, long)]
    offline: bool,

    /// 配置文件路径（默认 ~/.config/quicktool/config.json）
    #[arg(short, long)]
    config: Option<std::path::PathBuf>,
}

fn main() {
    // 初始化日志（默认 info 级别）
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    let config_path = cli.config.unwrap_or_else(config::default_config_path);

    if cli.offline {
        tracing::info!("Quick Tool 客户端离线启动，配置文件: {}", config_path.display());
        if let Err(e) = app::run_with_options(app::AppOptions {
            offline: true,
            debug_plugin: None,
            config_path,
        }) {
            tracing::error!("应用启动失败: {}", e);
            std::process::exit(1);
        }
    } else {
        tracing::info!("Quick Tool 客户端启动，配置文件: {}", config_path.display());
        if let Err(e) = app::run(app::AppOptions {
            offline: false,
            debug_plugin: None,
            config_path,
        }) {
            tracing::error!("应用启动失败: {}", e);
            std::process::exit(1);
        }
    }
}