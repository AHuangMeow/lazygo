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

    for res_dir in &res_dirs {
        if has_icons {
            copy_tree(&source, res_dir);
        }

        if let Some(manifest) = res_dir.parent().map(|dir| dir.join("AndroidManifest.xml")) {
            set_landscape(&manifest);
            println!("cargo:rerun-if-changed={}", manifest.display());
        }
    }

    if has_icons {
        println!("cargo:rerun-if-changed=android/res");
    }

    for res_dir in &res_dirs {
        println!("cargo:rerun-if-changed={}", res_dir.display());
    }
}

fn set_landscape(manifest: &Path) {
    let contents = match fs::read_to_string(manifest) {
        Ok(contents) => contents,
        Err(_) => return,
    };

    if contents.contains("android:screenOrientation") {
        return;
    }

    let marker = r#"android:name="dev.dioxus.main.MainActivity""#;
    let replacement = r#"android:name="dev.dioxus.main.MainActivity" android:screenOrientation="sensorLandscape""#;
    let updated = contents.replacen(marker, replacement, 1);

    if updated != contents {
        let _ = fs::write(manifest, updated);
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
