//! Slint 应用主模块
//!
//! 流程：启动后展示本地已安装插件列表 → 用户选择插件 → 进入插件窗口运行 WASM。

use crate::config::hotkey::Hotkey;
use crate::config::offline::OfflineState;
use crate::hotkey::create_manager;
use qt_runtime::engine::WasmEngine;
use qt_runtime::local::{discover_plugins, LocalPlugin};
use qt_runtime::plugin::WasmPlugin;
use slint::{ComponentFactory, ModelRc, Timer, TimerMode, VecModel};
use slint_interpreter::{ComponentHandle, ComponentInstance, Compiler, Value};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

/// 宏生成的 Slint 绑定代码（含 `ComponentContainer` 动态工厂，clippy 豁免）
#[expect(
    clippy::unwrap_used,
    clippy::panic,
    clippy::todo,
    clippy::indexing_slicing,
    reason = "slint 宏生成代码，非手写"
)]
mod generated {
    slint::include_modules!();
}
pub use generated::*;

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

/// 插件会话：已发现的本地插件 + 当前进入的插件实例
pub struct PluginSession {
    /// 本地插件列表
    pub plugins: Vec<LocalPlugin>,
    /// 当前进入的插件实例
    pub current: Option<WasmPlugin>,
}

/// 加锁并处理毒锁（poison）恢复
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 运行客户端应用（默认配置）
pub fn run() -> qt_core::Result<()> {
    run_with_options(AppOptions {
        offline: false,
        debug_plugin: None,
    })
}

/// 运行客户端应用（自定义配置）
pub fn run_with_options(options: AppOptions) -> qt_core::Result<()> {
    let mut state = AppState::default();

    if options.offline {
        state.offline_state.enter_offline();
        tracing::info!("客户端以离线模式运行");
    }

    state.debug_plugin = options.debug_plugin;
    let state_arc = Arc::new(Mutex::new(state));

    // 扫描本地插件目录
    let plugins = discover_local_plugins();
    tracing::info!("发现本地插件: {} 个", plugins.len());

    let session = Arc::new(Mutex::new(PluginSession {
        plugins,
        current: None,
    }));

    // 创建主窗口（插件列表）与插件窗口
    let main_window = MainWindow::new().map_err(|e| {
        qt_core::Error::Ui(format!("创建主窗口失败: {e}"))
    })?;
    let plugin_window = PluginWindow::new().map_err(|e| {
        qt_core::Error::Ui(format!("创建插件窗口失败: {e}"))
    })?;

    // 30 FPS 轮询定时器（进入插件时启动，返回列表时停止）
    let plugin_timer: Rc<RefCell<Option<Timer>>> = Rc::new(RefCell::new(None));

    // 填充插件列表
    {
        let session_guard = lock(&session);
        let names: Vec<slint::SharedString> = session_guard
            .plugins
            .iter()
            .map(|p| {
                slint::SharedString::from(format!("{} v{}", p.manifest.name, p.manifest.version))
            })
            .collect();
        main_window.set_tool_names(ModelRc::new(VecModel::from(names)));
    }

    // 设置离线模式状态显示
    {
        let state_guard = lock(&state_arc);
        if state_guard.offline_state.is_offline() {
            main_window.set_user_status(slint::SharedString::from("离线模式"));
        }
    }

    // 选中插件 → 进入插件窗口
    main_window.on_tool_selected({
        let weak_main = main_window.as_weak();
        let weak_plugin = plugin_window.as_weak();
        let session = session.clone();
        let plugin_timer = plugin_timer.clone();
        move |index| {
            let index = usize::try_from(index).unwrap_or_default();
            tracing::info!("选中插件，index={}", index);

            let (name, result) = {
                let session_guard = lock(&session);
                match session_guard.plugins.get(index) {
                    Some(plugin) => {
                        let name = plugin.manifest.name.clone();
                        let result = instantiate_plugin(plugin);
                        (name, result)
                    }
                    None => return,
                }
            };

            let Some(pw) = weak_plugin.upgrade() else {
                return;
            };
            pw.set_plugin_name(name.into());
            match result {
                Ok(plugin) => {
                    lock(&session).current = Some(plugin);
                    // 启动 30 FPS 轮询：模板变化时重编译，数据变化时 set_property 更新
                    let timer = Timer::default();
                    let session = session.clone();
                    let weak_plugin = weak_plugin.clone();
                    // 已编译实例的共享槽，供数据更新时 set_property
                    let instance_slot: Rc<RefCell<Option<ComponentInstance>>> =
                        Rc::new(RefCell::new(None));
                    let mut last_ui: Option<String> = None;
                    let mut last_state: Option<Vec<qt_runtime::plugin::Property>> = None;
                    timer.start(
                        TimerMode::Repeated,
                        Duration::from_millis(33),
                        move || {
                            let Some(pw) = weak_plugin.upgrade() else {
                                return;
                            };
                            let (ui, state) = match lock(&session).current.as_mut() {
                                Some(plugin) => {
                                    let ui = match plugin.get_ui() {
                                        Ok(ui) => ui,
                                        Err(e) => {
                                            tracing::error!("轮询插件模板失败: {}", e);
                                            return;
                                        }
                                    };
                                    let state = match plugin.get_state() {
                                        Ok(state) => state,
                                        Err(e) => {
                                            tracing::error!("轮询插件数据失败: {}", e);
                                            return;
                                        }
                                    };
                                    (ui, state)
                                }
                                None => return,
                            };
                            // 模板变化：重新编译渲染（页面级切换）
                            if last_ui.as_ref() != Some(&ui) {
                                tracing::info!("插件模板更新，重新编译");
                                let session = session.clone();
                                let dispatcher = move |action: &str| {
                                    let action = action.to_string();
                                    match lock(&session).current.as_mut() {
                                        Some(plugin) => {
                                            match plugin.dispatch_action(&action) {
                                                Ok(_changed) => {
                                                    tracing::info!("插件收到动作: {}", action)
                                                }
                                                Err(e) => {
                                                    tracing::error!(
                                                        "转发动作 {} 失败: {}",
                                                        action,
                                                        e
                                                    )
                                                }
                                            }
                                        }
                                        None => {
                                            tracing::warn!("无活动插件，忽略动作: {}", action)
                                        }
                                    }
                                };
                                match compile_plugin_ui(&ui, dispatcher, &instance_slot) {
                                    Ok(factory) => {
                                        pw.set_plugin_factory(factory);
                                        last_ui = Some(ui);
                                        // 重建后实例状态为初始值，下一 tick 重新应用数据
                                        *instance_slot.borrow_mut() = None;
                                        last_state = None;
                                    }
                                    Err(e) => tracing::error!("编译插件 UI 失败: {}", e),
                                }
                            }
                            // 数据变化：对已编译实例 set_property 增量更新
                            if !state_eq(last_state.as_ref(), Some(&state)) {
                                if let Some(instance) = instance_slot.borrow().as_ref() {
                                    apply_state(instance, &state);
                                }
                                last_state = Some(state);
                            }
                        },
                    );
                    *plugin_timer.borrow_mut() = Some(timer);
                }
                Err(e) => {
                    tracing::error!("插件加载失败: {}", e);
                }
            }
            let _ = pw.show();
            if let Some(mw) = weak_main.upgrade() {
                let _ = mw.hide();
            }
        }
    });

    // 返回插件列表
    plugin_window.on_back_to_list({
        let weak_main = main_window.as_weak();
        let weak_plugin = plugin_window.as_weak();
        let plugin_timer = plugin_timer.clone();
        let session = session.clone();
        move || {
            tracing::info!("返回插件列表");
            // 停止 30 FPS 轮询并清空渲染容器
            if let Some(timer) = plugin_timer.borrow_mut().take() {
                timer.stop();
            }
            if let Some(pw) = weak_plugin.upgrade() {
                pw.set_plugin_factory(ComponentFactory::default());
            }
            lock(&session).current = None;
            if let Some(mw) = weak_main.upgrade() {
                let _ = mw.show();
            }
            if let Some(pw) = weak_plugin.upgrade() {
                let _ = pw.hide();
            }
        }
    });

    // 启动全局快捷键监听线程（切换主窗口显示）
    start_hotkey_thread(main_window.as_weak(), state_arc.clone());

    // 运行事件循环
    if let Err(e) = main_window.run() {
        tracing::error!("应用运行失败: {}", e);
    }

    // 退出时处理离线数据同步
    let mut state_guard = lock(&state_arc);
    if state_guard.offline_state.is_offline() {
        let pending = state_guard.offline_state.exit_offline();
        if !pending.data_changes.is_empty() || !pending.tool_configs.is_empty() {
            tracing::info!(
                "退出离线模式，有待同步的数据: {} 条数据变更, {} 条配置变更",
                pending.data_changes.len(),
                pending.tool_configs.len()
            );
            // TODO: 实现自动同步逻辑
        }
    }
    Ok(())
}

/// 扫描本地插件目录（项目开发目录 + 用户数据目录）
fn discover_local_plugins() -> Vec<LocalPlugin> {
    let mut dirs = Vec::new();

    // 项目开发目录：<cwd>/plugins
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("plugins"));
    }

    // 用户数据目录：data_dir/plugins
    dirs.push(crate::config::data_dir().join("plugins"));

    let mut plugins = Vec::new();
    for dir in dirs {
        match discover_plugins(&dir) {
            Ok(mut found) => {
                // 按 id 去重，避免重复加载
                for plugin in found.drain(..) {
                    if !plugins
                        .iter()
                        .any(|p: &LocalPlugin| p.manifest.id == plugin.manifest.id)
                    {
                        plugins.push(plugin);
                    }
                }
            }
            Err(e) => tracing::warn!("扫描插件目录 {} 失败: {}", dir.display(), e),
        }
    }

    plugins
}

/// 实例化本地插件
fn instantiate_plugin(plugin: &LocalPlugin) -> qt_core::Result<WasmPlugin> {
    let wasm_path = plugin.wasm_path()?;
    tracing::info!("加载插件 WASM: {}", wasm_path.display());

    let wasm = std::fs::read(&wasm_path)
        .map_err(|e| qt_core::Error::WasmRuntime(format!("读取插件 WASM 失败: {}", e)))?;

    let engine = WasmEngine::new()?;
    engine.instantiate_plugin(&wasm)
}

/// 编译插件返回的 UI 模板为渲染工厂。
///
/// `dispatch` 用于把插件 UI 上声明的回调桥接到插件 `dispatch-action`：
/// 宿主枚举组件公开的回调，将每个回调注册为对 `dispatch` 的转发。
/// `instance_slot` 用于把工厂创建的组件实例暴露给轮询逻辑，以便
/// 数据变化时对其 `set_property` 增量更新（无需重建组件树）。
fn compile_plugin_ui(
    source: &str,
    dispatch: impl Fn(&str) + 'static,
    instance_slot: &Rc<RefCell<Option<ComponentInstance>>>,
) -> qt_core::Result<ComponentFactory> {
    let compiler = Compiler::new();
    let result = spin_on::spin_on(
        compiler.build_from_source(source.to_string(), PathBuf::from("plugin.slint")),
    );

    if result.has_errors() {
        let diagnostics = result
            .diagnostics()
            .map(|d| format!("{:?}: {}", d.level(), d.message()))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(qt_core::Error::WasmRuntime(format!(
            "插件 UI 编译失败:\n{}",
            diagnostics
        )));
    }

    let definition = result
        .components()
        .next()
        .ok_or_else(|| qt_core::Error::WasmRuntime("插件 UI 未定义组件".to_string()))?;

    // 插件 UI 公开的回调名即 dispatch-action 的事件名
    let callbacks: Vec<String> = definition.callbacks().collect();
    let dispatch: Rc<dyn Fn(&str)> = Rc::new(dispatch);
    let instance_slot = instance_slot.clone();

    Ok(ComponentFactory::new(move |ctx| {
        let instance = match definition.create_embedded(ctx) {
            Ok(instance) => instance,
            Err(e) => {
                tracing::error!("创建嵌入组件失败: {}", e);
                return None;
            }
        };

        // 暴露实例给轮询逻辑（弱引用避免循环持有）
        *instance_slot.borrow_mut() = Some(instance.clone_strong());

        for name in &callbacks {
            let cb_name = name.clone();
            let dispatch = dispatch.clone();
            let _ = instance.set_callback(name, move |_args| {
                dispatch(&cb_name);
                Value::Void
            });
        }
        Some(instance)
    }))
}

/// 比较两份插件数据快照是否等价（bindgen 生成的类型不实现 PartialEq）。
///
/// 仅比较属性名与值，忽略顺序差异。
fn state_eq(
    prev: Option<&Vec<qt_runtime::plugin::Property>>,
    next: Option<&Vec<qt_runtime::plugin::Property>>,
) -> bool {
    let (Some(prev), Some(next)) = (prev, next) else {
        return prev.is_some() == next.is_some();
    };
    let mut next_rest: Vec<_> = next.clone();
    for p in prev {
        let idx = next_rest
            .iter()
            .position(|q| state_value_eq(&q.value, &p.value) && q.name == p.name);
        match idx {
            Some(i) => {
                next_rest.remove(i);
            }
            None => return false,
        }
    }
    next_rest.is_empty()
}

/// 比较两个数据值是否等价
fn state_value_eq(
    a: &qt_runtime::plugin::Value,
    b: &qt_runtime::plugin::Value,
) -> bool {
    match (a, b) {
        (qt_runtime::plugin::Value::Boolean(x), qt_runtime::plugin::Value::Boolean(y)) => {
            x == y
        }
        (qt_runtime::plugin::Value::Text(x), qt_runtime::plugin::Value::Text(y)) => x == y,
        (qt_runtime::plugin::Value::Numeric(x), qt_runtime::plugin::Value::Numeric(y)) => x == y,
        _ => false,
    }
}
    /// 将插件数据快照（VO）应用到已编译组件实例。
///
/// 按属性名对组件实例逐个 `set_property`，属性须以 `in-out` / `in`
/// 可见性声明，否则 `set_property` 返回错误并记录日志。
fn apply_state(instance: &ComponentInstance, state: &[qt_runtime::plugin::Property]) {
    for property in state {
        let value = match &property.value {
            qt_runtime::plugin::Value::Boolean(b) => Value::Bool(*b),
            qt_runtime::plugin::Value::Text(t) => Value::String(t.clone().into()),
            qt_runtime::plugin::Value::Numeric(n) => Value::Number(*n),
        };
        if let Err(e) = instance.set_property(&property.name, value) {
            tracing::warn!(
                "设置插件属性 {} 失败（需 in-out/in 可见性）: {:?}",
                property.name,
                e
            );
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
            tracing::info!("快捷键触发");

            // 切换主窗口显示状态
            let visible = {
                let mut state_guard = lock(&state_arc);
                state_guard.visible = !state_guard.visible;
                state_guard.visible
            };

            // 使用 slint invoke_from_event_loop 在主线程中操作窗口
            let weak_app = weak_app.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(app) = weak_app.upgrade() {
                    if visible {
                        tracing::info!("显示窗口");
                        let _ = app.window().show();
                    } else {
                        tracing::info!("隐藏窗口");
                        let _ = app.window().hide();
                    }
                }
            });
        }
    });
}