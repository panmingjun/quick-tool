//! 插件独立键值存储测试
//!
//! 验证：打开初始化、读写往返、删除、键列表、双插件隔离、配额限制。

use qt_storage::per_plugin::{KvError, PluginStore, DEFAULT_QUOTA_BYTES};
use std::path::PathBuf;

/// 为单个用例生成独立的临时数据根目录
fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "qt-storage-test-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn kv_set_get_roundtrip() {
    let root = temp_root("roundtrip");
    let mut store = PluginStore::open(&root, "plugin-a", DEFAULT_QUOTA_BYTES).unwrap();

    assert_eq!(store.kv_get("missing").unwrap(), None, "不存在的键应返回 None");

    store.kv_set("greeting", "你好，世界").unwrap();
    assert_eq!(
        store.kv_get("greeting").unwrap().as_deref(),
        Some("你好，世界"),
        "读回应命中刚写入的值（含 UTF-8 多字节）"
    );

    // 覆盖写
    store.kv_set("greeting", "hello").unwrap();
    assert_eq!(store.kv_get("greeting").unwrap().as_deref(), Some("hello"));

    // 持久化：重新打开同一库可读到
    drop(store);
    let reopened = PluginStore::open(&root, "plugin-a", DEFAULT_QUOTA_BYTES).unwrap();
    assert_eq!(reopened.kv_get("greeting").unwrap().as_deref(), Some("hello"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn kv_delete_and_keys() {
    let root = temp_root("delete-keys");
    let mut store = PluginStore::open(&root, "plugin-a", DEFAULT_QUOTA_BYTES).unwrap();

    for key in ["b", "a", "c"] {
        store.kv_set(key, "v").unwrap();
    }
    assert_eq!(store.kv_keys().unwrap(), vec!["a", "b", "c"], "键应按字典序返回");

    store.kv_delete("b").unwrap();
    // 删除不存在的键视为成功
    store.kv_delete("nope").unwrap();
    assert_eq!(store.kv_keys().unwrap(), vec!["a", "c"]);
    assert_eq!(store.kv_get("b").unwrap(), None);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn plugins_are_isolated_by_separate_dbs() {
    let root = temp_root("isolation");
    let mut a = PluginStore::open(&root, "plugin-a", DEFAULT_QUOTA_BYTES).unwrap();
    let mut b = PluginStore::open(&root, "plugin-b", DEFAULT_QUOTA_BYTES).unwrap();

    a.kv_set("shared-key", "from-a").unwrap();
    b.kv_set("shared-key", "from-b").unwrap();

    assert_eq!(a.kv_get("shared-key").unwrap().as_deref(), Some("from-a"));
    assert_eq!(b.kv_get("shared-key").unwrap().as_deref(), Some("from-b"));
    assert_eq!(a.kv_keys().unwrap(), vec!["shared-key"]);
    assert_eq!(b.kv_keys().unwrap(), vec!["shared-key"]);

    // b 删除不影响 a
    b.kv_delete("shared-key").unwrap();
    assert!(a.kv_get("shared-key").unwrap().is_some());

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn quota_rejects_write_beyond_limit() {
    let root = temp_root("quota");
    // 配额 100 字节
    let mut store = PluginStore::open(&root, "plugin-a", 100).unwrap();

    store.kv_set("k1", "0123456789").unwrap(); // 2 + 10 = 12 字节
    assert_eq!(store.used_bytes().unwrap(), 12);

    // 覆盖写更小的值：总量下降，应成功
    store.kv_set("k1", "x").unwrap(); // 2 + 1 = 3 字节
    assert_eq!(store.used_bytes().unwrap(), 3);

    // 单次写入超过配额 → 拒绝并携带配额上限
    let big_value = "y".repeat(200);
    match store.kv_set("k2", &big_value) {
        Err(KvError::QuotaExceeded(quota)) => assert_eq!(quota, 100),
        other => panic!("应返回 QuotaExceeded(100)，实际 {other:?}"),
    }
    // 被拒绝的写入不应落库
    assert_eq!(store.kv_get("k2").unwrap(), None);

    // 累计超限同样拒绝
    for i in 0..30 {
        if store.kv_set(&format!("key{i}"), "0123456789").is_err() {
            break;
        }
    }
    assert!(
        store.used_bytes().unwrap() <= 100,
        "已用字节数不应超过配额"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn invalid_plugin_id_is_rejected() {
    let root = temp_root("invalid-id");
    for bad in ["../evil", "", ".hidden", "a/b", "a\\b"] {
        assert!(
            PluginStore::open(&root, bad, DEFAULT_QUOTA_BYTES).is_err(),
            "非法插件 ID 应被拒绝: {bad}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}
