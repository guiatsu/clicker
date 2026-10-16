use std::env;

fn main() {
    println!("cargo:rerun-if-changed=assets/neo-autoclicker.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon("assets/neo-autoclicker.ico")
            .set("ProductName", "Neo AutoClicker")
            .set("OriginalFilename", "NeoAutoClicker.exe")
            .set("InternalName", "NeoAutoClicker");
        resource
            .compile()
            .expect("could not compile the Windows icon and version resources");
    }
}
