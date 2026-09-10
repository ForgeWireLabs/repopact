# Guide: Install and build the RepoPact engine

*Diataxis mode: how-to (task-oriented).*

RepoPact wheels contain the Python compatibility package and a platform-native
`repopact-engine` executable. A wheel install is therefore compiler-free, while
an sdist build is a source build that requires Rust and Maturin.

## Install a wheel

```powershell
python -m venv .venv
.venv\Scripts\python -m pip install repopact
.venv\Scripts\repopact --help
```

The compatibility client only trusts `repopact-engine` beside the active
Python environment's scripts directory. `REPOPACT_ENGINE` is an explicit
development override and is still handshake- and version-checked. It does not
fall back to a Python validator when the engine is missing or incompatible.

## Build from source

Use a supported Rust toolchain with Cargo and Maturin 1.15 or newer but below
2.0. The repository's current proof was run with Rust/Cargo 1.98.0 and Maturin
1.15.0:

```powershell
python -m pip install "maturin>=1.15,<2"
# Verify the locked workspace before Maturin's source-export build.  Maturin's
# combined --sdist build performs its own source export and does not accept
# --locked for that second Cargo invocation.
cargo metadata --format-version 1 --locked --manifest-path rust/Cargo.toml
python -m maturin build --sdist --out dist
python -m pip install dist\repopact-*.tar.gz
```

Cargo dependencies must be available in the local cache or fetched from the
network. Source builds are not compiler-free; the resulting wheel can be
installed without Rust.

## Roll back an executable

Rollback is package-level: reinstall a previously compatible RepoPact wheel or
switch the development environment to the previous committed source revision.
No repository record conversion, Git-history rewrite, or durable transaction
state change is required. WI050 admission/guard operations remain on their
existing protected Python/provider path.
