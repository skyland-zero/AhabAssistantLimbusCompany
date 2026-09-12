"""Shared test isolation.

The sidecar resolves its configuration, runtime statistics, theme-pack state
and cleanup journals from ``AALC_CONFIG_PATH`` (plus the theme-pack path
overrides).  Point every test process at a per-session temporary directory
*before* any test module can import the process-wide config singleton, so the
suite can never rewrite the developer's real ``config.yaml``,
``runtime_stats.json`` or ``.aalc-runs`` journals.
"""

from __future__ import annotations

import os
import tempfile
from pathlib import Path

_ISOLATION_ROOT = Path(tempfile.mkdtemp(prefix="aalc-pytest-"))

# Force the override instead of ``setdefault``: a stale inherited value would
# silently point the suite back at real user state.
os.environ["AALC_CONFIG_PATH"] = str(_ISOLATION_ROOT / "config.yaml")
os.environ["AALC_THEME_PACK_LIST_PATH"] = str(_ISOLATION_ROOT / "theme_pack_list.yaml")
os.environ["AALC_THEME_PACK_WEIGHT_PATH"] = str(_ISOLATION_ROOT / "theme_pack_weight")
