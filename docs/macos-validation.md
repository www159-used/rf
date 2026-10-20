# macOS CLI validation

Environment: macOS 26.1 (25B78), Apple Silicon arm64, Rust 1.95.0.

## Automated coverage

Rust tests cover argument parsing, path resolution, text and file modes, tar contents, symbolic links, exclusion of archive storage, permission and special-file errors, cancellation cleanup, unique archive names, cleanup locking, and filesystem boundaries.

`scripts/smoke-macos.py` runs against the real clipboard with separate writer and reader processes. Its 12 checks cover:

1. The current directory when no path is given.
2. Unicode, whitespace, quotes, newlines, and URI-special characters.
3. Resolution of a symbolic link passed as input.
4. The `--` option terminator.
5. Repeated file-URL reads after the writer exits.
6. Equivalence of `rff` and `rf -f`.
7. Directory archive contents, including empty directories and symbolic links.
8. Preservation of the clipboard when path resolution fails.
9. Concurrent archive creation.
10. Large-directory refusal in both commands, preserving the clipboard and stored archives.
11. Cancellation with a real SIGINT after explicitly allowing a large archive.
12. Cleanup preview and deletion.

The smoke test uses a temporary data directory and restores the clipboard contents it could read before the test.

## Manual Finder checks

This machine denied the Apple Events permission needed to automate System Events (`-1743`). Finder was not automated and system permissions were not changed. Reading a native file URL does not establish that pasting into Finder works.

1. Run `rf -f /absolute/path/to/test-file`. After the command exits, press Command-V in another empty Finder directory. Check the copied contents and confirm that the source file still exists.
2. Run `rff /absolute/path/to/test-directory`. Paste after the command exits. Check that `archive.tar` appears and contains the expected files when extracted.

## File-reference regression

On this machine, a process that exited immediately after `NSPasteboard.writeObjects` returned success could leave the next process unable to read the file URL. A Swift comparison program also exhibited the behavior.

The adapter initializes NSPasteboard, writes an NSURL, then synchronously reads and checks `public.file-url` while the URL object is still alive. Only then does it report success. A lock serializes clipboard submission and verification between rf processes so concurrent copies cannot interfere with that check.

## Platform status

| Platform | Build | Native clipboard reads after exit | File-manager paste |
| --- | --- | --- | --- |
| macOS 26.1 arm64 | Passed locally | Passed | Finder: manual check pending |
| Other macOS versions / Intel | Not verified | Not verified | Not verified |
| Windows / Linux | Not implemented | Not verified | Not verified |

## Release baseline

For version 0.1.0, formatting, Clippy with warnings treated as errors, and all 14 Rust tests passed. The package built independently, installed through Cargo, and passed all 11 clipboard smoke checks, including after installation from crates.io.

Version 0.1.1 changes the CLI's own help, messages, and documentation to English. Unicode path fixtures remain in the tests to check that language changes do not alter file handling. Operating-system error details and user-provided paths may still contain other languages.
