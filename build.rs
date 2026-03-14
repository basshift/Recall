use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const GETTEXT_PACKAGE: &str = "io.github.basshift.Recall";

fn main() {
    track_resource_inputs();
    compile_translations();
    compile_resources();
}

fn track_resource_inputs() {
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
    println!("cargo:rerun-if-changed=data/victory/finish-flag.svg");
    println!("cargo:rerun-if-changed=data/howto/01-flow.svg");
    println!("cargo:rerun-if-changed=data/howto/02-goal.svg");
    println!("cargo:rerun-if-changed=data/howto/03-modes.svg");
    println!("cargo:rerun-if-changed=data/howto/04-difficulty.svg");
    println!("cargo:rerun-if-changed=data/howto/05-restless.svg");
    println!("cargo:rerun-if-changed=data/icons/hicolor/index.theme");
    println!("cargo:rerun-if-changed=data/icons/hicolor/scalable/apps/io.github.basshift.Recall.svg");
    println!(
        "cargo:rerun-if-changed=data/icons/hicolor/scalable/apps/io.github.basshift.Recall.Devel.svg"
    );
}

fn compile_translations() {
    let linguas_path = Path::new("po/LINGUAS");
    println!("cargo:rerun-if-changed={}", linguas_path.display());

    let linguas = fs::read_to_string(linguas_path).expect("failed to read po/LINGUAS");
    for lang in linguas.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let po_path = PathBuf::from(format!("po/{lang}.po"));
        let mo_path = PathBuf::from(format!("po/{lang}/LC_MESSAGES/{GETTEXT_PACKAGE}.mo"));

        println!("cargo:rerun-if-changed={}", po_path.display());

        if let Some(parent_dir) = mo_path.parent() {
            fs::create_dir_all(parent_dir).expect("failed to create locale output directory");
        }

        let status = Command::new("msgfmt")
            .arg(&po_path)
            .arg("-o")
            .arg(&mo_path)
            .status()
            .expect("failed to execute msgfmt; install gettext on the host");

        if !status.success() {
            panic!("msgfmt failed for {}", po_path.display());
        }
    }
}

fn compile_resources() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let output = Path::new(&out_dir).join("recall.gresource");
    let status = Command::new("glib-compile-resources")
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
