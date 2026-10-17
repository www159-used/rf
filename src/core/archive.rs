use crate::core::{Cancellation, Error, Progress, Result, io_error};
use fs2::FileExt;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read},
    path::{Path, PathBuf},
};
use tempfile::TempDir;
use walkdir::WalkDir;

#[derive(Clone, Debug)]
pub struct ArchiveStore {
    root: PathBuf,
}
impl ArchiveStore {
    pub fn system() -> Result<Self> {
        let base = match std::env::var_os("RF_DATA_DIR") {
            Some(path) => PathBuf::from(path),
            None => directories::BaseDirs::new()
                .ok_or_else(|| Error::Message("Cannot locate the user data directory".into()))?
                .data_dir()
                .join("rf"),
        };
        if !base.is_absolute() {
            return Err(Error::Message(
                "RF_DATA_DIR must be an absolute path".into(),
            ));
        }
        Ok(Self::new(base.join("archives")))
    }
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn open(&self) -> Result<(PathBuf, File)> {
        fs::create_dir_all(&self.root)
            .map_err(|e| io_error("Cannot create the archive storage directory", e))?;
        if fs::symlink_metadata(&self.root)
            .map_err(|e| io_error("Cannot inspect the archive storage directory", e))?
            .file_type()
            .is_symlink()
        {
            return Err(Error::Message(
                "Archive storage directory must not be a symbolic link".into(),
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.root, fs::Permissions::from_mode(0o700))
                .map_err(|e| io_error("Cannot set archive storage permissions", e))?;
        }
        let root = fs::canonicalize(&self.root)
            .map_err(|e| io_error("Cannot resolve the archive storage directory", e))?;
        if root.to_str().is_none() {
            return Err(Error::Message(
                "Archive storage path cannot be represented as valid Unicode".into(),
            ));
        }
        let mut opts = OpenOptions::new();
        opts.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        let lock = opts
            .open(root.join(".lock"))
            .map_err(|e| io_error("Cannot open the archive lock", e))?;
        if !lock
            .metadata()
            .map_err(|e| io_error("Cannot inspect the archive lock", e))?
            .is_file()
        {
            return Err(Error::Message("Archive lock is not a regular file".into()));
        }
        Ok((root, lock))
    }

    pub(crate) fn create(
        &self,
        source: &Path,
        cancel: &Cancellation,
        progress: &mut impl FnMut(Progress),
    ) -> Result<PendingArchive> {
        let (root, lock) = self.open()?;
        FileExt::try_lock_shared(&lock)
            .map_err(|e| io_error("Archive cleanup is in progress; try again later", e))?;
        if source.starts_with(&root) {
            return Err(Error::Message(
                "Cannot archive the archive storage directory or any of its subdirectories".into(),
            ));
        }
        let dir = tempfile::Builder::new()
            .prefix("rf-job-")
            .tempdir_in(&root)
            .map_err(|e| io_error("Cannot create an archive job directory", e))?;
        let part = dir.path().join("archive.part");
        let name = source
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("root"));
        // A constant output name avoids exceeding NAME_MAX for long source names.
        let path = dir.path().join("archive.tar");
        let file = File::create(&part).map_err(|e| io_error("Cannot create the tar archive", e))?;
        let mut builder = tar::Builder::new(file);
        builder.follow_symlinks(false);
        let mut state = Progress::default();
        let traversal = WalkDir::new(source)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| e.path() != root);
        for entry in traversal {
            cancel.check()?;
            let entry =
                entry.map_err(|e| Error::Message(format!("Cannot traverse the directory: {e}")))?;
            let src = entry.path();
            let relative = src.strip_prefix(source).expect("walk stays inside source");
            let dest = Path::new(name).join(relative);
            let meta = fs::symlink_metadata(src)
                .map_err(|e| io_error(format!("Cannot read {}", src.display()), e))?;
            if meta.is_dir() {
                builder
                    .append_dir(&dest, src)
                    .map_err(|e| io_error("Cannot add the directory to the archive", e))?;
            } else if meta.file_type().is_symlink() {
                let target =
                    fs::read_link(src).map_err(|e| io_error("Cannot read the symbolic link", e))?;
                let mut header = tar::Header::new_gnu();
                header.set_metadata(&meta);
                header.set_entry_type(tar::EntryType::Symlink);
                header.set_size(0);
                builder
                    .append_link(&mut header, &dest, target)
                    .map_err(|e| io_error("Cannot add the symbolic link to the archive", e))?;
            } else if meta.is_file() {
                let mut opts = OpenOptions::new();
                opts.read(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    opts.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
                }
                let file = opts
                    .open(src)
                    .map_err(|e| io_error(format!("Cannot read {}", src.display()), e))?;
                let before = file
                    .metadata()
                    .map_err(|e| io_error("Cannot read file metadata", e))?;
                if !before.is_file() {
                    return Err(Error::Message("File type changed while archiving".into()));
                }
                let mut header = tar::Header::new_gnu();
                header.set_metadata(&before);
                let reader = CancelReader {
                    inner: &file,
                    cancel,
                    state: &mut state,
                    progress,
                };
                builder
                    .append_data(&mut header, &dest, reader)
                    .map_err(|e| {
                        if cancel.is_cancelled() {
                            Error::Cancelled
                        } else {
                            io_error(format!("Cannot archive {}", src.display()), e)
                        }
                    })?;
                let after = file
                    .metadata()
                    .map_err(|e| io_error("Cannot recheck file metadata", e))?;
                if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
                    return Err(Error::Message(format!(
                        "File changed while archiving: {}",
                        src.display()
                    )));
                }
            } else {
                return Err(Error::Message(format!(
                    "Cannot archive a special file: {}",
                    src.display()
                )));
            }
            state.entries += 1;
            progress(state);
        }
        cancel.check()?;
        let file = builder
            .into_inner()
            .map_err(|e| io_error("Cannot finish writing the tar archive", e))?;
        file.sync_all()
            .map_err(|e| io_error("Cannot save the tar archive", e))?;
        fs::rename(&part, &path).map_err(|e| io_error("Cannot finalize the tar archive", e))?;
        Ok(PendingArchive {
            path,
            dir,
            _lock: lock,
        })
    }

    /// Exclusive lock prevents cleanup while any prepared archive is being copied.
    pub fn clean(&self, delete: bool) -> Result<Vec<PathBuf>> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let (root, lock) = self.open()?;
        FileExt::try_lock_exclusive(&lock).map_err(|e| {
            io_error(
                "An archive job is still running; try cleanup again later",
                e,
            )
        })?;
        let mut entries = Vec::new();
        for entry in fs::read_dir(&root).map_err(|e| io_error("Cannot list stored archives", e))? {
            let entry = entry.map_err(|e| io_error("Cannot read an archive storage entry", e))?;
            if !entry.file_name().to_string_lossy().starts_with("rf-job-") {
                continue;
            }
            let ty = entry
                .file_type()
                .map_err(|e| io_error("Cannot inspect an archive storage entry", e))?;
            // Never follow links, or touch unknown files in the store.
            if !ty.is_dir() || ty.is_symlink() {
                continue;
            }
            entries.push(entry.path());
        }
        entries.sort();
        if delete {
            for path in &entries {
                fs::remove_dir_all(path)
                    .map_err(|e| io_error("Cannot delete stored archives", e))?;
            }
        }
        Ok(entries)
    }
}

pub(crate) struct PendingArchive {
    pub path: PathBuf,
    dir: TempDir,
    _lock: File,
}
impl PendingArchive {
    pub fn keep(self) {
        let _ = self.dir.keep();
    }
}

struct CancelReader<'a, R, F> {
    inner: R,
    cancel: &'a Cancellation,
    state: &'a mut Progress,
    progress: &'a mut F,
}
impl<R: Read, F: FnMut(Progress)> Read for CancelReader<'_, R, F> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.cancel.is_cancelled() {
            return Err(io::Error::other("Operation cancelled"));
        }
        let limit = buf.len().min(64 * 1024);
        let n = self.inner.read(&mut buf[..limit])?;
        self.state.bytes += n as u64;
        (self.progress)(*self.state);
        Ok(n)
    }
}
