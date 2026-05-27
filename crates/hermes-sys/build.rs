use std::collections::HashSet;
use std::env;
use std::path::PathBuf;

use cmake::Config;

fn main() {
    let hermes_src_dir = "hermes";
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let hermes_src = manifest_dir.join(hermes_src_dir);

    println!("cargo:rerun-if-changed=src/binding.cc");
    println!("cargo:rerun-if-changed=src/binding.hpp");
    println!("cargo:rerun-if-changed={}/", hermes_src_dir);

    let hermes_build = Config::new(hermes_src_dir)
        .build_target("hermesvm_a")
        .configure_arg("-G Ninja")
        .define("HERMES_ENABLE_EH_RTTI", "ON")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("HERMES_BUILD_SHARED_JSI", "OFF")
        .define("HERMES_BUILD_APPLE_FRAMEWORK", "OFF")
        .build();

    let hermes_build_dir = format!("{}/build", hermes_build.display());

    println!("cargo:build_dir={}", hermes_build_dir);

    cc::Build::new()
        .cpp(true)
        .file("src/binding.cc")
        .include(hermes_src.join("API"))
        .include(hermes_src.join("API/jsi"))
        .include(hermes_src.join("public"))
        .include("src")
        .flag("-std=c++17")
        .flag("-fexceptions")
        .flag("-frtti")
        .compile("hermes_binding");

    let build_path = PathBuf::from(&hermes_build_dir);
    let mut search_dirs = HashSet::new();

    for entry in walkdir(&build_path) {
        if let Some(ext) = entry.extension() {
            if ext == "a" {
                let dir = entry.parent().unwrap();
                if search_dirs.insert(dir.to_path_buf()) {
                    println!("cargo:rustc-link-search=native={}", dir.display());
                }
                let stem = entry.file_stem().unwrap().to_str().unwrap();
                let name = stem.strip_prefix("lib").unwrap_or(stem);
                println!("cargo:rustc-link-lib=static={}", name);
            }
        }
    }

    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=c++");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    } else {
        println!("cargo:rustc-link-lib=stdc++");
        println!("cargo:rustc-link-lib=icuuc");
        println!("cargo:rustc-link-lib=icui18n");
        println!("cargo:rustc-link-lib=icudata");
    }
}

fn walkdir(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walkdir(&path));
            } else {
                files.push(path);
            }
        }
    }
    files
}
