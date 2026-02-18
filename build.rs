fn main() {
    println!("cargo:rerun-if-changed=data/resources.gresource.xml");
    println!("cargo:rerun-if-changed=data/style.vars.css");
    println!("cargo:rerun-if-changed=data/style.css");
    println!("cargo:rerun-if-changed=data/style.light.css");
    println!("cargo:rerun-if-changed=data/style.dark.css");
    println!("cargo:rerun-if-changed=data/style.mobile.css");
    println!("cargo:rerun-if-changed=data/victory/rank-s.svg");
    println!("cargo:rerun-if-changed=data/victory/rank-a.svg");
    println!("cargo:rerun-if-changed=data/victory/rank-b.svg");
    println!("cargo:rerun-if-changed=data/victory/rank-c.svg");
    println!("cargo:rerun-if-changed=data/icons/hicolor/index.theme");
    println!("cargo:rerun-if-changed=data/icons/hicolor/scalable/apps/io.basshift.Recall.svg");
    println!(
        "cargo:rerun-if-changed=data/icons/hicolor/scalable/apps/io.basshift.Recall.Devel.svg"
    );

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let output = std::path::Path::new(&out_dir).join("recall.gresource");
    let status = std::process::Command::new("glib-compile-resources")
        .arg("--sourcedir=data")
        .arg("--sourcedir=data/icons/hicolor")
        .arg("--target")
        .arg(&output)
        .arg("data/resources.gresource.xml")
        .status()
        .expect("failed to execute glib-compile-resources");

    if !status.success() {
        panic!("glib-compile-resources failed");
    }
}
