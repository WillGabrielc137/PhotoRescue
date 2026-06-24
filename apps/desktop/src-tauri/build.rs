fn main() {
    #[cfg(windows)]
    {
        let mut windows = tauri_build::WindowsAttributes::new();
        if std::env::var("PROFILE").as_deref() == Ok("release") {
            windows = windows.app_manifest(include_str!("windows-app-manifest.xml"));
        }

        let attributes = tauri_build::Attributes::new().windows_attributes(windows);
        tauri_build::try_build(attributes).expect("falha ao preparar os recursos do Windows");
    }

    #[cfg(not(windows))]
    tauri_build::build();
}
