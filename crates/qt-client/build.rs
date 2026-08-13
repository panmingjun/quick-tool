fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 启用实验特性：ComponentContainer 渲染容器需要 SLINT_ENABLE_EXPERIMENTAL_FEATURES
    std::env::set_var("SLINT_ENABLE_EXPERIMENTAL_FEATURES", "1");
    slint_build::compile("../../crates/qt-sdk/src/component/common.slint")?;
    Ok(())
}
