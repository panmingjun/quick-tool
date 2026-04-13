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
}

fn main() {
    // 初始化日志（默认 info 级别）
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    if cli.offline {
        tracing::info!("Quick Tool 客户端离线启动");
        app::run_with_options(app::AppOptions {
            offline: true,
            debug_plugin: None,
        });
    } else {
        tracing::info!("Quick Tool 客户端启动");
        app::run();
    }
}