use std::{
    collections::BTreeMap,
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{self, Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::fs::symlink;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    manifest_dir().parent().unwrap().to_path_buf()
}

fn release_dir() -> PathBuf {
    workspace_root().join("target/release")
}

fn compat_dir() -> PathBuf {
    workspace_root().join("target/compat")
}

fn compat_include_dir() -> PathBuf {
    compat_dir().join("include")
}

fn original_header() -> PathBuf {
    workspace_root().join("original/csv.h")
}

fn compiler() -> String {
    env::var("CC").unwrap_or_else(|_| "gcc".into())
}

fn unique_output_path(stem: &str, extension: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    env::temp_dir().join(format!("{stem}-{}-{nonce}.{extension}", process::id()))
}

fn run(command: &mut Command, context: &str) -> Output {
    let output = command
        .output()
        .unwrap_or_else(|err| panic!("{context} failed to start: {err}"));
    assert!(
        output.status.success(),
        "{context} failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    output
}

fn ensure_release_artifacts() {
    let shared = release_dir().join("libcsv.so");
    let archive = release_dir().join("libcsv.a");
    assert!(
        shared.is_file(),
        "missing shared library at {}; run `cargo build --manifest-path safe/Cargo.toml --release` first",
        shared.display(),
    );
    assert!(
        archive.is_file(),
        "missing static archive at {}; run `cargo build --manifest-path safe/Cargo.toml --release` first",
        archive.display(),
    );
}

#[cfg(unix)]
fn ensure_soname_symlink() {
    let soname = release_dir().join("libcsv.so.3");
    if soname.exists() {
        return;
    }

    let _ = fs::remove_file(&soname);
    symlink("libcsv.so", &soname).unwrap();
}

fn stage_public_header() -> PathBuf {
    let include_dir = compat_include_dir();
    fs::create_dir_all(&include_dir).unwrap();
    let staged = include_dir.join("csv.h");
    fs::copy(original_header(), &staged).unwrap();
    assert_eq!(
        fs::read(original_header()).unwrap(),
        fs::read(&staged).unwrap()
    );
    staged
}

fn compile_c_object(source: &Path, output: &Path, include_dir: Option<&Path>) {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    let mut command = Command::new(compiler());
    if let Some(include_dir) = include_dir {
        command.arg("-I").arg(include_dir);
    }
    command
        .current_dir(workspace_root())
        .arg("-c")
        .arg(source)
        .arg("-o")
        .arg(output);
    run(&mut command, &format!("compile {}", source.display()));
}

fn compile_shared_binary(source: &Path, include_dir: &Path, output: &Path) {
    let mut command = Command::new(compiler());
    command
        .current_dir(workspace_root())
        .arg("-I")
        .arg(include_dir)
        .arg(source)
        .arg("-L")
        .arg(release_dir())
        .arg(format!("-Wl,-rpath,{}", release_dir().display()))
        .arg("-lcsv")
        .arg("-o")
        .arg(output);
    run(&mut command, &format!("link shared {}", source.display()));
}

fn link_shared_binary(object: &Path, output: &Path) {
    let mut command = Command::new(compiler());
    command
        .current_dir(workspace_root())
        .arg(object)
        .arg("-L")
        .arg(release_dir())
        .arg(format!("-Wl,-rpath,{}", release_dir().display()))
        .arg("-lcsv")
        .arg("-o")
        .arg(output);
    run(
        &mut command,
        &format!("link shared compatibility object {}", object.display()),
    );
}

fn link_static_binary(object: &Path, output: &Path) {
    let mut command = Command::new(compiler());
    command
        .current_dir(workspace_root())
        .arg(object)
        .arg(release_dir().join("libcsv.a"))
        .arg("-o")
        .arg(output);
    run(
        &mut command,
        &format!("link static compatibility object {}", object.display()),
    );
}

fn run_binary(path: &Path) -> Output {
    let mut command = Command::new(path);
    command.current_dir(workspace_root());
    run(&mut command, &format!("run {}", path.display()))
}

fn run_binary_with_stdin(path: &Path, stdin_path: &Path) -> Output {
    let input = fs::read(stdin_path).unwrap();
    let mut command = Command::new(path);
    command
        .current_dir(workspace_root())
        .stdin(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::piped());
    let mut child = command
        .spawn()
        .unwrap_or_else(|err| panic!("run {} failed to start: {err}", path.display()));
    child.stdin.take().unwrap().write_all(&input).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "run {} failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        path.display(),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    output
}

fn compile_examples(include_dir: &Path) -> BTreeMap<String, PathBuf> {
    let mut examples = BTreeMap::new();
    let mut paths = fs::read_dir(workspace_root().join("original/examples"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("c"))
        .collect::<Vec<_>>();
    paths.sort();

    for path in paths {
        let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
        let output = unique_output_path(&format!("libcsv-{stem}"), "bin");
        compile_shared_binary(&path, include_dir, &output);
        examples.insert(stem, output);
    }

    examples
}

fn compile_c_fixture(name: &str, include_dir: &Path) -> PathBuf {
    let output = unique_output_path(&format!("libcsv-{name}"), "bin");
    let source = manifest_dir().join(format!("tests/c/{name}.c"));
    compile_shared_binary(&source, include_dir, &output);
    output
}

#[test]
fn staged_header_and_real_c_callers_match_the_upstream_surface() {
    ensure_release_artifacts();
    #[cfg(unix)]
    ensure_soname_symlink();

    let include_header = stage_public_header();
    let include_dir = include_header.parent().unwrap();

    let public_header_object = unique_output_path("libcsv-public-header-smoke", "o");
    compile_c_object(
        &manifest_dir().join("tests/c/public_header_smoke.c"),
        &public_header_object,
        Some(include_dir),
    );
    assert!(public_header_object.is_file());

    let object_path = compat_dir().join("original-test_csv.o");
    let _ = fs::remove_file(&object_path);
    compile_c_object(
        &workspace_root().join("original/test_csv.c"),
        &object_path,
        None,
    );
    assert!(object_path.is_file());

    let shared_test = unique_output_path("libcsv-test_csv-shared", "bin");
    link_shared_binary(&object_path, &shared_test);
    let shared_output = run_binary(&shared_test);
    assert_eq!(
        String::from_utf8(shared_output.stdout).unwrap(),
        "All tests passed\n"
    );

    let static_test = unique_output_path("libcsv-test_csv-static", "bin");
    link_static_binary(&object_path, &static_test);
    let static_output = run_binary(&static_test);
    assert_eq!(
        String::from_utf8(static_output.stdout).unwrap(),
        "All tests passed\n"
    );

    run_binary(&compile_c_fixture("abi_edges", include_dir));
    run_binary(&compile_c_fixture("allocator_failures", include_dir));

    let examples = compile_examples(include_dir);
    let sample = unique_output_path("libcsv-sample", "csv");
    fs::write(&sample, b"a,b\n1,2\n").unwrap();

    let csvtest = run_binary_with_stdin(examples.get("csvtest").unwrap(), &sample);
    assert_eq!(
        String::from_utf8(csvtest.stdout).unwrap(),
        "\"a\",\"b\"\n\"1\",\"2\"\n"
    );

    let mut command = Command::new(examples.get("csvvalid").unwrap());
    command.current_dir(workspace_root()).arg(&sample);
    let csvvalid = run(&mut command, "run csvvalid");
    assert_eq!(
        String::from_utf8(csvvalid.stdout).unwrap(),
        format!("{} well-formed\n", sample.display()),
    );

    let mut command = Command::new(examples.get("csvinfo").unwrap());
    command.current_dir(workspace_root()).arg(&sample);
    let csvinfo = run(&mut command, "run csvinfo");
    assert_eq!(
        String::from_utf8(csvinfo.stdout).unwrap(),
        format!("{}: 4 fields, 2 rows\n", sample.display()),
    );
}
