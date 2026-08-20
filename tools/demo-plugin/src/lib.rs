//! 核心插件机制测试插件：计数器 + 弹窗（数据驱动渲染）
//!
//! 依赖 qt-sdk 的 guest 绑定：实现 `Guest::get_ui` / `Guest::get_state` / `Guest::dispatch_action`。
//! - `get-ui`：宿主获取页面模板（.slint 源码），模板为单一稳定结构，
//!   通过 `in-out property` 声明宿主可写的驱动属性，不随状态变化而重建。
//! - `get-state`：宿主在交互事件后即时获取数据快照（VO），据此对已编译
//!   组件实例逐个 `set_property` 更新界面（事件驱动，无定时轮询）。
//! - `dispatch-action`：宿主转发插件 UI 上的按钮点击事件，插件计数 +1
//!   或切换弹窗显示。
//!
//! 模板一次性声明页面与弹窗：弹窗为 `if root.popup-visible` 控制的自定义
//! 浮层（全屏遮罩 + 居中卡片），宿主仅 `set_property` 驱动 `popup-visible`，
//! 组件树不重建、无闪烁，验证「类 Vue：单模板 + v-if 控制页面/弹窗」。
//! 注：Slint 内建 `PopupWindow` 为命令式 API（`show()`/`close()`），
//! 不支持条件渲染（`PopupWindow cannot be directly repeated or conditional`），
//! 故弹窗用普通矩形浮层实现，以配合宿主数据驱动。
//!
//! 本 crate 仅面向 wasm32 目标构建（cdylib 供宿主加载），
//! host 目标编译为空，避免误用 qt-sdk 宿主侧 API。

#![cfg(target_arch = "wasm32")]

use qt_sdk::bindings::export;
use qt_sdk::bindings::qt::plugin::types::{Property, Value};
use qt_sdk::bindings::Guest;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// 插件状态：计数
static COUNT: AtomicUsize = AtomicUsize::new(0);
/// 插件状态：弹窗是否显示
static POPUP_VISIBLE: AtomicBool = AtomicBool::new(false);

/// 插件实现：`get_ui` 返回稳定页面模板，`get_state` 返回数据快照
struct DemoPlugin;

impl Guest for DemoPlugin {
    /// 宿主获取页面模板。模板结构固定，仅当结构变化时宿主才重新编译。
    /// `count` / `popup-visible` 为 `in-out property`，可由宿主 `set_property` 驱动。
    fn get_ui() -> String {
        r#"
            import { Button } from "std-widgets.slint";
            export component CounterUI inherits Rectangle {
                callback button-clicked;
                callback open-popup;
                callback close-popup;
                in-out property <int> count: 0;
                in-out property <bool> popup-visible: false;
                VerticalLayout {
                    spacing: 16px;
                    Button {
                        text: "点击加一";
                        clicked => { root.button-clicked(); }
                    }
                    Button {
                        text: "打开弹窗";
                        clicked => { root.open-popup(); }
                    }
                    Text {
                        text: "Count: \{root.count}";
                        font-size: 32px;
                        horizontal-alignment: center;
                        vertical-alignment: center;
                    }
                }
                if root.popup-visible: Rectangle {
                    x: 0;
                    y: 0;
                    width: parent.width;
                    height: parent.height;
                    background: transparent;
                    TouchArea {
                        clicked => { root.close-popup(); }
                        Rectangle {
                            width: 300px;
                            height: 180px;
                            x: (parent.width - self.width) / 2;
                            y: (parent.height - self.height) / 2;
                            background: #313244;
                            border-radius: 8px;
                            VerticalLayout {
                                padding: 24px;
                                spacing: 16px;
                                Text {
                                    text: "这是一个弹窗";
                                    font-size: 18px;
                                    color: #cdd6f4;
                                    horizontal-alignment: center;
                                }
                                Button {
                                    text: "关闭弹窗";
                                    clicked => { root.close-popup(); }
                                }
                            }
                        }
                    }
                }
            }
        "#
        .to_string()
    }

    /// 宿主在交互事件后即时拉取的数据快照：驱动 `count` 与 `popup-visible` 属性
    fn get_state() -> Vec<Property> {
        vec![
            Property {
                name: "count".to_string(),
                value: Value::Numeric(COUNT.load(Ordering::Relaxed) as f64),
            },
            Property {
                name: "popup-visible".to_string(),
                value: Value::Boolean(POPUP_VISIBLE.load(Ordering::Relaxed)),
            },
        ]
    }

    /// 宿主转发插件 UI 事件，插件据此更新内部状态
    fn dispatch_action(action: String) -> bool {
        match action.as_str() {
            "button-clicked" => {
                COUNT.fetch_add(1, Ordering::Relaxed);
                true
            }
            "open-popup" => {
                POPUP_VISIBLE.store(true, Ordering::Relaxed);
                true
            }
            "close-popup" => {
                POPUP_VISIBLE.store(false, Ordering::Relaxed);
                true
            }
            _ => false,
        }
    }
}

// 声明导出（生成组件元数据），`with_types_in` 指向绑定生成所在的模块
export!(DemoPlugin with_types_in qt_sdk::bindings);
