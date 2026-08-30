include!("../build_support/winres.rs");

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../build_support/winres.rs");

    // The exe's own icon and version block. Without this resource the
    // taskbar, Alt-Tab and Explorer all show the default program icon —
    // which read as "the logo is missing" next to any released build.
    #[cfg(windows)]
    {
        let repo = std::env::current_dir()
            .ok()
            .and_then(|cwd| cwd.parent().map(|parent| parent.to_path_buf()))
            .expect("unterm-app sits inside the repository");
        let ico = repo.join("assets").join("windows").join("terminal.ico");
        emit_version_resource("unterm", "Unterm Terminal", Some(&ico));
    }
}
