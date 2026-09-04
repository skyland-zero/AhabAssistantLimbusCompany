"""Process-isolated task execution primitives.

The package intentionally keeps its import surface small.  In particular, importing
``module.execution`` must not import the application's configuration, device
manager, or task modules.  ``runner_bootstrap.py`` relies on this property while
it is still performing its pre-handshake work.
"""

from .cleanup_ledger import (
    CleanupActionExecutor,
    CleanupActionResult,
    CleanupActionState,
    CleanupExecutor,
    CleanupLedger,
    CleanupLedgerError,
    CleanupRecoveryResult,
)
from .event_adapter import BackendEventAdapter, ExecutionEventAdapter, RunnerEventAdapter
from .execution_control import ExecutionCancelled, ExecutionControl
from .ipc_protocol import (
    COMMAND_TYPES,
    EVENT_TYPES,
    PROTOCOL_VERSION,
    Frame,
    FrameCodec,
    FrameEOF,
    FrameTruncated,
    ProtocolError,
    encode_frame,
    read_frame,
    write_frame,
)
from .mediator_bridge import ExecutionMediatorBridge, MediatorBridge, MediatorEventBridge, RunnerMediatorBridge

__all__ = [
    "COMMAND_TYPES",
    "CleanupActionExecutor",
    "CleanupActionResult",
    "CleanupActionState",
    "CleanupExecutor",
    "CleanupLedger",
    "CleanupLedgerError",
    "CleanupRecoveryResult",
    "EVENT_TYPES",
    "ExecutionCancelled",
    "ExecutionControl",
    "ExecutionEventAdapter",
    "ExecutionMediatorBridge",
    "Frame",
    "FrameCodec",
    "FrameEOF",
    "FrameTruncated",
    "BackendEventAdapter",
    "MediatorBridge",
    "MediatorEventBridge",
    "PROTOCOL_VERSION",
    "ProtocolError",
    "RunnerEventAdapter",
    "RunnerMediatorBridge",
    "encode_frame",
    "read_frame",
    "write_frame",
]
