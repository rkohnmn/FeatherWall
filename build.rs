fn main() {
    if std::env::var("TARGET").map_or(false, |t| t.contains("windows")) {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/featherwall.ico");
        res.set("ProductName", "FeatherWall");
        res.set("FileDescription", "FeatherWall Native Animated Wallpaper & Information Overlay");
        res.set("LegalCopyright", "Copyright (c) 2026 Pleiades");
        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=Failed to compile Windows resource: {}", e);
        }
    }
}
