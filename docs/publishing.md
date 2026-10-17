# Publishing to crates.io

Publish one GPL-3.0-only-licensed package, `rf-copy`, containing the `rf` and `rff` commands. Internal code is organized into modules, so no separate internal crates need to be published.

Users install it with:

```sh
cargo install rf-copy --locked
```

The package and command names differ because `rf` and `rf-cli` are already used by other crates.io projects.

## Prepare a release

Update the version in `Cargo.toml` and refresh `Cargo.lock`, then run:

```sh
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo publish --dry-run --locked
cargo package --list --locked
```

Inspect the package file list. It should contain source, tests, manifests, the lockfile, license, README, documentation, and smoke-test scripts. OpenSpec files, Codex configuration, build output, credentials, and user archives must remain outside the package.

Cargo's dry run builds the extracted package to verify that it works independently of the checkout. Also install from that extracted source and run the smoke test:

```sh
# Replace the version below when preparing another release.
cargo install --path target/package/rf-copy-0.1.2 --locked --root target/cargo-install-check
python3 scripts/smoke-macos.py --bin-dir target/cargo-install-check/bin
```

## Publish and verify

Use your crates.io account and local Cargo credentials. Run `cargo login` in a terminal if needed; keep tokens out of source files and documentation.

```sh
cargo publish --locked
```

After publication, verify installation from the registry:

```sh
cargo install rf-copy --version 0.1.2 --locked --root target/crates-io-install-check
target/crates-io-install-check/bin/rf --version
target/crates-io-install-check/bin/rff --version
python3 scripts/smoke-macos.py --bin-dir target/crates-io-install-check/bin
```

The source repository is [www159-used/rf](https://github.com/www159-used/rf). GitHub hosts the source and CI; crates.io distributes the CLI.

Version 0.1.2 uses `GPL-3.0-only` and includes the full GPLv3 text. Previously published versions retain their original license.

A published version cannot be overwritten. Use a new version for subsequent changes. CI runs checks and a dry run; it does not hold a publication token or publish automatically.

Reference: [Publishing on crates.io — The Cargo Book](https://doc.rust-lang.org/cargo/reference/publishing.html).
