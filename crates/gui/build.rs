//! Embeds the program icon into faderust.exe (Task Manager, Explorer, shortcuts).

fn main() {
    println!("cargo:rerun-if-changed=faderust.rc");
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows") {
        embed_resource::compile("faderust.rc", embed_resource::NONE)
            .manifest_optional()
            .expect("icon resource");
    }
}
