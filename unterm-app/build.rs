fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // The exe's own icon and version block. Without this resource the
    // taskbar, Alt-Tab and Explorer all show the default program icon —
    // which read as "the logo is missing" next to any released build.
    #[cfg(windows)]
    {
        use std::io::Write;

        let repo = std::env::current_dir()
            .ok()
            .and_then(|cwd| cwd.parent().map(|parent| parent.to_path_buf()))
            .expect("unterm-app sits inside the repository");
        let ico = repo.join("assets").join("windows").join("terminal.ico");
        println!("cargo:rerun-if-changed={}", ico.display());
        let ico = ico.display().to_string().replace('\\', "\\\\");

        let version = std::env::var("CARGO_PKG_VERSION").unwrap();
        let mut parts: Vec<&str> = version.split('.').collect();
        while parts.len() < 4 {
            parts.push("0");
        }
        let commas = parts.join(",");

        let rc_path =
            std::path::Path::new(&std::env::var_os("OUT_DIR").unwrap()).join("resource.rc");
        let mut rc = std::fs::File::create(&rc_path).unwrap();
        write!(
            rc,
            r#"
#define IDI_ICON 0x101
IDI_ICON ICON "{ico}"
1 VERSIONINFO
FILEVERSION     {commas}
PRODUCTVERSION  {commas}
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904E4"
        BEGIN
            VALUE "CompanyName",      "Unzoo\0"
            VALUE "FileDescription",  "Unterm Terminal\0"
            VALUE "FileVersion",      "{version}\0"
            VALUE "LegalCopyright",   "Unzoo, MIT licensed\0"
            VALUE "InternalName",     "Unterm\0"
            VALUE "OriginalFilename", "unterm.exe\0"
            VALUE "ProductName",      "Unterm\0"
            VALUE "ProductVersion",   "{version}\0"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x409, 1252
    END
END
"#
        )
        .unwrap();
        drop(rc);
        // Named target, not "every artifact in this package": the package now
        // has a lib as well, and the resource belongs to the exe.
        embed_resource::compile_for(&rc_path, ["unterm"]);
    }
}
