@echo off
rem Rust build settings shared by run-nsh.bat and run-nsh-mock.bat.
rem
rem Called, not run on its own: it only sets variables in the caller.
rem
rem `.cargo\config.toml` already asks rustc for line-tables-only debug info, but
rem that file cannot pick a linker. The `dev` hosts are several hundred megabytes
rem of debug info, and linking them is what makes `nsh run` slow after a Rust
rem edit; lld-link links them considerably faster than MSVC's link.exe.
rem
rem   AALC_LINKER=default      keep link.exe
rem   AALC_LINKER=<path>       use that linker (no spaces in the path), e.g.
rem                            C:\Program Files\...\VC\Tools\Llvm\x64\bin\lld-link.exe
rem   AALC_LINKER unset        use lld-link when it is on PATH, else link.exe
rem   AALC_DEBUGINFO=2         full debug info for this session (variables in a debugger)
rem
rem RUSTFLAGS, when already set, is left alone: it replaces the config file's
rem rustflags wholesale, so whoever set it owns the debug-info level and linker.

if defined RUSTFLAGS goto :build_env_done

if not defined AALC_DEBUGINFO set "AALC_DEBUGINFO=1"
set "RUSTFLAGS=-C debuginfo=%AALC_DEBUGINFO%"

if /i "%AALC_LINKER%"=="default" goto :build_env_done
set "AALC_LLD=%AALC_LINKER%"
if not defined AALC_LLD where lld-link >nul 2>&1 && set "AALC_LLD=lld-link"
if not defined AALC_LLD goto :build_env_done
set "RUSTFLAGS=%RUSTFLAGS% -C linker=%AALC_LLD%"

:build_env_done
