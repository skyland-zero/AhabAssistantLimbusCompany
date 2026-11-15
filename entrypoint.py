"""Single frozen entry point for the sidecar and the one-shot Runner.

The shared runtime ships one ``AALC Backend.exe``.  The sidecar starts it with
no role flag; the execution supervisor appends ``--execution-runner`` when it
needs a Runner.  Both branches import their heavy modules lazily so the Runner
still avoids importing task or configuration modules before the ``start``
frame, exactly as the dedicated ``runner_bootstrap.py`` entry did.
"""

from __future__ import annotations

import sys

RUNNER_FLAG = "--execution-runner"


def main() -> int:
    if len(sys.argv) > 1 and sys.argv[1] == RUNNER_FLAG:
        from runner_bootstrap import main as runner_main

        return runner_main(sys.argv[2:])

    from main_backend import main as backend_main

    backend_main()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
