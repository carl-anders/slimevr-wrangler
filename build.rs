#[cfg(target_os = "windows")]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/icon.ico");
    res.set("ProductName", "SlimeVR Wrangler");
    res.set("FileDescription", "SlimeVR Wrangler");
    res.compile().unwrap();
}

#[cfg(not(target_os = "windows"))]
fn main() {
}
