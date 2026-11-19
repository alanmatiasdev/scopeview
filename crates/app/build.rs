//! Embeds the application icon into the Windows executable. Does nothing on other systems.

fn main() {
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("../../assets/icon/scopeview.ico");
        if let Err(error) = resource.compile() {
            println!("cargo:warning=could not embed the Windows icon: {error}");
        }
    }
    println!("cargo:rerun-if-changed=../../assets/icon/scopeview.ico");
}
