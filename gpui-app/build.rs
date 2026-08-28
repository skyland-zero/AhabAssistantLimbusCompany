fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=resources/windows/app.rc");
        println!("cargo:rerun-if-changed=resources/windows/app-icon.ico");
        embed_resource::compile("resources/windows/app.rc", embed_resource::NONE)
            .manifest_optional()
            .unwrap();
    }
}
