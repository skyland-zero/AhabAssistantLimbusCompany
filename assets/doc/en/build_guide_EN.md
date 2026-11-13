# Build guide

## Environment

- Windows x64
- Python 3.12+
- Rust 1.99.1
- Native Shell 0.5.0 (`nsh` on `PATH`)
- `uv` (recommended) and 7-Zip

Install dependencies:

```powershell
uv sync --frozen
rustup toolchain install 1.99.1
irm https://nsh.dev/install.ps1 | iex
```

## Run from source

```powershell
.\run-nsh.bat
```

This starts the Native Shell window; the Rust frontend launches `main_backend.py`
from the same checkout. For visual-only development or when no device is
available, run the Mock launcher from the repository root:

```powershell
.\run-nsh-mock.bat
```

That launcher does not start the Python backend. Production runs must not use
Mock. On Linux, `./run-nsh.sh` (and `./run-nsh.sh --mock`) does the same.

## Build a release package

```powershell
uv run python scripts/build.py --version 1.0.0
```

The build runs, in order:

1. `nsh package nsh-app --target windows` for the Native Shell frontend;
2. `main_backend.spec` for the headless `AALC Backend.exe`;
3. `updater.spec` for `AALC Updater.exe`;
4. staging of `dist/AALC/` with `nsh/`, `assets/`, the README and the LICENSE;
5. `dist/AALC_<version>.7z` through 7-Zip.

Without 7-Zip, an equivalent `.zip` archive is produced. A release package does
not require Python, Qt or WebView2 at runtime. Before the first start, check that
`AALC.exe`, `nsh/`, `backend/AALC Backend.exe` and `AALC Updater.exe` are in
`dist/AALC/`.
