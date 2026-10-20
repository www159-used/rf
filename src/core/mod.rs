mod archive;

pub use archive::ArchiveStore;
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: io::Error,
    },
    #[error("Operation cancelled")]
    Cancelled,
}
pub type Result<T> = std::result::Result<T, Error>;
pub(crate) fn io_error(context: impl Into<String>, source: io::Error) -> Error {
    Error::Io {
        context: context.into(),
        source,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyMode {
    Text,
    File,
}
impl CopyMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Text => "Path text",
            Self::File => "File reference (directories are archived)",
        }
    }
}
#[derive(Clone, Debug)]
pub struct CopyRequest {
    pub path: PathBuf,
    pub mode: CopyMode,
    pub allow_large: bool,
}
#[derive(Debug)]
pub struct CopyResult {
    pub resolved_source: PathBuf,
    pub copied_path: PathBuf,
    pub mode: CopyMode,
    pub archive: bool,
}

#[derive(Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
    pub fn check(&self) -> Result<()> {
        if self.is_cancelled() {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Progress {
    pub entries: u64,
    pub bytes: u64,
}

pub trait ClipboardPort {
    fn set_text(&mut self, text: &str) -> Result<()>;
    fn set_file(&mut self, path: &Path) -> Result<()>;
}

pub fn resolve(path: &Path) -> Result<PathBuf> {
    let resolved = fs::canonicalize(path)
        .map_err(|e| io_error(format!("Cannot resolve path {}", path.display()), e))?;
    if resolved.to_str().is_none() {
        return Err(Error::Message(
            "Path cannot be represented as valid Unicode".into(),
        ));
    }
    let meta = fs::metadata(&resolved).map_err(|e| io_error("Cannot read path metadata", e))?;
    if !meta.is_file() && !meta.is_dir() {
        return Err(Error::Message(
            "Only regular files and directories are supported".into(),
        ));
    }
    Ok(resolved)
}

/// Owns an unpublished archive and its shared store lock until clipboard commit.
/// Dropping a prepared copy removes any archive that was never successfully copied.
pub struct PreparedCopy {
    result: CopyResult,
    archive: Option<archive::PendingArchive>,
}
impl PreparedCopy {
    pub fn commit(
        mut self,
        clipboard: &mut (impl ClipboardPort + ?Sized),
        cancel: &Cancellation,
    ) -> Result<CopyResult> {
        cancel.check()?;
        match self.result.mode {
            CopyMode::Text => {
                clipboard.set_text(self.result.copied_path.to_str().expect("validated Unicode"))?
            }
            CopyMode::File => clipboard.set_file(&self.result.copied_path)?,
        }
        // Commit is deliberately not cancellable once the platform call has begun.
        if let Some(archive) = self.archive.take() {
            archive.keep();
        }
        Ok(self.result)
    }
}

pub fn prepare(
    request: CopyRequest,
    store: &ArchiveStore,
    cancel: &Cancellation,
    mut progress: impl FnMut(Progress),
) -> Result<PreparedCopy> {
    cancel.check()?;
    let source = resolve(&request.path)?;
    let archive = if request.mode == CopyMode::File && source.is_dir() {
        Some(store.create(&source, request.allow_large, cancel, &mut progress)?)
    } else {
        None
    };
    cancel.check()?;
    let copied_path = archive
        .as_ref()
        .map(|a| a.path.clone())
        .unwrap_or_else(|| source.clone());
    Ok(PreparedCopy {
        result: CopyResult {
            resolved_source: source,
            copied_path,
            mode: request.mode,
            archive: archive.is_some(),
        },
        archive,
    })
}
