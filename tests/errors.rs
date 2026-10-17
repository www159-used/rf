use std::process::Command;
#[test]
fn error_paths_have_nonzero_status_and_empty_stdout() {
    let temp = tempfile::tempdir().unwrap();
    for (args, code) in [
        (vec!["--unknown"], 2),
        (vec!["a", "b"], 2),
        (vec!["does-not-exist"], 1),
        (vec!["--tui"], 2),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_rf"))
            .args(args)
            .current_dir(temp.path())
            .env("RF_DATA_DIR", temp.path().join("data"))
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(code),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}
#[test]
fn cleanup_preview_on_empty_store_does_not_create_files() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("data");
    let output = Command::new(env!("CARGO_BIN_EXE_rf"))
        .arg("--clean-archives")
        .env("RF_DATA_DIR", &data)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!data.exists());
}
