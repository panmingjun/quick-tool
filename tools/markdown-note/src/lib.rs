//! Markdown 记事本插件
//!
//! 左侧目录树（文件夹层级 + 笔记列表），右侧 Markdown 编辑器/预览。
//! 支持编辑模式和预览模式切换。
//!
//! 数据持久化由宿主（qt-storage）管理，插件通过状态交换数据。

#![cfg(target_arch = "wasm32")]

use qt_sdk::bindings::export;
use qt_sdk::bindings::qt::plugin::types::{EventSource, PluginEvent, Property, Value};
use qt_sdk::bindings::Guest;
use std::sync::LazyLock;

/// 视图模式
#[derive(Debug, Clone, PartialEq, Eq)]
enum ViewMode {
    /// 编辑模式
    Edit,
    /// 预览模式
    Preview,
}

/// 插件内部状态
struct PluginState {
    /// 视图模式（编辑/预览）
    view_mode: ViewMode,
    /// 选中的文件夹 ID
    _selected_folder: Option<String>,
    /// 选中的笔记 ID
    selected_note: Option<String>,
    /// 笔记标题
    note_title: String,
    /// 笔记内容（Markdown）
    note_content: String,
    /// 文件夹树 JSON 字符串
    tree_json: String,
    /// 笔记列表 JSON 字符串
    note_list_json: String,
}

impl PluginState {
    fn new() -> Self {
        Self {
            view_mode: ViewMode::Edit,
            _selected_folder: None,
            selected_note: None,
            note_title: String::new(),
            note_content: String::new(),
            tree_json: String::new(),
            note_list_json: String::new(),
        }
    }
}

/// 全局插件状态（使用 LazyLock 初始化）
static STATE: LazyLock<std::sync::Mutex<PluginState>> =
    LazyLock::new(|| std::sync::Mutex::new(PluginState::new()));

/// 获取全局插件状态的不可变引用
fn get_state() -> std::sync::MutexGuard<'static, PluginState> {
    STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 获取全局插件状态的可变引用
fn get_state_mut() -> std::sync::MutexGuard<'static, PluginState> {
    STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Guest for PluginState {
    fn get_ui() -> String {
        r#"
            import { Button, TextInput, Text, Flickable, Rectangle, ColumnLayout, RowLayout } from "std-widgets.slint";
            export component MarkdownNoteWindow inherits Window {
                // === 左侧目录树 ===
                in property <string> tree-data: "";
                callback folder-selected(string);
                callback note-selected(string);

                // === 右侧编辑器/预览 ===
                in property <string> note-title: "";
                in-out property <string> note-content: "";
                in property <string> preview-content: "";
                in property <bool> is-preview-mode: false;
                callback title-changed(string);
                callback content-changed(string);

                // === 操作按钮 ===
                callback toggle-view-mode();
                callback new-folder();
                callback new-note();
                callback delete-note();

                width: 1000px;
                height: 700px;
                background: #1e1e2e;

                // 顶部工具栏
                RowLayout {
                    spacing: 8px;
                    padding: 8px;

                    Button {
                        text: "➕ 文件夹";
                        clicked => { root.new-folder(); }
                    }
                    Button {
                        text: "📄 笔记";
                        clicked => { root.new-note(); }
                    }
                    Button {
                        text: root.is-preview-mode ? "✏️ 编辑" : "👁️ 预览";
                        clicked => { root.toggle-view-mode(); }
                    }

                    Rectangle {
                        Layout { horizontal-stretch: 1; }
                    }

                    Text {
                        text: "Markdown 记事本";
                        color: #89b4fa;
                        font-size: 14px;
                    }
                }

                // 主区域：左右分栏
                RowLayout {
                    spacing: 0;
                    Layout { vertical-stretch: 1; }

                    // 左侧：目录树 + 笔记列表
                    Rectangle {
                        width: 280px;
                        background: #181825;

                        VerticalLayout {
                            spacing: 0;
                            padding: 4px;

                            Text {
                                text: "📂 目录";
                                color: #a6adc8;
                                font-size: 12px;
                                padding: 4px 8px;
                            }

                            Flickable {
                                Layout { vertical-stretch: 1; }
                                content-height: tree-content.height;

                ColumnLayout {
                                    id: tree-content;
                                    spacing: 2px;
                                    padding: 4px;

                                    Rectangle {
                                        Layout { horizontal-stretch: 1; }
                                        height: 200px;
                                        background: #1e1e2e;
                                        border-radius: 4px;

                                        VerticalLayout {
                                            padding: 4px;
                                            spacing: 2px;

                                            Text {
                                                text: "文件夹列表（由宿主渲染）";
                                                color: #6c7086;
                                                font-size: 11px;
                                                horizontal-alignment: center;
                                            }
                                        }
                                    }
                                }
                            }

                            // 笔记列表
                            Rectangle {
                                height: 200px;
                                background: #1e1e2e;
                                border-radius: 4px;
                                margin-top: 8px;

                                VerticalLayout {
                                    padding: 4px;
                                    spacing: 2px;

                                    Text {
                                        text: "📝 笔记列表";
                                        color: #a6adc8;
                                        font-size: 12px;
                                        padding: 2px 4px;
                                    }

                                    Flickable {
                                        Layout { vertical-stretch: 1; }
                                        content-height: note-list-content.height;

                ColumnLayout {
                                            id: note-list-content;
                                            spacing: 2px;
                                            padding: 4px;

                                            Rectangle {
                                                Layout { horizontal-stretch: 1; }
                                                height: 140px;
                                                background: #313244;
                                                border-radius: 4px;

                                                VerticalLayout {
                                                    padding: 4px;
                                                    spacing: 2px;

                                                    Text {
                                                        text: "选择或创建笔记";
                                                        color: #6c7086;
                                                        font-size: 11px;
                                                        horizontal-alignment: center;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // 分隔线
                    Rectangle {
                        width: 1px;
                        background: #45475a;
                    }

                    // 右侧：编辑器/预览
                    Rectangle {
                        Layout { horizontal-stretch: 1; }
                        background: #1e1e2e;

                        VerticalLayout {
                            spacing: 0;
                            padding: 8px;

                            // 笔记标题输入
                            Rectangle {
                                height: 40px;
                                background: #313244;
                                border-radius: 4px;

                                TextInput {
                                    text: note-title;
                                    color: #cdd6f4;
                                    font-size: 16px;
                                    padding: 4px 8px;
                                    focus: true;

                                    text-changed(new-text) { root.title-changed(new-text); }
                                }
                            }

                            // 编辑/预览区域
                            Rectangle {
                                Layout { vertical-stretch: 1; }
                                margin-top: 8px;
                                background: #313244;
                                border-radius: 4px;

                                // 编辑模式
                                TextInput {
                                    id: editor;
                                    visible: !root.is-preview-mode;
                                    anchors.fill: parent;
                                    anchors.margins: 8px;
                                    text: note-content;
                                    color: #cdd6f4;
                                    font-family: "Fira Code";
                                    font-size: 14px;
                                    wrap: TextEdit.Wrap;

                                    text-changed(new-text) { root.content-changed(new-text); }
                                }

                                // 预览模式（纯文本显示，实际应由宿主渲染 Markdown）
                                Text {
                                    id: preview;
                                    visible: root.is-preview-mode;
                                    anchors.fill: parent;
                                    anchors.margins: 8px;
                                    text: preview-content;
                                    color: #cdd6f4;
                                    font-size: 14px;
                                    wrap: TextEdit.Wrap;
                                    vertical-alignment: top;
                                    horizontal-alignment: left;
                                }
                            }
                        }
                    }
                }
            }
        "#
        .to_string()
    }

    fn get_state() -> Vec<Property> {
        let s = get_state();
        vec![
            Property {
                name: "note-title".to_string(),
                value: Value::Text(s.note_title.clone()),
            },
            Property {
                name: "note-content".to_string(),
                value: Value::Text(s.note_content.clone()),
            },
            Property {
                name: "preview-content".to_string(),
                value: Value::Text(s.note_content.clone()),
            },
            Property {
                name: "is-preview-mode".to_string(),
                value: Value::Boolean(matches!(s.view_mode, ViewMode::Preview)),
            },
            Property {
                name: "tree-data".to_string(),
                value: Value::Text(s.tree_json.clone()),
            },
            Property {
                name: "note-list-json".to_string(),
                value: Value::Text(s.note_list_json.clone()),
            },
        ]
    }

    /// 宿主转发的统一事件：仅处理自定义 UI 事件，忽略系统事件
    fn dispatch_event(event: PluginEvent) -> bool {
        // 系统事件（opened/tick/data-changed）不改变编辑器状态
        if event.source != EventSource::Custom {
            return false;
        }
        match event.kind.as_str() {
            "toggle-view-mode" => {
                let mut s = get_state_mut();
                s.view_mode = match s.view_mode {
                    ViewMode::Edit => ViewMode::Preview,
                    ViewMode::Preview => ViewMode::Edit,
                };
                true
            }
            "new-folder" => {
                let mut s = get_state_mut();
                s.tree_json = format!("{}|NEW_FOLDER", s.tree_json);
                true
            }
            "new-note" => {
                let mut s = get_state_mut();
                s.note_title = "新笔记".to_string();
                s.note_content.clear();
                s.tree_json = format!("{}|NEW_NOTE", s.tree_json);
                true
            }
            "delete-note" => {
                let mut s = get_state_mut();
                s.note_title.clear();
                s.note_content.clear();
                s.selected_note = None;
                s.tree_json = format!("{}|DELETE", s.tree_json);
                true
            }
            "folder-selected" => {
                // 文件夹选中事件由宿主处理
                true
            }
            "note-selected" => {
                // 笔记选中事件由宿主处理
                true
            }
            "title-changed" => {
                // 标题变化由宿主轮询 get_state 获取
                true
            }
            "content-changed" => {
                // 内容变化由宿主轮询 get_state 获取
                true
            }
            _ => false,
        }
    }
}

// 声明导出
export!(PluginState with_types_in qt_sdk::bindings);
