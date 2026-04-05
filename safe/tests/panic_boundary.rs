use std::{
    env, fs,
    path::PathBuf,
    process::{self, Command},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::{fs::symlink, process::ExitStatusExt};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    manifest_dir().parent().unwrap().to_path_buf()
}

fn release_dir() -> PathBuf {
    workspace_root().join("target/release")
}

fn unique_output_path(stem: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    env::temp_dir().join(format!("{stem}-{}-{nonce}", process::id()))
}

#[cfg(unix)]
fn ensure_soname_symlink() {
    let release_dir = release_dir();
    let soname = release_dir.join("libcsv.so.3");
    if soname.exists() {
        return;
    }

    let _ = fs::remove_file(&soname);
    symlink("libcsv.so", &soname).unwrap();
}

fn compile_tripwire() -> PathBuf {
    let output = unique_output_path("libcsv-panic-tripwire");
    let source = manifest_dir().join("tests/helpers/panic_tripwire.rs");
    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let status = Command::new(rustc)
        .arg("--edition=2024")
        .arg(&source)
        .arg("-L")
        .arg(format!("native={}", release_dir().display()))
        .arg("-o")
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success(), "failed to compile panic tripwire helper");
    output
}

fn ld_library_path() -> String {
    let release_dir = release_dir();
    match env::var_os("LD_LIBRARY_PATH") {
        Some(existing) if !existing.is_empty() => {
            format!(
                "{}:{}",
                release_dir.display(),
                PathBuf::from(existing).display()
            )
        }
        _ => release_dir.display().to_string(),
    }
}

#[test]
fn panic_across_the_c_boundary_aborts_instead_of_unwinding() {
    #[cfg(unix)]
    ensure_soname_symlink();

    let helper = compile_tripwire();
    let output = Command::new(&helper)
        .env("LD_LIBRARY_PATH", ld_library_path())
        .output()
        .unwrap();

    assert!(!output.status.success(), "tripwire unexpectedly succeeded");

    #[cfg(unix)]
    assert_eq!(
        output.status.signal(),
        Some(6),
        "expected SIGABRT, stderr was:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("panic") || stderr.contains("abort"),
        "unexpected stderr: {stderr}",
    );
}
