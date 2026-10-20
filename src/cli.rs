use crate::core::{ArchiveStore, Cancellation, CopyMode, CopyRequest, Error};
use clap::Parser;
use std::{
    io::{self, IsTerminal, Write},
    path::PathBuf,
    process::ExitCode,
};

#[derive(Debug, Parser)]
#[command(
    name = "rf",
    version,
    about = "Copy paths or file references to the clipboard; archive directories as tar files"
)]
struct Args {
    /// Copy a file reference instead of path text (the default for rff)
    #[arg(short = 'f', long)]
    file: bool,
    /// Allow directory archives of 512 MiB or more, or 10,000 entries or more
    #[arg(long, conflicts_with = "clean_archives")]
    allow_large: bool,
    /// List stored archives; add --yes to delete them
    #[arg(long, conflicts_with_all = ["path", "file"])]
    clean_archives: bool,
    /// Delete archives instead of only listing them
    #[arg(long, requires = "clean_archives")]
    yes: bool,
    /// Path to copy (default: current directory); use -- before paths starting with -
    path: Option<PathBuf>,
}

pub fn main_entry(default_file: bool) -> ExitCode {
    let args = Args::parse();
    match run(args, default_file) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rf: {error}");
            ExitCode::from(if matches!(error, Error::Cancelled) {
                130
            } else {
                1
            })
        }
    }
}
fn run(args: Args, default_file: bool) -> crate::core::Result<()> {
    let store = ArchiveStore::system()?;
    if args.clean_archives {
        eprintln!(
            "Deleting archives invalidates any clipboard references to them. {}",
            if args.yes {
                "Deleting archives."
            } else {
                "Preview only; add --yes to delete."
            }
        );
        for path in store.clean(args.yes)? {
            println!("{}", path.display());
        }
        return Ok(());
    }
    let path = args.path.unwrap_or(
        std::env::current_dir()
            .map_err(|e| Error::Message(format!("Cannot determine the current directory: {e}")))?,
    );
    let mode = if args.file || default_file {
        CopyMode::File
    } else {
        CopyMode::Text
    };
    let cancel = Cancellation::default();
    let signal = cancel.clone();
    ctrlc::set_handler(move || signal.cancel())
        .map_err(|e| Error::Message(format!("Cannot register the interrupt handler: {e}")))?;
    let mut last = std::time::Instant::now();
    let request = CopyRequest {
        path,
        mode,
        allow_large: args.allow_large,
    };
    let prepared = crate::core::prepare(request, &store, &cancel, |p| {
        if io::stderr().is_terminal() && last.elapsed().as_millis() > 150 {
            eprint!("\rArchiving: {} entries / {} bytes", p.entries, p.bytes);
            last = std::time::Instant::now();
        }
    })?;
    let result = prepared.commit(&mut crate::clipboard::SystemClipboard, &cancel)?;
    writeln!(io::stdout().lock(), "{}", result.copied_path.display()).map_err(|e| {
        Error::Message(format!(
            "Clipboard updated, but cannot write the result path to stdout: {e}"
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arguments_preserve_paths_and_reject_conflicts() {
        assert_eq!(
            Args::try_parse_from(["rf", "tui"]).unwrap().path,
            Some("tui".into())
        );
        assert_eq!(
            Args::try_parse_from(["rf", "--", "-f"]).unwrap().path,
            Some("-f".into())
        );
        for argv in [
            vec!["rf", "a", "b"],
            vec!["rf", "--tui", "--gui"],
            vec!["rf", "--clean-archives", "a"],
            vec!["rf", "--yes"],
            vec!["rf", "--clean-archives", "-f"],
            vec!["rf", "--clean-archives", "--allow-large"],
        ] {
            assert!(Args::try_parse_from(argv).is_err());
        }
        assert!(
            Args::try_parse_from(["rff", "--allow-large", "large"])
                .unwrap()
                .allow_large
        );
    }
}
