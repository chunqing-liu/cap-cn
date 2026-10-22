#[path = "../../scripts/diagnostic-build.rs"]
mod diagnostic_build;

fn main() {
    diagnostic_build::emit();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rerun-if-changed=native/exception_guard.m");
        cc::Build::new()
            .file("native/exception_guard.m")
            .flag("-fobjc-exceptions")
            .flag("-fno-objc-arc")
            .compile("cap_exception_guard");
        println!("cargo:rustc-link-lib=framework=Foundation");
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        // Export dispatch constructs media futures on the UI thread before Tokio can poll them.
        println!("cargo:rustc-link-arg-bin=cap-gpui=/STACK:16777216");
    }

    let icon = "../desktop/src-tauri/icons/icon.ico";
    println!("cargo:rerun-if-changed={icon}");
    let product_name = if std::env::var_os("CARGO_FEATURE_LOCAL_CHINESE_TRIAL").is_some() {
        "Cap 中文试用版（非官方）"
    } else {
        "Cap"
    };
    tauri_winres::WindowsResource::new()
        .set_icon_with_id(icon, "1")
        .set("ProductName", product_name)
        .set("FileDescription", product_name)
        .set("OriginalFilename", "Cap.exe")
        .compile()
        .expect("failed to compile the Cap Windows icon resource");
}
