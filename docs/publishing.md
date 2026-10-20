# Publishing to crates.io

`rf-copy` is a GPL-3.0-only package containing the `rf` and `rff` commands. Users install it with `cargo install rf-copy --locked`. The package name differs from the commands because `rf` and `rf-cli` are already used by other crates.io projects.

Source and CI live in [www159-used/rf](https://github.com/www159-used/rf). Maintainers publish from a local macOS checkout using cargo-release; CI validates changes without publishing credentials.

## Set up cargo-release

Install the tested version as a development tool, separate from the application's dependencies:

```sh
make tools
```

Use your existing local Cargo credentials, or run `cargo login` in a terminal. Keep tokens out of the repository.

The root `release.toml` permits releases from `main`, runs `make check` before committing, verifies the packaged crate, publishes to crates.io, creates an annotated version tag, and pushes the branch and tag to `origin`. Tags default to `v0.1.2` for the root crate; a future workspace member uses a crate-prefixed tag such as `rf-copy-v0.1.3`.

## Preview and release

Start with a clean checkout on `main`, synchronize with `git pull --ff-only`, and confirm the commit's GitHub CI passed. `make release` invokes cargo-release in dry-run mode. Review its output before running `make publish`, which passes `--execute`.

The current `0.1.2` version is already prepared in the manifest. Publish it without another version bump:

```sh
make release
make publish
```

For the next patch after that release, let cargo-release update both the manifest and lockfile:

```sh
make release RELEASE_LEVEL=patch
make publish RELEASE_LEVEL=patch
```

Set `RELEASE_LEVEL=minor` or `RELEASE_LEVEL=major` when appropriate. The release hook runs formatting, Clippy with warnings treated as errors, and workspace tests. Cargo then builds the packaged source before upload. A published version cannot be overwritten.

## Package and clipboard checks

To inspect the package independently of a release:

```sh
make check
make package
cargo package --list --locked
version=$(python3 -c 'import tomllib; print(tomllib.load(open("Cargo.toml", "rb"))["package"]["version"])')
cargo install --path "target/package/rf-copy-$version" --locked --root target/cargo-install-check
python3 scripts/smoke-macos.py --bin-dir target/cargo-install-check/bin
```

The package includes source, tests, manifests, the lockfile, license, README, documentation, the Makefile, and validation scripts. Release configuration, OpenSpec records, Codex configuration, build output, credentials, and user archives stay outside the published package.

After publication, verify the registry version in a separate installation directory:

```sh
cargo install rf-copy --version "$version" --locked --root target/crates-io-install-check
target/crates-io-install-check/bin/rf --version
target/crates-io-install-check/bin/rff --version
python3 scripts/smoke-macos.py --bin-dir target/crates-io-install-check/bin
```

Set `version` from the manifest as shown above if using a new shell. Clipboard smoke checks require a logged-in macOS desktop and temporarily change the clipboard; actual Finder paste remains a manual check.

## GitHub CI

`.github/workflows/macos.yml` runs on pushes to `main`, version tags, pull requests, and manual dispatch. Both `macos-15` (Apple Silicon) and `macos-15-intel` run the pinned toolchain from `rust-toolchain.toml`, `make check`, `make package`, and installation checks for both commands. Cargo dependencies and build output are cached; a newer run cancels an older run for the same ref.

CI uses read-only repository permissions. Publishing stays with the local cargo-release command, so no crates.io token is needed in GitHub Actions.

## Future workspace

Cargo-release supports selecting a member with `cargo release -p rf-copy patch`. When moving this crate into a workspace, keep shared release policy at the workspace root and put crate-specific hooks in the member's `release.toml` or `[package.metadata.release]`. Update script paths and CI package installation paths during that migration. Independent crate versions and prefixed tags can coexist.

Version 0.1.2 uses `GPL-3.0-only` and includes the full GPLv3 text. Previously published versions retain their original license.

References: [cargo-release configuration](https://github.com/crate-ci/cargo-release/blob/v1.1.2/docs/reference.md), [workspace configuration](https://github.com/crate-ci/cargo-release/blob/v1.1.2/docs/faq.md), [Cargo publishing](https://doc.rust-lang.org/cargo/reference/publishing.html).
