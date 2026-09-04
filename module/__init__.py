"""Paths that are safe to use before the rest of :mod:`module` is imported.

The runner bootstrap sets ``AALC_CONFIG_PATH`` immediately before importing
this package.  Keep resolving the value here (rather than in
``module.config``) so importing ``module.config`` cannot create a singleton
for the parent process' configuration first.
"""

import os
from pathlib import Path

_APPLICATION_ROOT = Path(__file__).resolve().parent.parent
_CONFIG_ENV_PATH = os.environ.get("AALC_CONFIG_PATH")

VERSION_PATH = str((_APPLICATION_ROOT / "assets" / "config" / "version.txt").resolve())
EXAMPLE_PATH = str((_APPLICATION_ROOT / "assets" / "config" / "config.example.yaml").resolve())
# ``Path.resolve`` also normalizes a relative environment override.  The
# default intentionally resolves against the application root, not whatever
# directory happened to be current when a frozen runner was launched.
CONFIG_PATH = str(
    Path(_CONFIG_ENV_PATH).expanduser().resolve()
    if _CONFIG_ENV_PATH
    else (_APPLICATION_ROOT / "config.yaml").resolve()
)
THEME_PACK_LIST_EXAMPLE_PATH = str(
    (_APPLICATION_ROOT / "assets" / "config" / "theme_pack_list.example.yaml").resolve()
)
THEME_PACK_LIST_PATH = str((_APPLICATION_ROOT / "theme_pack_list.yaml").resolve())
THEME_PACK_WEIGHT_PATH = str((_APPLICATION_ROOT / "theme_pack_weight").resolve())
