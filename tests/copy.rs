use rf_copy::core::{
    ArchiveStore, Cancellation, ClipboardPort, CopyMode, CopyRequest, Error, prepare, resolve,
};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Default)]
struct Clipboard {
    text: Option<String>,
    file: Option<PathBuf>,
    fail: bool,
}
impl ClipboardPort for Clipboard {
    fn set_text(&mut self, text: &str) -> rf_copy::core::Result<()> {
        if self.fail {
            return Err(Error::Message("backend failed".into()));
        }
        self.text = Some(text.into());
        Ok(())
    }
    fn set_file(&mut self, path: &Path) -> rf_copy::core::Result<()> {
        if self.fail {
            return Err(Error::Message("backend failed".into()));
        }
        self.file = Some(path.into());
        Ok(())
    }
}
fn request(path: &Path, mode: CopyMode) -> CopyRequest {
    CopyRequest {
        path: path.into(),
        mode,
        allow_large: false,
    }
}
fn setup() -> (tempfile::TempDir, PathBuf, ArchiveStore) {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("中文 空格 ' $ # %\n目录");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("a.txt"), "hello 世界").unwrap();
    fs::create_dir(source.join("empty")).unwrap();
    let store = ArchiveStore::new(temp.path().join("store"));
    (temp, source, store)
}
#[test]
fn modes_are_distinct_and_text_has_no_added_newline() {
    let (_temp, source, store) = setup();
    let cancel = Cancellation::default();
    let file = source.join("a.txt");
    for mode in [CopyMode::Text, CopyMode::File] {
        let mut clipboard = Clipboard::default();
        let copied = prepare(request(&file, mode), &store, &cancel, |_| {})
            .unwrap()
            .commit(&mut clipboard, &cancel)
            .unwrap();
        assert_eq!(copied.copied_path, fs::canonicalize(&file).unwrap());
        assert_eq!(clipboard.text.is_some(), mode == CopyMode::Text);
        assert_eq!(clipboard.file.is_some(), mode == CopyMode::File);
        if let Some(text) = clipboard.text {
            assert_eq!(text, copied.copied_path.to_str().unwrap());
        }
    }
    assert!(!store.root().exists());
}
#[test]
fn archives_are_unique_complete_and_retained_after_commit() {
    let (_temp, source, store) = setup();
    let cancel = Cancellation::default();
    let first = prepare(request(&source, CopyMode::File), &store, &cancel, |_| {}).unwrap();
    let second = prepare(request(&source, CopyMode::File), &store, &cancel, |_| {}).unwrap();
    let a = first.commit(&mut Clipboard::default(), &cancel).unwrap();
    let b = second.commit(&mut Clipboard::default(), &cancel).unwrap();
    assert_ne!(a.copied_path, b.copied_path);
    assert!(a.archive && a.copied_path.exists() && b.copied_path.exists());
    let mut archive = tar::Archive::new(fs::File::open(&a.copied_path).unwrap());
    let mut found = false;
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        if entry.path().unwrap().ends_with("a.txt") {
            let mut content = String::new();
            entry.read_to_string(&mut content).unwrap();
            assert_eq!(content, "hello 世界");
            found = true;
        }
    }
    assert!(found);
    let preview = store.clean(false).unwrap();
    assert_eq!(preview.len(), 2);
    assert!(a.copied_path.exists());
    assert_eq!(store.clean(true).unwrap(), preview);
    assert!(!a.copied_path.exists());
    assert!(source.join("a.txt").exists());
}
#[test]
fn cancellation_during_streaming_cleans_and_never_commits() {
    let (_temp, source, store) = setup();
    fs::write(source.join("large"), vec![0; 1024 * 1024]).unwrap();
    let cancel = Cancellation::default();
    let result = prepare(request(&source, CopyMode::File), &store, &cancel, |p| {
        if p.bytes > 0 {
            cancel.cancel();
        }
    });
    assert!(matches!(result, Err(Error::Cancelled)));
    assert!(store.clean(false).unwrap().is_empty());
}
#[test]
fn pending_copy_blocks_cleanup_and_failed_commit_removes_archive() {
    let (_temp, source, store) = setup();
    let cancel = Cancellation::default();
    let pending = prepare(request(&source, CopyMode::File), &store, &cancel, |_| {}).unwrap();
    assert!(store.clean(true).is_err());
    let mut clipboard = Clipboard {
        fail: true,
        ..Default::default()
    };
    assert!(pending.commit(&mut clipboard, &cancel).is_err());
    assert!(store.clean(false).unwrap().is_empty());
    assert!(clipboard.text.is_none() && clipboard.file.is_none());
}
#[test]
fn cancellation_before_commit_drops_prepared_archive() {
    let (_temp, source, store) = setup();
    let cancel = Cancellation::default();
    let pending = prepare(request(&source, CopyMode::File), &store, &cancel, |_| {}).unwrap();
    cancel.cancel();
    let mut clipboard = Clipboard::default();
    assert!(matches!(
        pending.commit(&mut clipboard, &cancel),
        Err(Error::Cancelled)
    ));
    assert!(clipboard.file.is_none());
    assert!(store.clean(false).unwrap().is_empty());
}
#[test]
fn storage_inside_source_is_excluded_and_cannot_be_archived_directly() {
    let (_temp, source, _) = setup();
    let store = ArchiveStore::new(source.join("archives"));
    let cancel = Cancellation::default();
    let result = prepare(request(&source, CopyMode::File), &store, &cancel, |_| {})
        .unwrap()
        .commit(&mut Clipboard::default(), &cancel)
        .unwrap();
    let mut tar = tar::Archive::new(fs::File::open(result.copied_path).unwrap());
    for entry in tar.entries().unwrap() {
        assert!(
            !entry
                .unwrap()
                .path()
                .unwrap()
                .components()
                .any(|p| p.as_os_str() == "archives")
        );
    }
    assert!(
        prepare(
            request(store.root(), CopyMode::File),
            &store,
            &cancel,
            |_| {}
        )
        .is_err()
    );
}
#[test]
fn missing_path_and_unwritable_store_fail_without_publishing() {
    let (temp, source, _) = setup();
    assert!(resolve(&temp.path().join("missing")).is_err());
    let block = temp.path().join("not-a-dir");
    fs::write(&block, "x").unwrap();
    let store = ArchiveStore::new(block.join("archives"));
    assert!(
        prepare(
            request(&source, CopyMode::File),
            &store,
            &Cancellation::default(),
            |_| {}
        )
        .is_err()
    );
}
#[cfg(unix)]
mod unix {
    use super::*;
    use std::os::unix::{
        ffi::OsStringExt,
        fs::{PermissionsExt, symlink},
        net::UnixListener,
    };
    #[test]
    fn top_level_links_resolve_but_nested_links_remain_links() {
        let (temp, source, store) = setup();
        let alias = temp.path().join("alias");
        symlink(&source, &alias).unwrap();
        assert_eq!(resolve(&alias).unwrap(), resolve(&source).unwrap());
        symlink("missing", source.join("dangling")).unwrap();
        assert!(resolve(&source.join("dangling")).is_err());
        symlink(&source, source.join("cycle")).unwrap();
        let token = Cancellation::default();
        let result = prepare(request(&alias, CopyMode::File), &store, &token, |_| {})
            .unwrap()
            .commit(&mut Clipboard::default(), &token)
            .unwrap();
        let mut archive = tar::Archive::new(fs::File::open(result.copied_path).unwrap());
        let links = archive
            .entries()
            .unwrap()
            .filter(|e| e.as_ref().unwrap().header().entry_type().is_symlink())
            .count();
        assert_eq!(links, 2);
    }
    #[test]
    fn non_unicode_and_special_files_are_rejected() {
        let (temp, source, store) = setup();
        let path = temp.path().join(std::ffi::OsString::from_vec(vec![0xff]));
        // APFS rejects non-Unicode names before the application can open them.
        #[cfg(not(target_os = "macos"))]
        fs::write(&path, "x").unwrap();
        assert!(resolve(&path).is_err());
        let socket = source.join("socket");
        let _listener = UnixListener::bind(&socket).unwrap();
        assert!(resolve(&socket).is_err());
        assert!(
            prepare(
                request(&source, CopyMode::File),
                &store,
                &Cancellation::default(),
                |_| {}
            )
            .is_err()
        );
        assert!(store.clean(false).unwrap().is_empty());
    }
    #[test]
    fn unreadable_file_fails_and_cleans() {
        let (_temp, source, store) = setup();
        let file = source.join("a.txt");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();
        let result = prepare(
            request(&source, CopyMode::File),
            &store,
            &Cancellation::default(),
            |_| {},
        );
        fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(result.is_err());
        assert!(store.clean(false).unwrap().is_empty());
    }
    #[test]
    fn cleanup_does_not_follow_symlinks_or_delete_unknown_entries() {
        let (temp, source, store) = setup();
        fs::create_dir_all(store.root()).unwrap();
        symlink(&source, store.root().join("rf-job-external")).unwrap();
        fs::write(store.root().join("unrelated"), "keep").unwrap();
        fs::create_dir(store.root().join("rf-job-interrupted")).unwrap();
        fs::write(
            store.root().join("rf-job-interrupted/archive.part"),
            "partial",
        )
        .unwrap();
        assert_eq!(store.clean(true).unwrap().len(), 1);
        assert!(source.join("a.txt").exists());
        assert!(store.root().join("unrelated").exists());
        let alias = temp.path().join("store-alias");
        symlink(store.root(), &alias).unwrap();
        assert!(ArchiveStore::new(alias).clean(true).is_err());
    }
}

#[test]
fn entry_limit_requires_explicit_override() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("many");
    fs::create_dir(&source).unwrap();
    // The source directory itself counts as one archive entry.
    for index in 0..9_999 {
        fs::File::create(source.join(index.to_string())).unwrap();
    }
    let store = ArchiveStore::new(temp.path().join("store"));
    let cancel = Cancellation::default();
    let error = prepare(request(&source, CopyMode::File), &store, &cancel, |_| {})
        .err()
        .expect("entry limit must reject the archive");
    assert!(error.to_string().contains("--allow-large"));
    assert!(store.clean(false).unwrap().is_empty());

    let mut allowed = request(&source, CopyMode::File);
    allowed.allow_large = true;
    let mut clipboard = Clipboard::default();
    let result = prepare(allowed, &store, &cancel, |_| {})
        .unwrap()
        .commit(&mut clipboard, &cancel)
        .unwrap();
    let mut archive = tar::Archive::new(fs::File::open(&result.copied_path).unwrap());
    assert_eq!(archive.entries().unwrap().count(), 10_000);
    assert_eq!(clipboard.file.as_ref(), Some(&result.copied_path));
}

#[test]
fn growth_after_preflight_still_stops_and_removes_the_partial_archive() {
    let (_temp, source, store) = setup();
    let file = source.join("a.txt");
    let mut grew = false;
    let result = prepare(
        request(&source, CopyMode::File),
        &store,
        &Cancellation::default(),
        |progress| {
            if !grew && progress.entries == 1 {
                fs::OpenOptions::new()
                    .write(true)
                    .open(&file)
                    .unwrap()
                    .set_len(512 * 1024 * 1024)
                    .unwrap();
                grew = true;
            }
        },
    );
    assert!(grew);
    assert!(result.err().unwrap().to_string().contains("--allow-large"));
    assert!(store.clean(false).unwrap().is_empty());
}

#[test]
#[cfg(unix)]
fn size_scan_excludes_storage_and_does_not_follow_internal_links() {
    let (temp, source, _store) = setup();
    let store = ArchiveStore::new(source.join("stored-archives"));
    fs::create_dir(store.root()).unwrap();
    for path in [store.root().join("old.tar"), temp.path().join("outside")] {
        fs::File::create(path)
            .unwrap()
            .set_len(512 * 1024 * 1024)
            .unwrap();
    }
    std::os::unix::fs::symlink(temp.path().join("outside"), source.join("link")).unwrap();
    let prepared = prepare(
        request(&source, CopyMode::File),
        &store,
        &Cancellation::default(),
        |_| {},
    )
    .unwrap();
    drop(prepared);
    assert!(store.clean(false).unwrap().is_empty());
}

#[test]
fn size_limit_sums_files_but_does_not_block_references_or_text() {
    let (_temp, source, store) = setup();
    let cancel = Cancellation::default();
    for name in ["one", "two"] {
        fs::File::create(source.join(name))
            .unwrap()
            .set_len(256 * 1024 * 1024)
            .unwrap();
    }
    assert!(
        prepare(request(&source, CopyMode::File), &store, &cancel, |_| {})
            .err()
            .unwrap()
            .to_string()
            .contains("--allow-large")
    );
    let big_file = source.join("one");
    fs::OpenOptions::new()
        .write(true)
        .open(&big_file)
        .unwrap()
        .set_len(512 * 1024 * 1024)
        .unwrap();
    for (path, mode) in [(&source, CopyMode::Text), (&big_file, CopyMode::File)] {
        let result = prepare(request(path, mode), &store, &cancel, |_| {})
            .unwrap()
            .commit(&mut Clipboard::default(), &cancel)
            .unwrap();
        assert!(!result.archive);
    }
    assert!(store.clean(false).unwrap().is_empty());
}
