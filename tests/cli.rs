use std::path::{Path, PathBuf};
use std::process::Command;

fn pith_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pith"))
}

fn fixture_path(path: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(path)
        .to_string_lossy()
        .into_owned()
}

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("pith-cli-{name}-{unique}"));
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[test]
fn multiple_inputs_get_source_sections() {
    let first = fixture_path("plain/01_ascii.txt");
    let second = fixture_path("plain/02_utf8.txt");

    let output = pith_bin()
        .args([&first, &second])
        .output()
        .expect("run pith");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains(&format!("# Source: {first}")));
    assert!(stdout.contains(&format!("# Source: {second}")));
    assert!(stdout.contains("Hello world"));
    assert!(stdout.contains("中文"));
}

#[test]
fn glob_inputs_expand_inside_cli() {
    let dir = TestDir::new("glob_inputs_expand_inside_cli");
    std::fs::write(dir.path().join("b.txt"), "bravo\n").unwrap();
    std::fs::write(dir.path().join("a.txt"), "alpha\n").unwrap();
    std::fs::write(dir.path().join("skip.md"), "skip\n").unwrap();

    let pattern = dir.path().join("*.txt").to_string_lossy().into_owned();
    let output = pith_bin().arg(pattern).output().expect("run pith");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("# Source:"));
    assert!(stdout.find("a.txt").unwrap() < stdout.find("b.txt").unwrap());
    assert!(stdout.contains("alpha"));
    assert!(stdout.contains("bravo"));
    assert!(!stdout.contains("skip"));
}

#[test]
fn format_override_is_honored() {
    let source = fixture_path("html/06_links.html");
    let output = pith_bin()
        .args(["--format", "text", &source])
        .output()
        .expect("run pith");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("<a href=\"https://example.com\""));
    assert!(!stdout.contains("[our site](https://example.com)"));
}

#[test]
fn extraction_errors_exit_nonzero() {
    let source = fixture_path("plain/01_ascii.txt");
    let output = pith_bin()
        .args(["--format", "docx", &source])
        .output()
        .expect("run pith");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("error:"));
    assert!(stderr.contains("failed to open docx archive"));
}

#[test]
fn version_flag_reports_binary_name() {
    let output = pith_bin().arg("--version").output().expect("run pith");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.starts_with("pith "));
}
