use std::env;

fn main() {
    println!("cargo:rerun-if-changed=assets/rust-autoclicker.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon("assets/rust-autoclicker.ico")
            .set("ProductName", "Rust AutoClicker")
            .set("OriginalFilename", "RustAutoClicker.exe")
            .set("InternalName", "RustAutoClicker");
        resource
            .compile()
            .expect("could not compile the Windows icon and version resources");
    }
}
