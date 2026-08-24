//! 核心插件机制集成测试
//!
//! 验证：宿主加载 WASM 组件 → 转换 component → 实例化 → 事件驱动调用
//! get-ui / get-state / dispatch-action → 模板、数据快照与 storage import
//! （每插件独立 SQLite 键值存储）的完整链路。
//! 依赖已通过 `cargo build -p demo-plugin --target wasm32-unknown-unknown --release` 构建的产物。

use qt_runtime::engine::WasmEngine;
use qt_runtime::local::discover_plugins;
use qt_runtime::plugin::{
    custom_event, system_event, PluginHostState, Value, WasmPlugin, EVENT_OPENED, EVENT_TICK,
};
use std::path::PathBuf;

/// 验证插件返回的模板可被宿主的 slint-interpreter 成功编译，且含指定驱动属性
fn assert_ui_compiles(source: &str, expect_callback: bool, expect_property: &str) {
    let compiler = slint_interpreter::Compiler::new();
    let result = spin_on::spin_on(compiler.build_from_source(
        source.to_string(),
        PathBuf::from("plugin.slint"),
    ));
    let diagnostics: Vec<_> = result.diagnostics().collect();
    assert!(!result.has_errors(), "模板编译失败: {diagnostics:?}");
    let definition = result
        .components()
        .next();
    #[expect(clippy::panic, reason = "测试断言：无组件定义即失败")]
    let Some(definition) = definition else {
        panic!("模板应定义组件");
    };
    let has_callback = definition.callbacks().any(|c| c == "button-clicked");
    assert_eq!(has_callback, expect_callback, "回调注册状态不一致");
    let has_property = definition.properties().any(|(n, _)| n == expect_property);
    assert!(has_property, "模板应声明可驱动属性 {expect_property}");
}

/// 定位测试插件 WASM 产物（workspace 根 target 目录）
fn demo_plugin_wasm() -> qt_core::Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir.join("../../target/wasm32-unknown-unknown/release/demo_plugin.wasm");
    Ok(path)
}

/// 为单个用例生成独立的临时数据根目录
fn temp_data_root(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("qt-plugin-test-{tag}-{}", std::process::id()))
}

/// 在指定数据根目录下实例化插件（带独立键值存储）
fn instantiate_at(engine: &WasmEngine, data_root: &std::path::Path) -> qt_core::Result<WasmPlugin> {
    let wasm = std::fs::read(demo_plugin_wasm()?)
        .map_err(|e| qt_core::Error::WasmRuntime(format!("读取插件失败: {e}")))?;
    let host_state = PluginHostState::with_storage(data_root, "demo-plugin")?;
    engine.instantiate_plugin(&wasm, host_state)
}

/// 在独立的临时目录中实例化插件（带独立键值存储），返回实例与数据根目录
fn instantiate_with_storage(
    engine: &WasmEngine,
    tag: &str,
) -> qt_core::Result<(WasmPlugin, PathBuf)> {
    let data_root = temp_data_root(tag);
    Ok((instantiate_at(engine, &data_root)?, data_root))
}

/// 从数据快照中读取指定属性的数值
fn read_count(state: &[qt_runtime::plugin::Property]) -> qt_core::Result<f64> {
    let prop = state
        .iter()
        .find(|p| p.name == "count")
        .ok_or_else(|| qt_core::Error::WasmRuntime("数据快照缺少 count 属性".to_string()))?;
    match prop.value {
        Value::Numeric(n) => Ok(n),
        ref v => Err(qt_core::Error::WasmRuntime(format!(
            "count 属性应为 numeric，实际为 {v:?}"
        ))),
    }
}

/// 从数据快照中读取指定属性的布尔值
fn read_bool(state: &[qt_runtime::plugin::Property], name: &str) -> qt_core::Result<bool> {
    let prop = state
        .iter()
        .find(|p| p.name == name)
        .ok_or_else(|| qt_core::Error::WasmRuntime(format!("数据快照缺少 {name} 属性")))?;
    match prop.value {
        Value::Boolean(b) => Ok(b),
        ref v => Err(qt_core::Error::WasmRuntime(format!(
            "{name} 属性应为 boolean，实际为 {v:?}"
        ))),
    }
}

/// 从数据快照中读取指定属性的文本值
fn read_text(state: &[qt_runtime::plugin::Property], name: &str) -> qt_core::Result<String> {
    let prop = state
        .iter()
        .find(|p| p.name == name)
        .ok_or_else(|| qt_core::Error::WasmRuntime(format!("数据快照缺少 {name} 属性")))?;
    match prop.value {
        Value::Text(ref t) => Ok(t.clone()),
        ref v => Err(qt_core::Error::WasmRuntime(format!(
            "{name} 属性应为 text，实际为 {v:?}"
        ))),
    }
}

#[test]
fn plugin_counter_increments_via_state_driven_vo() -> qt_core::Result<()> {
    let engine = WasmEngine::new()?;
    let (mut plugin, _data_root) = instantiate_with_storage(&engine, "counter")?;

    // 模板：单一稳定结构，含按钮回调与可驱动属性 count / popup-visible
    let ui = plugin.get_ui()?;
    assert!(ui.contains("CounterUI"), "模板应为 Slint 组件定义: {ui}");
    assert!(ui.contains("button-clicked"), "模板应包含按钮回调: {ui}");
    assert!(ui.contains("open-popup"), "模板应包含打开弹窗回调: {ui}");
    assert!(ui.contains("close-popup"), "模板应包含关闭弹窗回调: {ui}");
    assert!(ui.contains("popup-visible"), "模板应声明弹窗显示属性: {ui}");
    assert!(ui.contains("count"), "模板应声明 count 属性: {ui}");
    assert_ui_compiles(&ui, true, "count");
    assert_ui_compiles(&ui, true, "popup-visible");

    // 初始数据快照：count 为 0，弹窗未显示
    let state = plugin.get_state()?;
    assert_eq!(read_count(&state)?, 0.0, "初始 count 应为 0");
    assert_eq!(
        read_bool(&state, "popup-visible")?,
        false,
        "初始弹窗应为隐藏"
    );

    // 点击按钮 → 计数 +1 → 快照为 1
    assert!(plugin.dispatch_event(&custom_event("button-clicked"))?, "点击应触发状态变更");
    let state_after = plugin.get_state()?;
    assert_eq!(read_count(&state_after)?, 1.0, "点击一次后 count 应为 1");

    // 再次点击 → 计数 +1 → 快照为 2
    assert!(plugin.dispatch_event(&custom_event("button-clicked"))?, "点击应触发状态变更");
    let state_after2 = plugin.get_state()?;
    assert_eq!(read_count(&state_after2)?, 2.0, "点击两次后 count 应为 2");

    // 打开弹窗 → 快照 popup-visible 为 true
    assert!(plugin.dispatch_event(&custom_event("open-popup"))?, "打开弹窗应触发状态变更");
    let state_popup = plugin.get_state()?;
    assert_eq!(
        read_bool(&state_popup, "popup-visible")?,
        true,
        "打开弹窗后 popup-visible 应为 true"
    );

    // 关闭弹窗 → 快照 popup-visible 恢复 false
    assert!(plugin.dispatch_event(&custom_event("close-popup"))?, "关闭弹窗应触发状态变更");
    let state_closed = plugin.get_state()?;
    assert_eq!(
        read_bool(&state_closed, "popup-visible")?,
        false,
        "关闭弹窗后 popup-visible 应为 false"
    );

    // 模板稳定：交互前后 get-ui 返回相同模板（不重建）
    let ui_after = plugin.get_ui()?;
    assert_eq!(ui, ui_after, "交互前后模板应保持稳定（不重建）");

    // 快照语义：状态不变时各次拉取返回相同数据
    let state_again = plugin.get_state()?;
    assert_eq!(read_count(&state_again)?, 2.0, "状态不变时快照应保持相同");
    assert_eq!(
        read_bool(&state_again, "popup-visible")?,
        false,
        "状态不变时弹窗状态应保持相同"
    );

    Ok(())
}

#[test]
fn plugin_system_events_are_distinguished_from_custom() -> qt_core::Result<()> {
    let engine = WasmEngine::new()?;
    let (mut plugin, _data_root) = instantiate_with_storage(&engine, "events")?;

    // 系统事件 opened：进入插件后由宿主发送一次；demo 仅回显不改业务状态
    let changed = plugin.dispatch_event(&system_event(EVENT_OPENED, None))?;
    assert!(!changed, "opened 不应改变业务状态");
    assert_eq!(
        read_text(&plugin.get_state()?, "event-log")?,
        "[系统] opened",
        "opened 事件应被回显并标注系统来源"
    );

    // 系统事件 tick：每秒节拍，payload 携带数字时间戳（Unix 秒）
    let timestamp = "1_779_657_600".replace('_', ""); // 2026-05-25 00:00:00 UTC
    plugin.dispatch_event(&system_event(EVENT_TICK, Some(timestamp)))?;
    assert_eq!(
        read_text(&plugin.get_state()?, "event-log")?,
        "[系统] tick",
        "tick 事件应覆盖回显"
    );
    assert_eq!(
        read_text(&plugin.get_state()?, "time-text")?,
        "2026-05-24 21:20:00 UTC",
        "tick 时间戳应被格式化为年月日时分秒"
    );

    // 自定义 UI 事件：来源标注为 UI，且驱动计数变化
    plugin.dispatch_event(&custom_event("button-clicked"))?;
    let state = plugin.get_state()?;
    assert_eq!(
        read_text(&state, "event-log")?,
        "[UI] button-clicked",
        "自定义事件应标注 UI 来源"
    );
    assert_eq!(read_count(&state)?, 1.0, "UI 事件应驱动计数");

    Ok(())
}

#[test]
fn plugin_window_resize_request_via_import() -> qt_core::Result<()> {
    let engine = WasmEngine::new()?;
    let (mut plugin, _data_root) = instantiate_with_storage(&engine, "resize")?;

    // 无请求时取走返回 None
    assert_eq!(plugin.take_window_request(), None, "初始不应有窗口请求");

    // UI 事件触发 window.set-size import → 宿主侧可取走尺寸请求
    plugin.dispatch_event(&custom_event("enlarge-window"))?;
    assert_eq!(
        plugin.take_window_request(),
        Some((720, 560)),
        "放大窗口事件应产生 720x560 尺寸请求"
    );
    assert_eq!(
        plugin.take_window_request(),
        None,
        "取走后应清空（一次性消费）"
    );

    plugin.dispatch_event(&custom_event("restore-window"))?;
    assert_eq!(
        plugin.take_window_request(),
        Some((520, 420)),
        "还原窗口事件应产生 520x420 尺寸请求"
    );

    Ok(())
}

#[test]
fn plugin_kv_storage_persists_across_instances() -> qt_core::Result<()> {
    let engine = WasmEngine::new()?;

    // 实例一：计数两次后保存 count-2 到独立 SQLite 库，随后释放（模拟退出）
    let saved_value;
    {
        let (mut plugin, data_root) = instantiate_with_storage(&engine, "persist-a")?;
        plugin.dispatch_event(&custom_event("button-clicked"))?;
        plugin.dispatch_event(&custom_event("button-clicked"))?;
        assert_eq!(read_count(&plugin.get_state()?)?, 2.0);

        assert!(
            plugin.dispatch_event(&custom_event("save-to-storage"))?,
            "保存动作应触发状态变更"
        );
        let msg = read_text(&plugin.get_state()?, "storage-message")?;
        assert_eq!(msg, "已保存: count-2", "保存回显不符");
        saved_value = "count-2".to_string();
        // drop(plugin)：模拟退出插件窗口，WASM 实例与 Store 释放
        drop(plugin);
        assert!(data_root.join("demo-plugin").join("data.sqlite").is_file());
    }

    // 实例二（全新 Store）：同一数据目录下从库读取，验证退出后数据仍在
    {
        let data_root = temp_data_root("persist-a");
        let mut plugin2 = instantiate_at(&engine, &data_root)?;
        assert!(
            plugin2.dispatch_event(&custom_event("load-from-storage"))?,
            "读取动作应触发状态变更"
        );
        let msg = read_text(&plugin2.get_state()?, "storage-message")?;
        assert_eq!(msg, format!("已恢复: {saved_value}"), "跨实例读取应命中已存值");
        // 读取应恢复计数状态（数据驱动：宿主即时拉取快照覆盖 UI）
        assert_eq!(
            read_count(&plugin2.get_state()?)?,
            2.0,
            "从存储读取后 count 应恢复为保存值"
        );

        // 清理测试数据目录
        let _ = std::fs::remove_dir_all(&data_root);
    }

    Ok(())
}

#[test]
fn discover_local_plugin_and_resolve_wasm() -> qt_core::Result<()> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let plugins_dir = manifest_dir.join("../../plugins");
    let plugins = discover_plugins(&plugins_dir)?;

    let demo = plugins
        .iter()
        .find(|p| p.manifest.id == "demo-plugin")
        .ok_or_else(|| qt_core::Error::Tool("未发现 demo-plugin".to_string()))?;

    assert_eq!(demo.manifest.engine, "wasm");
    assert_eq!(demo.manifest.name, "Demo 插件");

    let wasm_path = demo.wasm_path()?;
    assert!(wasm_path.is_file(), "插件入口不存在: {}", wasm_path.display());
    Ok(())
}
