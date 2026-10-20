# rf-copy

[![CI](https://github.com/www159-used/rf/actions/workflows/macos.yml/badge.svg)](https://github.com/www159-used/rf/actions/workflows/macos.yml)

Copy a path or file reference to the macOS clipboard from your terminal.

```sh
rf ./report.txt    # Copy the absolute path as text
rff ./report.txt   # Copy the file reference for pasting into Finder
rff ./project     # Archive the directory as tar, then copy the archive reference
```

## Install

```sh
cargo install rf-copy --locked
```

The package is named `rf-copy`; it installs two commands, `rf` and `rff`.

Requirements: macOS, Rust 1.95 or later, and Xcode Command Line Tools. Tested on Apple Silicon with macOS 26.1. Windows and Linux are not supported yet.

Cargo installs commands into its bin directory, usually `~/.cargo/bin`. Make sure that directory is on your `PATH`. If you already have shell functions or aliases named `rf` or `rff`, check `type -a rf rff` or invoke `~/.cargo/bin/rf` directly. This tool does not change your shell configuration. Cargo will report a conflict if another installed package provides the same command names.

To install from source:

```sh
cargo install --path . --locked
```

## Usage

```sh
rf                     # Copy the current directory's absolute path
rf ./some-file         # Copy an absolute path as text
rf -f ./some-file      # Copy a file reference
rff ./some-folder      # Create a tar archive and copy its file reference
rf -- -file.txt        # Handle a path that starts with a dash
```

`rff` is equivalent to `rf -f`. Both commands use the current directory when no path is given.

On success, stdout contains the absolute path that was copied, followed by a newline. For a directory in file mode, this is the generated tar path. Clipboard text has no added newline. Errors go to stderr and return a nonzero exit code. A failed file copy never silently falls back to path text.

Paths may contain Unicode characters, spaces, quotes, or newlines. Relative paths resolve against the working directory. A symbolic link passed as the input resolves to its target; links inside an archived directory are preserved without following them. Only existing regular files and directories are accepted.

Press Ctrl-C to cancel directory archiving. Incomplete archives are removed. Once the clipboard write begins, that short step cannot be cancelled.

| Exit code | Meaning |
| --- | --- |
| `0` | Success |
| `1` | Operation failed |
| `2` | Invalid arguments |
| `130` | Cancelled before clipboard submission |

## Archive storage

Completed archives remain in `~/Library/Application Support/rf/archives/` until you delete them explicitly:

```sh
rf --clean-archives        # Preview the archive directories to delete
rf --clean-archives --yes  # Delete them
```

Deleting an archive invalidates any clipboard reference to it. Cleanup leaves source files untouched and refuses to run while an archive job is active. The archive store is excluded when archiving a directory that contains it.

Set `RF_DATA_DIR` to an absolute path to use a different data directory, for example during testing.

## Update or uninstall

```sh
cargo install rf-copy --locked  # Install a newer version, if available
cargo uninstall rf-copy
```

Uninstalling the commands leaves stored archives in place. Run archive cleanup first if you want to remove them too.

## Development

```sh
make help           # List development commands
make check          # Formatting, Clippy, and tests
make build-release  # Build optimized rf and rff binaries
make package        # Verify the crates.io package
```

To test against the real macOS clipboard:

```sh
make smoke
```

The smoke test temporarily changes the clipboard and restores its readable contents when finished. Avoid other clipboard activity during the test. Native format reads have been verified; actual Finder paste remains a separate manual check. See the validation and publishing notes in the package's `docs/` directory.

GitHub CI runs checks and verifies installation on Apple Silicon and Intel macOS. Maintainers use [cargo-release](docs/publishing.md) to publish to crates.io and push version tags.

Run `make tools` to install cargo-release, `make release` to preview the prepared version, and `make publish` to publish it. For a version bump, use the same `RELEASE_LEVEL=patch` (or `minor` / `major`) with both commands. `make publish` also creates a Git tag and pushes to `origin`.

## License

Licensed under the GNU General Public License, version 3 only (`GPL-3.0-only`). See [LICENSE](LICENSE) for the full license text.
