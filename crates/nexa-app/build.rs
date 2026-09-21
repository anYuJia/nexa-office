#[cfg(target_os = "windows")]
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=ui/app.slint");
    slint_build::compile("ui/app.slint").expect("the checked-in Slint UI must compile");

    #[cfg(target_os = "windows")]
    {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../packaging/windows/nexa-office.manifest")
            .canonicalize()
            .expect("Windows application manifest must exist");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    }
}
