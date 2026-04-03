fn main() {
    // Compile Slint UI
    slint_build::compile("ui/main.slint").unwrap();

    // Embed icon in the .exe on Windows
    // NOTE: #[cfg(target_os)] checks the HOST os, not the target.
    // Use CARGO_CFG_TARGET_OS for correct cross-compilation support.
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("icons/xai_generator.ico");
        res.set("ProductName", "AI Batch Image Generator");
        res.set("FileDescription", "Batch Image Generator v2");
        res.set("ProductVersion", "2.2.0");
        res.compile().unwrap();
    }
}