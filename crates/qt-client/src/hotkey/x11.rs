//! X11 全局快捷键实现
//!
//! 使用 XGrabKey 拦截全局按键

use crate::config::hotkey::{Hotkey, Key, Modifier};
use crate::hotkey::platform::{HotkeyEvent, HotkeyManager};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

/// X11 快捷键管理器
pub struct X11HotkeyManager {
    registered_hotkeys: Vec<Hotkey>,
    event_sender: Option<Sender<HotkeyEvent>>,
}

impl X11HotkeyManager {
    pub fn new() -> Self {
        Self {
            registered_hotkeys: Vec::new(),
            event_sender: None,
        }
    }
}

/// 将修饰键转换为 X11 ModMask
fn modifier_to_x11(modifiers: &[Modifier]) -> u32 {
    let mut mask = 0u32;
    for m in modifiers {
        match m {
            Modifier::Ctrl => mask |= x11rb::protocol::xproto::ModMask::CONTROL.into(),
            Modifier::Alt => mask |= x11rb::protocol::xproto::ModMask::M1.into(),
            Modifier::Shift => mask |= x11rb::protocol::xproto::ModMask::SHIFT.into(),
            Modifier::Super => mask |= x11rb::protocol::xproto::ModMask::M4.into(),
        }
    }
    mask
}

/// 将按键转换为 X11 keycode
fn key_to_x11_keycode(key: &Key) -> Option<u8> {
    // X11 keycode 常用映射（可能因键盘布局不同）
    match key {
        Key::Space => Some(65),
        Key::A => Some(38),
        Key::B => Some(56),
        Key::C => Some(54),
        Key::F1 => Some(67),
        Key::F2 => Some(68),
        Key::F3 => Some(69),
        Key::F4 => Some(70),
        Key::F5 => Some(71),
        Key::F6 => Some(72),
        Key::F7 => Some(73),
        Key::F8 => Some(74),
        Key::F9 => Some(75),
        Key::F10 => Some(76),
        Key::F11 => Some(95),
        Key::F12 => Some(96),
        _ => None,
    }
}

impl HotkeyManager for X11HotkeyManager {
    fn register(&mut self, hotkey: &Hotkey) -> qt_core::Result<()> {
        tracing::info!("注册 X11 快捷键: {:?}", hotkey);
        self.registered_hotkeys.push(hotkey.clone());
        Ok(())
    }

    fn unregister(&mut self, hotkey: &Hotkey) -> qt_core::Result<()> {
        tracing::info!("取消注册 X11 快捷键: {:?}", hotkey);
        self.registered_hotkeys.retain(|h| h != hotkey);
        Ok(())
    }

    fn listen(&mut self) -> Receiver<HotkeyEvent> {
        let (tx, rx) = channel();
        self.event_sender = Some(tx.clone());

        // 启动 X11 事件监听线程
        let hotkeys = self.registered_hotkeys.clone();
        thread::spawn(move || {
            if let Err(e) = run_x11_event_loop(hotkeys, tx) {
                tracing::error!("X11 事件循环错误: {}", e);
            }
        });

        rx
    }
}

/// 运行 X11 事件循环
fn run_x11_event_loop(hotkeys: Vec<Hotkey>, sender: Sender<HotkeyEvent>) -> qt_core::Result<()> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::*;
    use x11rb::xcb_ffi::XCBConnection;

    // 连接到 X11
    let (conn, screen_num) = XCBConnection::connect(None)
        .map_err(|e| qt_core::Error::Hotkey(format!("X11 连接失败: {}", e)))?;

    let screen = &conn.setup().roots[screen_num];
    let root_window = screen.root;

    tracing::info!("X11 连接成功，根窗口: {}", root_window);

    // 注册所有快捷键
    for hotkey in &hotkeys {
        let keycode = key_to_x11_keycode(&hotkey.key);
        if let Some(keycode) = keycode {
            let modifiers = modifier_to_x11(&hotkey.modifiers);

            // 使用 XGrabKey 拦截按键
            // GrabModeAsync 表示异步处理，不阻塞其他程序
            conn.send_request(&GrabKey {
                owner_events: true,
                grab_window: root_window,
                modifiers: modifiers,
                key: keycode,
                pointer_mode: GrabMode::ASYNC,
                keyboard_mode: GrabMode::ASYNC,
            })
            .map_err(|e| qt_core::Error::Hotkey(format!("XGrabKey 请求失败: {}", e)))?;

            tracing::info!("已注册快捷键: keycode={}, modifiers={}", keycode, modifiers);
        } else {
            tracing::warn!("无法映射按键: {:?}", hotkey.key);
        }
    }

    // 刷新连接
    conn.flush()
        .map_err(|e| qt_core::Error::Hotkey(format!("X11 flush 失败: {}", e)))?;

    // 选择 KeyPress 事件
    conn.send_request(&SelectInput {
        window: root_window,
        event_mask: EventMask::KEY_PRESS | EventMask::SUBSTRUCTURE_NOTIFY,
    })
    .map_err(|e| qt_core::Error::Hotkey(format!("SelectInput 失败: {}", e)))?;

    conn.flush()
        .map_err(|e| qt_core::Error::Hotkey(format!("X11 flush 失败: {}", e)))?;

    tracing::info!("开始监听 X11 键盘事件");

    // 事件循环
    loop {
        conn.flush()
            .map_err(|e| qt_core::Error::Hotkey(format!("X11 flush 失败: {}", e)))?;

        // 等待事件
        let event = conn.wait_for_event()
            .map_err(|e| qt_core::Error::Hotkey(format!("等待事件失败: {}", e)))?;

        match event {
            Event::KeyPress(kp) => {
                tracing::debug!("KeyPress: keycode={}, state={}", kp.detail, kp.state);

                // 检查是否匹配注册的快捷键
                for hotkey in &hotkeys {
                    let expected_keycode = key_to_x11_keycode(&hotkey.key);
                    let expected_modifiers = modifier_to_x11(&hotkey.modifiers);

                    if expected_keycode == Some(kp.detail) && expected_modifiers == kp.state {
                        tracing::info!("快捷键匹配: {:?}", hotkey);

                        sender.send(HotkeyEvent {
                            hotkey: hotkey.clone(),
                        }).ok();
                    }
                }
            }
            _ => {}
        }
    }
}

impl Default for X11HotkeyManager {
    fn default() -> Self {
        Self::new()
    }
}