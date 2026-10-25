"""Shared constants for the GPUI ↔ sidecar JSON-RPC handshake.

``contract/contract.json`` is the reviewable source of truth.  This module
holds the runtime values and is checked against the manifest by
``scripts/validate_contract.py`` so the handshake cannot drift silently.
"""

from __future__ import annotations

SCHEMA_VERSION = 3
PRIMARY_CAPABILITY = "core.rpc.v3"
CAPABILITIES = (
    PRIMARY_CAPABILITY,
    "app.shutdown",
    "app.diagnose",
    "event.bootId",
    "telemetry.backpressure",
)

__all__ = ["CAPABILITIES", "PRIMARY_CAPABILITY", "SCHEMA_VERSION"]
