//! 插件发现集成测试
//!
//! 验证从项目目录的 config/config.json 能够正确扫描到所有本地插件。

use std::path::PathBuf;

/// 获取项目根目录（crates/qt-runtime 的祖父目录）
fn project_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let crates_dir = manifest
        .parent()
        .expect("无法获取 crates 目录");
    crates_dir
        .parent()
        .expect("无法获取项目根目录")
        .to_path_buf()
}

#[test]
fn test_discover_from_config_path() {
    let root = project_root();
    let config_path = root.join("config/config.json");
    assert!(
        config_path.is_file(),
        "config.json 应该在 {:?} 存在",
        config_path
    );

    let content = std::fs::read_to_string(&config_path)
        .expect("读取 config.json");
    let config: serde_json::Value =
        serde_json::from_str(&content).expect("解析 config.json");

    let sources = config["plugin_sources"]["sources"]
        .as_array()
        .expect("plugin_sources.sources 应该是数组");
    assert!(
        !sources.is_empty(),
        "应该有至少一个插件源"
    );

    for src in sources {
        let kind = src["kind"].as_str().expect("kind 字段");
        let url = src["url"].as_str().expect("url 字段");
        let enabled = src["enabled"].as_bool().unwrap_or(false);

        if kind != "local" || !enabled {
            continue;
        }

        // 相对路径相对于项目根目录解析
        let source_dir = if url.starts_with('/') {
            PathBuf::from(url)
        } else {
            root.join(url)
        };

        assert!(
            source_dir.is_dir(),
            "插件源目录 {:?} 应该是目录",
            source_dir
        );

        let entries = std::fs::read_dir(&source_dir)
            .expect("读取插件源目录");
        let mut plugin_count = 0;
        for entry in entries {
            let p = entry.unwrap().path();
            if p.is_dir() && p.join("plugin.json").is_file() {
                plugin_count += 1;
                let manifest =
                    std::fs::read_to_string(p.join("plugin.json"))
                        .expect("读取 plugin.json");
                let m: serde_json::Value =
                    serde_json::from_str(&manifest).expect("解析 plugin.json");
                let id = m["id"].as_str().unwrap();
                println!("发现插件: {} ({:?})", id, p);
            }
        }

        assert_eq!(
            plugin_count, 2,
            "插件源 {:?} 应该有 2 个插件 (demo-plugin, markdown-note)，实际找到 {} 个",
            source_dir, plugin_count
        );
    }
}
