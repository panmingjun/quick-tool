//! Slint 应用主模块

use crate::config::offline::OfflineState;
use crate::hotkey::{create_manager, HotkeyManager};
use crate::config::hotkey::Hotkey;
use std::sync::{Arc, Mutex};
use std::thread;

slint::include_modules!();

/// 应用启动选项
pub struct AppOptions {
    /// 是否离线模式
    pub offline: bool,
    /// 调试插件 ID（用于工具独立调试）
    pub debug_plugin: Option<String>,
}

/// 全局应用状态
#[derive(Clone)]
pub struct AppState {
    /// 离线模式状态
    pub offline_state: OfflineState,
    /// 调试插件 ID
    pub debug_plugin: Option<String>,
    /// 窗口是否可见
    pub visible: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            offline_state: OfflineState::default(),
            debug_plugin: None,
            visible: true,
        }
    }
}

/// 运行客户端应用（默认配置）
pub fn run() {
    run_with_options(AppOptions {
        offline: false,
        debug_plugin: None,
    });
}

/// 运行客户端应用（自定义配置）
pub fn run_with_options(options: AppOptions) {
    let mut state = AppState::default();

    if options.offline {
        state.offline_state.enter_offline();
        tracing::info!("客户端以离线模式运行");
    }

    state.debug_plugin = options.debug_plugin;

    // 共享状态用于快捷键线程和UI线程通信
    let state_arc = Arc::new(Mutex::new(state));

    // 创建主窗口
    let app = MainWindow::new().unwrap();

    // 设置初始窗口状态
    let weak_app = app.as_weak();

    // 设置离线模式状态显示
    {
        let state = state_arc.lock().unwrap();
        if state.offline_state.is_offline() {
            app.set_user_status(slint::SharedString::from("离线模式"));
        }
    }

    // ESC键隐藏窗口回调
    app.on_escape_pressed({
        let weak_app = weak_app.clone();
        let state_arc_clone = state_arc.clone();
        move || {
            tracing::info!("ESC键按下，隐藏窗口");
            if let Some(app) = weak_app.upgrade() {
                // 隐藏窗口
                app.window().hide();
                let mut state = state_arc_clone.lock().unwrap();
                state.visible = false;
            }
        }
    });

    // 启动全局快捷键监听线程
    start_hotkey_thread(weak_app.clone(), state_arc.clone());

    // 设置初始工具列表（测试数据）
    app.set_tool_names([
        slint::SharedString::from("Markdown 记事本"),
        slint::SharedString::from("密码管理器"),
        slint::SharedString::from("系统设置"),
    ].into());

    // 运行事件循环
    app.run().unwrap();

    // 退出时处理离线数据同步
    let state = state_arc.lock().unwrap();
    if state.offline_state.is_offline() {
        let pending = state.offline_state.exit_offline();
        if !pending.data_changes.is_empty() || !pending.tool_configs.is_empty() {
            tracing::info!("退出离线模式，有待同步的数据: {} 条数据变更, {} 条配置变更",
                pending.data_changes.len(), pending.tool_configs.len());
            // TODO: 实现自动同步逻辑
        }
    }
}

/// 启动全局快捷键监听线程
fn start_hotkey_thread(weak_app: slint::Weak<MainWindow>, state_arc: Arc<Mutex<AppState>>) {
    thread::spawn(move || {
        let mut manager = create_manager();

        // 注册唤起快捷键 Command+Space
        let toggle_hotkey = Hotkey::command_space();
        if let Err(e) = manager.register(&toggle_hotkey) {
            tracing::error!("注册快捷键失败: {}", e);
            return;
        }

        tracing::info!("全局快捷键已注册: Command+Space");

        // 监听快捷键事件
        let receiver = manager.listen();

        while let Ok(_event) = receiver.recv() {
            tracing::info!("快捷键触发: Command+Space");

            // 切换窗口显示状态
            let visible = {
                let mut state = state_arc.lock().unwrap();
                state.visible = !state.visible;
                state.visible
            };

            // 使用 slint invoke_from_event_loop 在主线程中操作窗口
            slint::invoke_from_event_loop(move || {
                if let Some(app) = weak_app.upgrade() {
                    if visible {
                        tracing::info!("显示窗口");
                        app.window().show();
                    } else {
                        tracing::info!("隐藏窗口");
                        app.window().hide();
                    }
                }
            });
        }
    });
}