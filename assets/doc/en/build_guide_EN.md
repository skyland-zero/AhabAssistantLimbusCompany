# Build guide

## Environment

- Windows x64
- Python 3.12+
- Rust nightly (required by the current GPUI source)
- `uv` (recommended) and 7-Zip

Install dependencies:

```powershell
uv sync --frozen
rustup toolchain install nightly
```

## Run from source

```powershell
.\run-gpui.bat
```

The GPUI window starts and launches `main_backend.py` from the same checkout.
For visual-only development or when no device is available, run the Mock
launcher from the repository root:

```powershell
.\run-gpui-mock.bat
```

The launcher does not start the Python backend. Production mode must not use
Mock.

## Build a release package

```powershell
uv run python scripts/build.py --version 1.0.0
```

The build compiles the GPUI frontend, creates `AALC Backend.exe` from
`main_backend.spec`, creates `AALC Updater.exe`, stages the `assets/` and
`resources/` trees, and produces `dist/AALC_<version>.7z`.

Without 7-Zip, an equivalent `.zip` archive is produced. A release package
does not require Python, Qt, Node, or WebView2 at runtime.
