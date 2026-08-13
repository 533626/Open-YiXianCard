use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let mut paths = vec![
        root.join("Cargo.toml"),
        root.join("Cargo.lock"),
        root.join("build.rs"),
    ];
    collect_files(&root.join("src"), &mut paths);
    collect_files(&root.join("data"), &mut paths);
    paths.sort_by_key(|path| relative_path(&root, path));

    let mut hash = FNV_OFFSET_BASIS;
    for path in paths {
        println!("cargo:rerun-if-changed={}", path.display());
        update_hash(&mut hash, relative_path(&root, &path).as_bytes());
        update_hash(&mut hash, &[0]);
        update_hash(
            &mut hash,
            &fs::read(&path).unwrap_or_else(|error| {
                panic!(
                    "failed to read fingerprint input {}: {error}",
                    path.display()
                )
            }),
        );
        update_hash(&mut hash, &[0]);
    }
    println!("cargo:rustc-env=YIXIAN_ENGINE_SOURCE_FINGERPRINT={hash:016x}");
}

fn collect_files(root: &Path, paths: &mut Vec<PathBuf>) {
    let mut entries = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
        .map(|entry| entry.expect("directory entry").path())
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_files(&path, paths);
        } else if path.is_file() {
            paths.push(path);
        }
    }
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("fingerprint input under manifest root")
        .to_string_lossy()
        .replace('\\', "/")
}

fn update_hash(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}
