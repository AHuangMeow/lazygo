use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("android") {
        return;
    }

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let source = manifest_dir.join("android").join("res");
    let has_icons = source.is_dir();

    let target_dir = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                manifest_dir.join(path)
            }
        })
        .unwrap_or_else(|| manifest_dir.join("target"));

    let mut res_dirs = Vec::new();
    find_res_dirs(&target_dir.join("dx"), &mut res_dirs);

    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let abi = match arch.as_str() {
        "aarch64" => Some("arm64-v8a"),
        "x86_64" => Some("x86_64"),
        _ => None,
    };
    let engine = abi.map(|abi| {
        manifest_dir
            .join("target")
            .join("katago-dist")
            .join(abi)
            .join("libkatago.so")
    });

    for res_dir in &res_dirs {
        if has_icons {
            copy_tree(&source, res_dir);
        }

        if let Some(manifest) = res_dir.parent().map(|dir| dir.join("AndroidManifest.xml")) {
            set_portrait(&manifest);
            println!("cargo:rerun-if-changed={}", manifest.display());
        }
    }

    if has_icons {
        println!("cargo:rerun-if-changed=android/res");
    }

    if let Some(engine) = engine {
        if engine.exists() {
            let mut jni_dirs = Vec::new();
            find_jni_dirs(&target_dir.join("dx"), &mut jni_dirs);

            for jni_dir in &jni_dirs {
                let destination = jni_dir.join(abi.unwrap());
                let _ = fs::create_dir_all(&destination);
                let _ = fs::copy(&engine, destination.join("libkatago.so"));
            }

            println!("cargo:rerun-if-changed={}", engine.display());
        } else {
            println!(
                "cargo:warning=KataGo engine missing at {}; analysis will be disabled",
                engine.display()
            );
        }
    }

    for res_dir in &res_dirs {
        println!("cargo:rerun-if-changed={}", res_dir.display());
    }
}

fn set_portrait(manifest: &Path) {
    let contents = match fs::read_to_string(manifest) {
        Ok(contents) => contents,
        Err(_) => return,
    };

    if contents.contains("android:screenOrientation") {
        return;
    }

    let marker = r#"android:name="dev.dioxus.main.MainActivity""#;
    let replacement =
        r#"android:name="dev.dioxus.main.MainActivity" android:screenOrientation="portrait""#;
    let updated = contents.replacen(marker, replacement, 1);

    if updated != contents {
        let _ = fs::write(manifest, updated);
    }
}

fn find_jni_dirs(dir: &Path, found: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        if path.ends_with("android/app/app/src/main/jniLibs")
            || path.ends_with("android/app/src/main/jniLibs")
        {
            found.push(path);
        } else {
            find_jni_dirs(&path, found);
        }
    }
}

fn find_res_dirs(dir: &Path, found: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        if is_android_res_dir(&path) {
            found.push(path);
        } else {
            find_res_dirs(&path, found);
        }
    }
}

fn is_android_res_dir(path: &Path) -> bool {
    path.ends_with("android/app/app/src/main/res") || path.ends_with("android/app/src/main/res")
}

fn copy_tree(source: &Path, destination: &Path) {
    let entries = match fs::read_dir(source) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let target = destination.join(entry.file_name());

        if path.is_dir() {
            let _ = fs::create_dir_all(&target);
            copy_tree(&path, &target);
        } else {
            if let Some(parent) = target.parent() {
                let _ = fs::create_dir_all(parent);
            }

            let _ = fs::copy(&path, &target);
        }
    }
}
