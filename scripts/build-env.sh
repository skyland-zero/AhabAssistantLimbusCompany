# Build settings for machines where the disk is the bottleneck (an HDD) or RAM is
# tight — source it, do not run it: `run-nsh.sh` sources it, and so can you:
#
#     . scripts/build-env.sh && cargo test --manifest-path nsh-app/native/Cargo.toml
#
# `.cargo/config.toml` already picks the target directory and `debuginfo=1`, but
# it cannot cap cargo's parallelism or choose a linker. Eight rustc processes
# reading and writing the same platter is what makes an HDD build stall (and, with
# ~10 GB of RAM, push the linker into swap).
#
#   AALC_JOBS=4        rustc jobs (default 4, clamped to `nproc`)
#   AALC_LINKER=...    linker for `cc` to drive: mold, lld, or default
#                      (unset: mold when installed, else lld when installed,
#                      else `cc`'s own linker; `default` always means the latter)
#   AALC_DEBUGINFO=2   full debug info for this session (variables in a debugger)
#
# RUSTFLAGS, when already set, is left alone: it replaces the config file's
# rustflags wholesale, so whoever set it owns the debug-info level and linker.
#
# Measured on this checkout (relink of the gpui dev host after a one-line Rust
# edit): 2m08s with GNU ld, 1m18s with mold. mold writes a ~20% larger binary,
# so the gain is smaller than its CPU advantage alone would suggest.

if [ -z "${CARGO_BUILD_JOBS:-}" ]; then
    jobs=${AALC_JOBS:-4}
    nproc_count=$(nproc 2>/dev/null || echo 4)
    [ "$nproc_count" -lt "$jobs" ] && jobs=$nproc_count
    CARGO_BUILD_JOBS=$jobs
    export CARGO_BUILD_JOBS
fi

if [ -z "${RUSTFLAGS:-}" ]; then
    linker=${AALC_LINKER:-}
    if [ -z "$linker" ]; then
        if command -v mold >/dev/null 2>&1; then
            linker=mold
        elif command -v ld.lld >/dev/null 2>&1; then
            linker=lld
        fi
    fi
    [ "$linker" = default ] && linker=

    RUSTFLAGS="-C debuginfo=${AALC_DEBUGINFO:-1}"
    if [ -n "$linker" ]; then
        RUSTFLAGS="$RUSTFLAGS -C link-arg=-fuse-ld=$linker"
    fi
    export RUSTFLAGS
fi

# Sourced by scripts with `set -e`: leave a successful status behind.
:
