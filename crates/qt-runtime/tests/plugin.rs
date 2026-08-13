//! 核心插件机制集成测试
//!
//! 验证：宿主加载 WASM 组件 → 转换 component → 实例化 → 轮询调用
//! get-ui / get-state → 读取模板与数据快照。
//! 依赖已通过 `cargo build -p demo-plugin --target wasm32-unknown-unknown --release` 构建的产物。

use qt_runtime::engine::WasmEngine;
use qt_runtime::local::discover_plugins;
use qt_runtime::plugin::Value;
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

#[test]
fn plugin_counter_increments_via_state_driven_vo() -> qt_core::Result<()> {
    let wasm = std::fs::read(demo_plugin_wasm()?)
        .map_err(|e| qt_core::Error::WasmRuntime(format!("读取插件失败: {e}")))?;

    let engine = WasmEngine::new()?;
    let mut plugin = engine.instantiate_plugin(&wasm)?;

    // 模板：单一稳定结构，含按钮回调与可驱动属性 count
    let ui = plugin.get_ui()?;
    assert!(ui.contains("CounterUI"), "模板应为 Slint 组件定义: {ui}");
    assert!(ui.contains("button-clicked"), "模板应包含按钮回调: {ui}");
    assert!(ui.contains("count"), "模板应声明 count 属性: {ui}");
    assert_ui_compiles(&ui, true, "count");

    // 初始数据快照：count 为 0
    let state = plugin.get_state()?;
    assert_eq!(read_count(&state)?, 0.0, "初始 count 应为 0");

    // 点击按钮 → 计数 +1 → 快照为 1
    assert!(plugin.dispatch_action("button-clicked")?, "点击应触发状态变更");
    let state_after = plugin.get_state()?;
    assert_eq!(read_count(&state_after)?, 1.0, "点击一次后 count 应为 1");

    // 再次点击 → 计数 +1 → 快照为 2
    assert!(plugin.dispatch_action("button-clicked")?, "点击应触发状态变更");
    let state_after2 = plugin.get_state()?;
    assert_eq!(read_count(&state_after2)?, 2.0, "点击两次后 count 应为 2");

    // 模板稳定：点击前后 get-ui 返回相同模板（不重建）
    let ui_after = plugin.get_ui()?;
    assert_eq!(ui, ui_after, "点击前后模板应保持稳定（不重建）");

    // 轮询语义：状态不变时各 tick 返回相同数据快照
    let state_again = plugin.get_state()?;
    assert_eq!(read_count(&state_again)?, 2.0, "状态不变时快照应保持相同");

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
