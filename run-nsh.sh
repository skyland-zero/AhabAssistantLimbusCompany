#!/bin/sh
# Linux/macOS launcher for the Native Shell app; run-nsh.bat is the Windows one.
#
#   ./run-nsh.sh            # real backend (python sidecar)
#   ./run-nsh.sh --mock     # deterministic mock, no Python needed
#   ./run-nsh.sh --target gpui
#
# Builds are IO- and RAM-heavy: on an HDD the build itself is what makes the
# machine unresponsive, so this caps cargo's parallelism (scripts/build-env.sh)
# and runs the build at a lower IO/CPU priority than everything else.
#
#   AALC_JOBS=6          more rustc jobs, if the machine can take it
#   AALC_LINKER=mold     link with mold/lld instead of the default linker
#   AALC_DRY_RUN=1       print the environment and command, run nothing

set -eu
cd "$(dirname "$0")"

if [ -x .venv/bin/python ]; then
    AHAB_PYTHON=$PWD/.venv/bin/python
    export AHAB_PYTHON
fi

case ${1:-} in
    --mock)
        AHAB_BACKEND=mock
        export AHAB_BACKEND
        shift
        ;;
esac

. scripts/build-env.sh

set -- nsh run nsh-app "$@"

if [ "${AALC_DRY_RUN:-}" = 1 ]; then
    printf 'CARGO_BUILD_JOBS=%s\nRUSTFLAGS=%s\n' "$CARGO_BUILD_JOBS" "${RUSTFLAGS:-}"
    printf 'AHAB_BACKEND=%s\n' "${AHAB_BACKEND:-sidecar}"
    printf '+ %s\n' "$*"
    exit 0
fi

if command -v ionice >/dev/null 2>&1; then
    exec ionice -c 2 -n 6 nice -n 5 "$@"
fi
exec nice -n 5 "$@"
