//! 核心插件机制测试插件：计数器（数据驱动渲染）
//!
//! 依赖 qt-sdk 的 guest 绑定：实现 `Guest::get_ui` / `Guest::get_state` / `Guest::dispatch_action`。
//! - `get-ui`：宿主获取页面模板（.slint 源码），模板为单一稳定结构，
//!   通过 `in-out property` 声明宿主可写的驱动属性，不随状态变化而重建。
//! - `get-state`：宿主按 30FPS 轮询获取数据快照（VO），宿主据此对
//!   已编译组件实例逐个 `set_property` 更新界面。
//! - `dispatch-action`：宿主转发插件 UI 上的按钮点击事件，插件计数 +1。
//!
//! 本 crate 仅面向 wasm32 目标构建（cdylib 供宿主加载），
//! host 目标编译为空，避免误用 qt-sdk 宿主侧 API。

use qt_sdk::bindings::export;
use qt_sdk::bindings::qt::plugin::types::{Property, Value};
use qt_sdk::bindings::Guest;
use std::sync::atomic::{AtomicUsize, Ordering};

/// 插件状态：计数
static COUNT: AtomicUsize = AtomicUsize::new(0);

/// 插件实现：`get_ui` 返回稳定页面模板，`get_state` 返回数据快照
struct DemoPlugin;

impl Guest for DemoPlugin {
    /// 宿主获取页面模板。模板结构固定，仅当结构变化时宿主才重新编译。
    /// `count` 为 `in-out property`，可由宿主 `set_property` 驱动。
    fn get_ui() -> String {
        r#"
            import { Button } from "std-widgets.slint";
            export component CounterUI inherits Rectangle {
                callback button-clicked;
                in-out property <int> count: 0;
                VerticalLayout {
                    spacing: 16px;
                    Button {
                        text: "点击加一";
                        clicked => { root.button-clicked(); }
                    }
                    Text {
                        text: "Count: \{root.count}";
                        font-size: 32px;
                        horizontal-alignment: center;
                        vertical-alignment: center;
                    }
                }
            }
        "#
        .to_string()
    }

    /// 宿主按固定 tick 轮询的数据快照：驱动 `count` 属性的当前值
    fn get_state() -> Vec<Property> {
        vec![Property {
            name: "count".to_string(),
            value: Value::Numeric(COUNT.load(Ordering::Relaxed) as f64),
        }]
    }

    /// 宿主转发插件 UI 事件，插件据此更新内部状态
    fn dispatch_action(action: String) -> bool {
        match action.as_str() {
            "button-clicked" => {
                COUNT.fetch_add(1, Ordering::Relaxed);
                true
            }
            _ => false,
        }
    }
}

// 声明导出（生成组件元数据），`with_types_in` 指向绑定生成所在的模块
export!(DemoPlugin with_types_in qt_sdk::bindings);
