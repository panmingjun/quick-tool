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
use qt_sdk::bindings::qt::plugin::storage;
use qt_sdk::bindings::qt::plugin::types::{EventSource, PluginEvent, Property, Value};
use qt_sdk::bindings::qt::plugin::window;
use qt_sdk::bindings::Guest;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex};

/// 插件状态：计数
static COUNT: AtomicUsize = AtomicUsize::new(0);
/// 插件状态：弹窗是否显示
static POPUP_VISIBLE: AtomicBool = AtomicBool::new(false);
/// 插件状态：存储读写回显（保存/读取结果）
static STORAGE_MESSAGE: LazyLock<Mutex<String>> = LazyLock::new(|| Mutex::new(String::new()));
/// 插件状态：最近收到的事件回显（系统事件 / 自定义 UI 事件）
static EVENT_LOG: LazyLock<Mutex<String>> = LazyLock::new(|| Mutex::new(String::new()));
/// 插件状态：tick 时间戳格式化后的展示文本（年月日时分秒）
static TIME_TEXT: LazyLock<Mutex<String>> = LazyLock::new(|| Mutex::new(String::new()));

/// 持久化键：演示用固定 key
const DEMO_KEY: &str = "demo-note";

/// 放大窗口的目标尺寸（逻辑像素）
const ENLARGED_SIZE: (u32, u32) = (720, 560);
/// 还原窗口的目标尺寸（逻辑像素）
const RESTORED_SIZE: (u32, u32) = (520, 420);

/// 把 Unix 秒时间戳格式化为 `YYYY-MM-DD HH:MM:SS`（UTC，纯 std 无依赖）
fn format_timestamp(unix_secs: u64) -> String {
    let days = (unix_secs / 86_400) as i64;
    let rem = unix_secs % 86_400;
    let (hh, mm, ss) = (rem / 3_600, (rem % 3_600) / 60, rem % 60);
    // civil_from_days：1970-01-01 起的天数 → 年月日（Howard Hinnant 算法）
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    format!("{year:04}-{month:02}-{day:02} {hh:02}:{mm:02}:{ss:02} UTC")
}

fn set_storage_message(msg: String) {
    *STORAGE_MESSAGE.lock().unwrap_or_else(|p| p.into_inner()) = msg;
}

fn storage_message() -> String {
    STORAGE_MESSAGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

fn set_event_log(msg: String) {
    *EVENT_LOG.lock().unwrap_or_else(|p| p.into_inner()) = msg;
}

fn event_log() -> String {
    EVENT_LOG.lock().unwrap_or_else(|p| p.into_inner()).clone()
}

fn set_time_text(msg: String) {
    *TIME_TEXT.lock().unwrap_or_else(|p| p.into_inner()) = msg;
}

fn time_text() -> String {
    TIME_TEXT.lock().unwrap_or_else(|p| p.into_inner()).clone()
}

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
                callback save-to-storage;
                callback load-from-storage;
                callback enlarge-window;
                callback restore-window;
                in-out property <int> count: 0;
                in-out property <bool> popup-visible: false;
                in-out property <string> storage-message: "";
                in-out property <string> event-log: "";
                in-out property <string> time-text: "";
                VerticalLayout {
                    spacing: 16px;
                    HorizontalLayout {
                        spacing: 12px;
                        Button {
                            text: "点击加一";
                            clicked => { root.button-clicked(); }
                        }
                        Button {
                            text: "打开弹窗";
                            clicked => { root.open-popup(); }
                        }
                    }
                    HorizontalLayout {
                        spacing: 12px;
                        Button {
                            text: "保存计数到存储";
                            clicked => { root.save-to-storage(); }
                        }
                        Button {
                            text: "从存储读取";
                            clicked => { root.load-from-storage(); }
                        }
                    }
                    HorizontalLayout {
                        spacing: 12px;
                        Button {
                            text: "放大窗口";
                            clicked => { root.enlarge-window(); }
                        }
                        Button {
                            text: "还原窗口";
                            clicked => { root.restore-window(); }
                        }
                    }
                    Text {
                        text: "Count: \{root.count}";
                        font-size: 32px;
                        horizontal-alignment: center;
                        vertical-alignment: center;
                    }
                    Text {
                        text: root.time-text;
                        font-size: 18px;
                        color: #89b4fa;
                        horizontal-alignment: center;
                    }
                    Text {
                        text: root.storage-message;
                        font-size: 14px;
                        color: #a6e3a1;
                        horizontal-alignment: center;
                        wrap: word-wrap;
                    }
                    Text {
                        text: "事件: " + root.event-log;
                        font-size: 12px;
                        color: #f9e2af;
                        horizontal-alignment: center;
                        wrap: word-wrap;
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

    /// 宿主在事件后即时拉取的数据快照：驱动 count / popup-visible / 回显属性
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
            Property {
                name: "storage-message".to_string(),
                value: Value::Text(storage_message()),
            },
            Property {
                name: "event-log".to_string(),
                value: Value::Text(event_log()),
            },
            Property {
                name: "time-text".to_string(),
                value: Value::Text(time_text()),
            },
        ]
    }

    /// 宿主转发的统一事件：区分系统事件与自定义 UI 事件
    ///
    /// - 自定义事件（UI 触发，默认）：计数/弹窗/存储读写演示，以及
    ///   窗口调节按钮（经 `window` import 的 `set-size` 请求宿主调整工具窗口）。
    /// - 系统事件：`opened`（打开插件）/ `tick`（每秒节拍，payload 为数字
    ///   时间戳）/ `data-changed`（数据外部变更，预留）。
    fn dispatch_event(event: PluginEvent) -> bool {
        let source_label = match event.source {
            EventSource::System => "系统",
            EventSource::Custom => "UI",
        };
        set_event_log(format!("[{source_label}] {}", event.kind));

        match (event.source, event.kind.as_str()) {
            // —— 自定义 UI 事件 ——
            (EventSource::Custom, "button-clicked") => {
                COUNT.fetch_add(1, Ordering::Relaxed);
                true
            }
            (EventSource::Custom, "open-popup") => {
                POPUP_VISIBLE.store(true, Ordering::Relaxed);
                true
            }
            (EventSource::Custom, "close-popup") => {
                POPUP_VISIBLE.store(false, Ordering::Relaxed);
                true
            }
            (EventSource::Custom, "save-to-storage") => {
                let value = format!("count-{}", COUNT.load(Ordering::Relaxed));
                match storage::kv_set(DEMO_KEY, &value) {
                    Ok(()) => set_storage_message(format!("已保存: {value}")),
                    Err(e) => set_storage_message(format!("保存失败: {e:?}")),
                }
                true
            }
            (EventSource::Custom, "load-from-storage") => {
                match storage::kv_get(DEMO_KEY) {
                    Some(v) => {
                        // 解析 "count-N" 并恢复计数：数据驱动下宿主即时拉取快照，
                        // Count 界面随之覆盖为新值（持久化状态还原）
                        match v.strip_prefix("count-").and_then(|n| n.parse::<usize>().ok()) {
                            Some(restored) => {
                                COUNT.store(restored, Ordering::Relaxed);
                                set_storage_message(format!("已恢复: {v}"));
                            }
                            None => set_storage_message(format!("读取: {v}")),
                        }
                    }
                    None => set_storage_message("读取: (空)".to_string()),
                }
                true
            }
            (EventSource::Custom, "enlarge-window") => {
                let _ = window::set_size(ENLARGED_SIZE.0, ENLARGED_SIZE.1);
                true
            }
            (EventSource::Custom, "restore-window") => {
                let _ = window::set_size(RESTORED_SIZE.0, RESTORED_SIZE.1);
                true
            }
            // —— 系统事件 ——
            // tick：payload 为数字时间戳（Unix 秒），格式化后展示年月日时分秒
            (EventSource::System, "tick") => {
                if let Some(ts) = event.payload.and_then(|p| p.parse::<u64>().ok()) {
                    set_time_text(format_timestamp(ts));
                }
                true
            }
            _ => false,
        }
    }
}


// 声明导出（生成组件元数据），`with_types_in` 指向绑定生成所在的模块
export!(DemoPlugin with_types_in qt_sdk::bindings);
