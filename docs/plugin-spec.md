# 插件接入规范

> 接入规范参考 [uTools 插件开发](https://www.u-tools.cn/docs/developer/information/plugin-json.html) 的 `plugin.json` 结构，运行引擎为 WASM（Wasmtime）。
> 插件在客户端中采用「先选择，后进入」的交互：启动后展示本地已安装插件列表，选中插件后进入其界面。

## 1. 插件包结构

每个插件是本地一个独立目录，目录中必须包含 `plugin.json` 作为插件入口配置：

```
plugins/
└── demo-plugin/                # 插件目录（目录名建议与 id 一致）
    ├── plugin.json             # 插件清单（必需）
    ├── main.wasm               # WASM 入口（plugin.json 的 main 指向，相对路径）
    └── logo.png                # 图标（可选）
```

客户端会扫描以下位置的插件目录：

- 项目开发目录 `<cwd>/plugins`
- 用户数据目录 `<data_dir>/plugins`

每个插件子目录内的 `plugin.json` 将被解析并加入列表。

## 2. plugin.json 字段

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 是 | 插件唯一标识 |
| `name` | string | 是 | 插件显示名称 |
| `version` | string | 否 | 版本号 |
| `author` | string | 否 | 作者 |
| `description` | string | 否 | 功能描述 |
| `main` | string | 是 | WASM 入口，相对 `plugin.json` 所在目录的路径 |
| `logo` | string | 否 | 图标路径 |
| `engine` | string | 否 | 运行引擎标识，默认 `wasm`；仅加载 `wasm` 插件 |
| `features` | array | 否 | 功能指令集合（参考 uTools） |

### features

| 字段 | 类型 | 说明 |
|------|------|------|
| `code` | string | 功能指令编码 |
| `explain` | string | 功能说明 |
| `cmds` | array | 匹配指令（关键词，用于唤起/搜索） |

### 完整示例

```json
{
  "id": "demo-plugin",
  "name": "Demo 插件",
  "version": "1.0.0",
  "author": "Quick Tool Team",
  "description": "验证本地 WASM 插件机制的演示插件",
  "main": "main.wasm",
  "engine": "wasm",
  "features": [
    {
      "code": "demo",
      "explain": "运行演示：点击按钮数字加一",
      "cmds": ["demo", "计数", "测试"]
    }
  ]
}
```

## 3. WASM ABI 约定

插件基于 wasmtime **Component Model** 与 WIT 契约（`crates/qt-sdk/wit/plugin.wit`）开发。WIT 是插件与宿主共享的接口定义，双方各自用 `wit-bindgen` 生成绑定，类型安全、内存自动管理，无需手写指针/长度。

### 渲染模型：数据驱动（类 Vue）

宿主/插件采用「类 Vue」的运行模型，`get-ui` 与 `get-state` 分离，从而根治状态变化导致的组件重建与闪烁：

| 侧 | 职责 | 对应 |
|----|------|------|
| 插件 `get-ui` | 返回静态页面模板（`.slint` 源码） | Vue `<template>` |
| 插件 `get-state` | 返回数据快照（VO），宿主据此更新界面 | Vue `data` |
| 宿主 | 编译模板、绑定数据、转发事件 | Vue 运行时 |
| 插件 `dispatch-action` | 处理交互事件，更新内部状态 | Vue methods |

当前 `qt:plugin` 世界导出以下函数：

| 导出函数 | 类型 | 说明 |
|----------|------|------|
| `get-ui` | `func() -> string` | 返回页面模板（`.slint` 源码字符串）。宿主仅在模板**变化时**重新编译渲染（页面级切换） |
| `get-state` | `func() -> vo` | 返回数据快照（VO）。宿主在事件后即时拉取，数据变化时对已编译组件实例 `set_property` 增量更新，**不重建组件树** |
| `dispatch-event` | `func(event: plugin-event) -> bool` | 宿主转发统一事件结构（系统事件 + 自定义 UI 事件），插件处理并返回是否发生状态变化 |

并开放以下 **import 接口**（宿主能力出口）：

| 导入接口 | 说明 |
|----------|------|
| `storage` | 键值存储（key/value 均为 UTF-8 字符串）。每个插件一个独立 SQLite 库（`data/plugins/<plugin_id>/data.sqlite`），数据按插件完全隔离；写入按已用字节累计受配额限制（默认 10 MiB/插件），超限返回 `quota-exceeded`。可用函数：`kv-get` / `kv-set` / `kv-delete` / `kv-keys` / `used-bytes` |

> 架构边界：插件对存储**只发指令**——`storage::kv_*` 是 WIT 生成的绑定函数，
> 仅序列化参数并触发宿主回调；SQL 执行、文件 IO、配额检查全部由宿主
> （`qt-storage::per_plugin::PluginStore`）实现。插件侧无任何数据库依赖，
> 只需依赖 `qt-sdk`。

WIT 数据类型：

- `value`：变体，取值 `boolean(bool)` / `text(string)` / `numeric(f64)`。
- `property`：记录，`{ name: string, value: value }`，`name` 对应模板中公开属性的标识符（snake_case）。
- `vo`：`list<property>`，`get-state` 的返回类型。

### 事件模型

宿主与插件间通过统一事件结构 `plugin-event`（record）通信，以 `event-source`
区分来源，避免系统事件与 UI 事件混用字符串：

| 来源 | `kind` 取值 | 触发时机 |
|------|------------|----------|
| `system` | `opened` | 插件被打开（进入插件窗口后发送一次） |
| `system` | `tick` | 时间节拍（插件窗口活跃期间每秒一次） |
| `system` | `data-changed` | 插件数据被外部变更（预留，如同步回写） |
| `custom` | 模板回调名（如 `button-clicked`） | UI 上触发的交互（**默认**） |

- record 结构：`{ source: event-source, kind: string, payload: option<string> }`，
  系统事件可通过 `payload` 携带数据（如变更的数据键）。
- 宿主侧便捷构造：`qt_runtime::plugin::{custom_event, system_event}` 与
  常量 `EVENT_OPENED` / `EVENT_TICK` / `EVENT_DATA_CHANGED`。

### 插件模板要求

- 模板结构应**保持稳定**：状态变化走数据更新，不通过更换模板实现。
- 需由宿主驱动的属性必须以 `in-out property` / `in property` 可见性声明，否则宿主 `set_property` 会失败：

```slint
import { Button } from "std-widgets.slint";
export component CounterUI inherits Rectangle {
    callback button-clicked;
    in-out property <int> count: 0;
    VerticalLayout {
        Button {
            text: "点击加一";
            clicked => { root.button-clicked(); }
        }
        Text {
            text: "Count: \{root.count}";
            font-size: 32px;
            horizontal-alignment: center;
        }
    }
}
```

### 插件侧（guest）

插件依赖 `qt-sdk`，实现 `Guest` trait 后通过 `export!` 声明导出：

```rust
// demo-plugin/src/lib.rs
#![cfg(target_arch = "wasm32")]

use qt_sdk::bindings::export;
use qt_sdk::bindings::qt::plugin::types::{Property, Value};
use qt_sdk::bindings::Guest;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNT: AtomicUsize = AtomicUsize::new(0);

struct DemoPlugin;

impl Guest for DemoPlugin {
    fn get_ui() -> String {
        /* 稳定模板，见上文 */
        r#"..."#.to_string()
    }

    fn get_state() -> Vec<Property> {
        vec![Property {
            name: "count".to_string(),
            value: Value::Numeric(COUNT.load(Ordering::Relaxed) as f64),
        }]
    }

    fn dispatch_action(action: String) -> bool {
        match action.as_str() {
            "button-clicked" => { COUNT.fetch_add(1, Ordering::Relaxed); true }
            _ => false,
        }
    }
}

export!(DemoPlugin with_types_in qt_sdk::bindings);
```

### 宿主侧（host）

`qt-runtime` 用 `wasmtime::component::bindgen!` 生成宿主绑定，加载时先用 `wit-component` 的 `ComponentEncoder` 把 core wasm（含 wit-bindgen 写入的 `component-type` 自定义段）转换为 component 二进制，再实例化并轮询调用：

```rust
let engine = qt_runtime::engine::WasmEngine::new()?;
let mut plugin = engine.instantiate_plugin(&wasm)?;          // 内部完成 core → component 转换
let ui: String = plugin.get_ui()?;                           // 获取页面模板
let state: Vec<Property> = plugin.get_state()?;              // 轮询获取数据快照
```

宿主渲染流程：

1. `get-ui` 获取模板，编译为渲染工厂；工厂创建实例后存入共享槽。
2. 按 30FPS 轮询 `get-state`，与上次快照比较；数据变化则对共享实例逐个 `set_property`，Slint 内部增量更新。
3. 捕获 UI 回调 → `dispatch-action` 转发给插件 → 插件更新状态 → 下一 tick 数据快照变化 → 宿主更新界面。

> 注：宿主能力出口（存储/网络/剪贴板/加密）通过 WIT `import` 接口开放，
> 宿主与插件两侧绑定同步更新。当前已开放 `storage`，其余待引入。

## 4. 构建与调试

```bash
# 添加 WASM 目标
rustup target add wasm32-unknown-unknown

# 构建插件
cargo build -p demo-plugin --target wasm32-unknown-unknown --release

# 启动客户端（先展示插件列表）
cargo run --bin qt-client
```

交互流程：

1. 启动后主窗口展示本地已发现插件列表（名称 + 版本）。
2. 点击某个插件 → 进入插件窗口（标题显示插件名）。
3. 点击「执行插件」→ 调用 WASM 导出函数，文本显示插件返回内容（如 `Hello, World!`）。
4. 点击「返回列表」→ 回到插件列表。

## 5. 参考

- [总体设计文档](design.md)
- [插件开发指南](plugin-development.md)
- [调试启动命令](debug-commands.md)