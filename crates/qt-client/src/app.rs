//! Slint 应用主模块
//!
//! 流程：启动时读取配置文件 → 扫描插件源 → 展示本地已安装插件列表
//! → 用户选择插件 → 进入插件窗口运行 WASM。

use crate::config::hotkey::Hotkey;
use crate::config::offline::OfflineState;
use crate::config::plugin_install_dir;
use crate::hotkey::create_manager;
use qt_runtime::engine::WasmEngine;
use qt_runtime::plugin::{PluginHostState, WasmPlugin};
use qt_runtime::registry::{discover_plugins_from_source, PluginRegistry};
use slint::{ComponentFactory, ModelRc, Timer, TimerMode, VecModel};
use slint_interpreter::{ComponentHandle, ComponentInstance, Compiler, Value};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;

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
    /// 配置文件路径
    pub config_path: PathBuf,
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
    pub plugins: Vec<PluginInfo>,
    /// 当前进入的插件实例
    pub current: Option<WasmPlugin>,
}

/// 统一的插件信息
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// 插件 ID
    pub id: String,
    /// 插件名称
    pub name: String,
    /// 版本号
    pub version: String,
    /// 作者
    pub author: String,
    /// 描述
    pub description: String,
    /// 所属插件源 ID
    pub source_id: String,
    /// WASM 文件路径
    pub wasm_path: PathBuf,
    /// 插件根目录
    pub root_dir: PathBuf,
}

/// 加锁并处理毒锁（poison）恢复
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 运行客户端应用
pub fn run(options: AppOptions) -> qt_core::Result<()> {
    run_with_options(options)
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

    // 加载插件注册表
    let install_root = plugin_install_dir();
    let registry = match PluginRegistry::load(&options.config_path, install_root.clone()) {
        Ok(reg) => {
            tracing::info!("加载插件注册表成功，配置文件: {}", options.config_path.display());
            reg
        }
        Err(e) => {
            tracing::warn!("加载插件注册表失败，使用默认配置: {e}");
            // 回退到默认配置
            let default_config = qt_core::AppConfig::default();
            let registry = PluginRegistry::from_config(default_config, install_root);
            registry
        }
    };

    // 扫描插件源下的插件
    let plugins = discover_plugins_from_registry(&registry)?;
    tracing::info!("发现插件: {} 个", plugins.len());

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

    // 系统事件 tick 定时器槽：进入插件时启动（每秒 tick），返回列表时停止
    let plugin_timer: Rc<RefCell<Option<Timer>>> = Rc::new(RefCell::new(None));

    // 填充插件列表
    {
        let session_guard = lock(&session);
        let names: Vec<slint::SharedString> = session_guard
            .plugins
            .iter()
            .map(|p| {
                slint::SharedString::from(format!("{} v{}", p.name, p.version))
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
                        let name = plugin.name.clone();
                        let result = instantiate_plugin(&plugin);
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
                    // 事件驱动渲染：无定时轮询。
                    // 统一事件链路：Slint 回调 / 系统事件（opened、tick）
                    // → dispatch-event → 即时 get-state → set_property。
                    let weak_plugin = weak_plugin.clone();
                    let ui: Rc<RefCell<PluginUi>> = Rc::new(RefCell::new(PluginUi {
                        instance: None,
                        last_ui: None,
                        handler: None,
                    }));
                    let weak_ui = Rc::downgrade(&ui);
                    // 事件处理器：转发给插件后立即同步 UI（模板 diff 重编译 / 数据 set_property）。
                    // 对 ui 持弱引用避免循环持有（ui.handler → handler → 弱ui）。
                    let handler: PluginEventHandler = Rc::new({
                        let session = session.clone();
                        let weak_plugin = weak_plugin.clone();
                        move |event: qt_runtime::plugin::PluginEvent| {
                            let kind = event.kind.clone();
                            // 1. 转发事件给插件
                            match lock(&session).current.as_mut() {
                                Some(plugin) => {
                                    if let Err(e) = plugin.dispatch_event(&event) {
                                        tracing::error!("转发事件 {} 失败: {}", kind, e);
                                        return;
                                    }
                                    tracing::info!("插件收到事件: {}", kind);
                                }
                                None => {
                                    tracing::warn!("无活动插件，忽略事件: {}", kind);
                                    return;
                                }
                            }
                            // 2. 同步一次 UI
                            let Some(pw) = weak_plugin.upgrade() else {
                                return;
                            };
                            let Some(ui) = weak_ui.upgrade() else {
                                return;
                            };
                            match lock(&session).current.as_mut() {
                                Some(plugin) => {
                                    if let Err(e) = sync_plugin_once(plugin, &pw, &ui) {
                                        tracing::error!("同步插件 UI 失败: {}", e);
                                    }
                                    // 3. 应用插件请求的窗口尺寸（window.set-size import）
                                    if let Some((w, h)) = plugin.take_window_request() {
                                        tracing::info!("插件请求窗口尺寸: {w}x{h}");
                                        pw.window().set_size(slint::WindowSize::Logical(
                                            slint::LogicalSize::new(w as f32, h as f32),
                                        ));
                                    }
                                }
                                None => {}
                            }
                        }
                    });
                    ui.borrow_mut().handler = Some(handler.clone());

                    // 首次渲染：进入时立即同步一次（编译模板 + 应用初始数据快照）
                    if let Some(pw2) = weak_plugin.upgrade() {
                        match lock(&session).current.as_mut() {
                            Some(plugin) => {
                                if let Err(e) = sync_plugin_once(plugin, &pw2, &ui) {
                                    tracing::error!("初始同步插件 UI 失败: {}", e);
                                }
                            }
                            None => {}
                        }
                    }

                    // 系统事件 opened：插件被打开（发送一次）
                    handler(qt_runtime::plugin::system_event(
                        qt_runtime::plugin::EVENT_OPENED,
                        None,
                    ));

                    // 系统事件 tick：插件窗口活跃期间每秒发送一次，
                    // payload 携带数字时间戳（Unix 秒，字符串形式）
                    let tick_handler = handler.clone();
                    let timer = Timer::default();
                    timer.start(
                        TimerMode::Repeated,
                        std::time::Duration::from_secs(1),
                        move || {
                            let unix_secs = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();
                            tick_handler(qt_runtime::plugin::system_event(
                                qt_runtime::plugin::EVENT_TICK,
                                Some(unix_secs.to_string()),
                            ));
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
        let session = session.clone();
        let plugin_timer = plugin_timer.clone();
        move || {
            tracing::info!("返回插件列表");
            // 停止 tick 定时器并清空渲染容器
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

/// 从插件注册表发现插件
fn discover_plugins_from_registry(
    registry: &PluginRegistry,
) -> qt_core::Result<Vec<PluginInfo>> {
    let mut plugins = Vec::new();

    for source in registry.enabled_sources() {
        match &source.kind {
            qt_core::PluginSourceKind::Local => {
                let source_dir = registry.resolve_source_url(source);
                if !source_dir.is_dir() {
                    tracing::warn!("本地插件源目录不存在: {:?}", source_dir);
                    continue;
                }
                let found = discover_plugins_from_source(&source_dir, &source.id)
                    .map_err(|e| qt_core::Error::PluginSource(format!("扫描插件源 {id} 失败: {e}", id = source.id)))?;
                for p in found {
                    plugins.push(PluginInfo {
                        id: p.id,
                        name: p.name,
                        version: p.version,
                        author: p.author,
                        description: p.description,
                        source_id: p.source_id,
                        wasm_path: p.wasm_path,
                        root_dir: p.root_dir,
                    });
                }
            }
            qt_core::PluginSourceKind::Remote => {
                // 从本地安装目录查找已缓存的远程插件
                let cached_dir = registry.resolve_source_url(source);
                if cached_dir.is_dir() {
                    let found = discover_plugins_from_source(&cached_dir, &source.id)
                        .map_err(|e| qt_core::Error::PluginSource(format!("扫描缓存插件源 {id} 失败: {e}", id = source.id)))?;
                    for p in found {
                        plugins.push(PluginInfo {
                            id: p.id,
                            name: p.name,
                            version: p.version,
                            author: p.author,
                            description: p.description,
                            source_id: p.source_id,
                            wasm_path: p.wasm_path,
                            root_dir: p.root_dir,
                        });
                    }
                }
            }
        }
    }

    Ok(plugins)
}

/// 实例化本地插件
///
/// 为插件打开独立键值存储（`data/plugins/<plugin_id>/data.sqlite`），
/// 随实例注入，供插件的 storage import 调用。
fn instantiate_plugin(plugin: &PluginInfo) -> qt_core::Result<WasmPlugin> {
    tracing::info!("加载插件 WASM: {}", plugin.wasm_path.display());

    let wasm = std::fs::read(&plugin.wasm_path)
        .map_err(|e| qt_core::Error::WasmRuntime(format!("读取插件 WASM 失败: {e}")))?;

    let host_state = PluginHostState::with_storage(&plugin_install_dir(), &plugin.id)?;
    let engine = WasmEngine::new()?;
    engine.instantiate_plugin(&wasm, host_state)
}

/// 插件事件处理器：把统一事件（系统事件 + 自定义 UI 事件）转发给插件并即时同步 UI
type PluginEventHandler = Rc<dyn Fn(qt_runtime::plugin::PluginEvent)>;

/// 插件 UI 渲染状态（事件驱动，无定时轮询）。
///
/// 交互回调触发 `sync_plugin_once`：模板变化时重编译并替换工厂，
/// 数据变化时对已编译实例 `set_property` 增量更新（组件树不重建）。
struct PluginUi {
    /// 已编译组件实例（工厂创建时填充）
    instance: Option<ComponentInstance>,
    /// 上次模板源码（用于模板 diff）
    last_ui: Option<String>,
    /// 动作处理器：转发插件交互事件并即时同步 UI。
    ///
    /// 存于此（随窗口 factory 存活），handler 对 ui 持弱引用避免循环持有。
    handler: Option<PluginEventHandler>,
}

/// 编译插件返回的 UI 模板为渲染工厂。
///
/// 宿主枚举组件公开的回调，将每个回调注册为对 `ui.handler` 的转发
/// （即转发动作 + 即时同步）；`ui` 同时用于把工厂创建的组件实例暴露给
/// `sync_plugin_once`，以便数据变化时对其 `set_property` 增量更新。
fn compile_plugin_ui(
    source: &str,
    ui: &Rc<RefCell<PluginUi>>,
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
            "插件 UI 编译失败:\n{diagnostics}"
        )));
    }

    let definition = result
        .components()
        .next()
        .ok_or_else(|| qt_core::Error::WasmRuntime("插件 UI 未定义组件".to_string()))?;

    // 插件 UI 公开的回调名即 dispatch-action 的事件名
    let callbacks: Vec<String> = definition.callbacks().collect();
    let ui = ui.clone();

    Ok(ComponentFactory::new(move |ctx| {
        let instance = match definition.create_embedded(ctx) {
            Ok(instance) => instance,
            Err(e) => {
                tracing::error!("创建嵌入组件失败: {}", e);
                return None;
            }
        };

        // 暴露实例给同步逻辑（弱引用避免循环持有）
        let mut ui_guard = ui.borrow_mut();
        ui_guard.instance = Some(instance.clone_strong());
        let handler = ui_guard.handler.clone();
        drop(ui_guard);
        tracing::info!("插件组件实例已创建");

        let Some(handler) = handler else {
            tracing::warn!("插件组件创建时 handler 未就绪");
            return Some(instance);
        };

        for name in &callbacks {
            let cb_name = name.clone();
            let handler = handler.clone();
            let _ = instance.set_callback(name, move |_args| {
                // UI 上触发的事件默认为自定义事件
                handler(qt_runtime::plugin::custom_event(&cb_name));
                Value::Void
            });
        }
        Some(instance)
    }))
}

/// 事件驱动的即时同步：拉取插件最新模板/数据并应用到 UI。
///
/// 由交互回调调用（无定时轮询）。模板变化时重新编译并替换窗口工厂；
/// 数据快照变化时对已编译实例 `set_property` 增量更新。
/// 重编译时新工厂回调绑定 `ui.handler`，保持同一事件驱动链路。
fn sync_plugin_once(
    plugin: &mut WasmPlugin,
    pw: &PluginWindow,
    ui: &Rc<RefCell<PluginUi>>,
) -> qt_core::Result<()> {
    // 模板 diff：变化则重编译（页面级切换）
    let ui_source = plugin.get_ui()?;
    let mut ui_guard = ui.borrow_mut();
    if ui_guard.last_ui.as_ref() != Some(&ui_source) {
        tracing::info!("插件模板更新，重新编译");
        if ui_guard.handler.is_none() {
            return Err(qt_core::Error::WasmRuntime("插件 handler 未就绪".to_string()));
        }
        let factory = compile_plugin_ui(&ui_source, ui)?;
        pw.set_plugin_factory(factory);
        ui_guard.last_ui = Some(ui_source);
        // 重建后实例状态为初始值，需重新应用数据快照
        ui_guard.instance = None;
    }
    drop(ui_guard);

    // 数据快照 → set_property 增量更新
    let state = plugin.get_state()?;
    match ui.borrow().instance.as_ref() {
        Some(instance) => {
            tracing::info!("同步插件 UI：实例存在，应用 {} 个属性", state.len());
            apply_state(instance, &state);
        }
        None => {
            tracing::warn!("同步插件 UI：实例为空，跳过 set_property");
        }
    }
    Ok(())
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
        match instance.set_property(&property.name, value) {
            Ok(()) => tracing::info!("设置插件属性 {} = {:?}", property.name, property.value),
            Err(e) => tracing::warn!(
                "设置插件属性 {} 失败（需 in-out/in 可见性）: {:?}",
                property.name,
                e
            ),
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
            tracing::error!("注册快捷键失败: {e}");
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
