include!("../build_support/winres.rs");

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../build_support/winres.rs");
    #[cfg(windows)]
    emit_version_resource("unterm-cli", "Unterm CLI", None);
}
