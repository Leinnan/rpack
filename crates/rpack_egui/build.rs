use std::process::Command;

fn main() {
    // note: add error checking yourself.
    {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        let git_hash = String::from_utf8(output.stdout).unwrap();
        println!("cargo:rustc-env=GIT_HASH={}", &git_hash[..7]);
    }
    {
        let output = Command::new("git")
            .args(["log", "-1", "--date=format:%Y/%m/%d %T", "--format=%ad"])
            .output()
            .unwrap();
        let git_hash = String::from_utf8(output.stdout).unwrap();
        println!("cargo:rustc-env=GIT_DATE={}", git_hash);
    }

    if std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|t| t.eq("windows")) {
        let icon = image::open("./static/base_icon.png").expect("Failed to open icon");
        let new_icon = icon.resize(128, 128, image::imageops::FilterType::Lanczos3);
        new_icon
            .save("../../target/icon_128.png")
            .expect("Failed to save");
        let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
        let gen_image_file =
            std::fs::File::open("../../target/icon_128.png").expect("failed to open icon file");
        let gen_image =
            ico::IconImage::read_png(gen_image_file).expect("failed to read icon image");
        icon_dir.add_entry(ico::IconDirEntry::encode(&gen_image).unwrap());
        let file = std::fs::File::create("../../target/icon.ico").unwrap();
        icon_dir.write(file).expect("Failed to write icon");
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../target/icon.ico");
        let _ = res.compile();
    }
}
