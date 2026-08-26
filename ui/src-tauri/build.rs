fn main() {
    // Windows: 以管理员权限运行整个应用（壳提权后，Python sidecar 子进程天然继承）
    #[cfg(windows)]
    {
        let attrs = tauri_build::Attributes::new().windows_attributes(
            tauri_build::WindowsAttributes::new().app_manifest(include_str!("app.manifest")),
        );
        tauri_build::try_build(attrs).expect("failed to run tauri-build");
    }
    #[cfg(not(windows))]
    tauri_build::build()
}
