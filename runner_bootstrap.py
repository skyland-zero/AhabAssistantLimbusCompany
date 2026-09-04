"""Minimal, one-shot Runner entry point.

Only standard-library modules and a tiny local frame codec are used before the
sidecar sends ``start``.  This is important: importing task modules (or
the application's configuration singleton) before ``AALC_CONFIG_PATH`` is set can
bind the Runner to the wrong config file.

The development invocation is:

    python runner_bootstrap.py --run-id <id> --protocol 1 \
        --expected-parent-pid <sidecar-pid>

The sidecar normally wires command/event/stderr through standard handles on
Windows and supplies explicit fd arguments on POSIX.
"""

from __future__ import annotations

import argparse
import json
import os
import signal
import struct
import sys
import traceback
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import Any

_BOOTSTRAP_MAX_TOTAL_LENGTH = 8 * 1024 * 1024
_BOOTSTRAP_MAX_HEADER_LENGTH = 64 * 1024
_U32 = struct.Struct("!I")


class _BootstrapProtocolError(ValueError):
    pass


@dataclass(frozen=True, slots=True)
class _BootstrapFrame:
    header: dict[str, Any]
    payload: bytes = b""


def _bootstrap_read_exact(stream: Any, size: int) -> bytes:
    chunks: list[bytes] = []
    remaining = size
    while remaining:
        chunk = stream.read(remaining)
        if chunk is None or not chunk:
            raise _BootstrapProtocolError("truncated Runner frame")
        chunk = bytes(chunk)
        if len(chunk) > remaining:
            raise _BootstrapProtocolError("Runner stream returned too many bytes")
        chunks.append(chunk)
        remaining -= len(chunk)
    return b"".join(chunks)


def _bootstrap_encode(header: Mapping[str, Any], payload: bytes = b"") -> bytes:
    if not isinstance(header, Mapping):
        raise _BootstrapProtocolError("frame header must be an object")
    value = dict(header)
    declared = value.get("binaryLength", len(payload))
    if isinstance(declared, bool) or not isinstance(declared, int) or declared < 0 or declared != len(payload):
        raise _BootstrapProtocolError("binaryLength does not match payload")
    value["binaryLength"] = declared
    try:
        encoded = json.dumps(value, ensure_ascii=False, allow_nan=False, separators=(",", ":"), sort_keys=True).encode("utf-8")
    except (TypeError, ValueError, UnicodeError) as exc:
        raise _BootstrapProtocolError(f"invalid frame header: {exc}") from exc
    total = 4 + len(encoded) + len(payload)
    if len(encoded) > _BOOTSTRAP_MAX_HEADER_LENGTH or total > _BOOTSTRAP_MAX_TOTAL_LENGTH:
        raise _BootstrapProtocolError("Runner frame exceeds protocol limit")
    return _U32.pack(total) + _U32.pack(len(encoded)) + encoded + payload


def _bootstrap_read_frame(stream: Any) -> _BootstrapFrame | None:
    prefix = stream.read(4)
    if prefix is None:
        raise _BootstrapProtocolError("Runner stream returned no data")
    prefix = bytes(prefix)
    if not prefix:
        return None
    if len(prefix) < 4:
        prefix += _bootstrap_read_exact(stream, 4 - len(prefix))
    if len(prefix) != 4:
        raise _BootstrapProtocolError("truncated Runner length")
    total = _U32.unpack(prefix)[0]
    if total < 4 or total > _BOOTSTRAP_MAX_TOTAL_LENGTH:
        raise _BootstrapProtocolError("Runner frame length is invalid")
    body = _bootstrap_read_exact(stream, total)
    header_length = _U32.unpack(body[:4])[0]
    if header_length > _BOOTSTRAP_MAX_HEADER_LENGTH or header_length > total - 4:
        raise _BootstrapProtocolError("Runner header length is invalid")
    try:
        header = json.loads(body[4 : 4 + header_length].decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise _BootstrapProtocolError("Runner header is not valid JSON") from exc
    if not isinstance(header, dict):
        raise _BootstrapProtocolError("Runner header must be an object")
    payload = body[4 + header_length :]
    declared = header.get("binaryLength", len(payload))
    if isinstance(declared, bool) or not isinstance(declared, int) or declared < 0 or declared != len(payload):
        raise _BootstrapProtocolError("binaryLength does not match payload")
    return _BootstrapFrame(header, bytes(payload))


def _bootstrap_write(stream: Any, data: bytes) -> None:
    offset = 0
    while offset < len(data):
        written = stream.write(data[offset:])
        if written is None:
            written = len(data) - offset
        if (
            isinstance(written, bool)
            or not isinstance(written, int)
            or written <= 0
            or written > len(data) - offset
        ):
            raise _BootstrapProtocolError("Runner stream made no write progress")
        offset += written
    flush = getattr(stream, "flush", None)
    if callable(flush):
        flush()


class _BootstrapWriter:
    def __init__(self, stream: Any) -> None:
        self.stream = stream

    def send(self, header: Mapping[str, Any], payload: bytes = b"") -> None:
        _bootstrap_write(self.stream, _bootstrap_encode(header, payload))


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Ahab Assistant one-shot execution Runner")
    parser.add_argument("--run-id", required=True)
    parser.add_argument("--protocol", type=int, default=1)
    parser.add_argument("--expected-parent-pid", type=int, default=0)
    parser.add_argument("--command-fd", type=int, default=-1)
    parser.add_argument("--event-fd", type=int, default=-1)
    parser.add_argument("--stderr-fd", type=int, default=-1)
    # Windows launchers may prefer the explicit handle spelling.  The values are
    # consumed identically; argparse keeps this module useful for direct smoke
    # tests on both platforms.
    parser.add_argument("--command-handle", type=int, default=-1)
    parser.add_argument("--event-handle", type=int, default=-1)
    parser.add_argument("--stderr-handle", type=int, default=-1)
    return parser


def _install_parent_death_signal(expected_parent_pid: int) -> None:
    """Install Linux PDEATHSIG before any business import and recheck the parent."""

    if not sys.platform.startswith("linux"):
        return
    if expected_parent_pid > 0:
        try:
            import ctypes

            libc = ctypes.CDLL(None, use_errno=True)
            prctl = libc.prctl
            prctl.argtypes = [ctypes.c_int, ctypes.c_ulong, ctypes.c_ulong, ctypes.c_ulong, ctypes.c_ulong]
            prctl.restype = ctypes.c_int
            # Linux PR_SET_PDEATHSIG = 1, SIGKILL is deliberately non-catchable.
            if prctl(1, signal.SIGKILL, 0, 0, 0) != 0:
                error = ctypes.get_errno()
                raise OSError(error, os.strerror(error))
        except (OSError, AttributeError) as exc:
            raise RuntimeError(f"cannot install parent-death signal: {exc}") from exc
        # The parent may have died between process creation and prctl.  This second
        # check closes that small race as required by the process contract.
        if os.getppid() != expected_parent_pid:
            raise RuntimeError("expected sidecar parent is no longer present")


def _fd_or_standard(fd: int, standard: Any, mode: str) -> Any:
    if fd is None or fd < 0:
        # ``sys.stdin.buffer`` is a BufferedReader.  A command-reader thread
        # blocked on that lock can make Python's interpreter shutdown fail after
        # a finished Runner.  Use a private unbuffered descriptor when possible;
        # the fallback is retained for test doubles without ``fileno``.
        try:
            return os.fdopen(os.dup(standard.fileno()), mode, buffering=0)
        except (AttributeError, OSError):
            return standard
    # Duplicate explicitly supplied descriptors so closing the bootstrap stream
    # cannot accidentally close the process's standard handle.
    duplicate = os.dup(fd)
    return os.fdopen(duplicate, mode, buffering=0)


def _open_streams(args: argparse.Namespace) -> tuple[Any, Any, Any]:
    command_fd = args.command_fd if args.command_fd >= 0 else args.command_handle
    event_fd = args.event_fd if args.event_fd >= 0 else args.event_handle
    stderr_fd = args.stderr_fd if args.stderr_fd >= 0 else args.stderr_handle
    command = _fd_or_standard(command_fd, getattr(sys.stdin, "buffer", sys.stdin), "rb")
    event = _fd_or_standard(event_fd, getattr(sys.stdout, "buffer", sys.stdout), "wb")
    stderr = _fd_or_standard(stderr_fd, getattr(sys.stderr, "buffer", sys.stderr), "wb")
    return command, event, stderr


def _require_absolute(value: Any, field: str) -> str | None:
    if value is None:
        return None
    if not isinstance(value, str) or not value:
        raise ValueError(f"{field} must be a non-empty path")
    path = Path(value)
    if not path.is_absolute():
        raise ValueError(f"{field} must be absolute")
    return str(path)


def _validate_start_spec(spec: Mapping[str, Any], *, run_id: str, protocol: int) -> dict[str, Any]:
    if not isinstance(spec, Mapping):
        raise ValueError("start.spec must be an object")
    value = dict(spec)
    if value.get("runId", run_id) != run_id:
        raise ValueError("start.spec.runId does not match bootstrap runId")
    if "protocol" in value and value["protocol"] != protocol:
        raise ValueError("start.spec.protocol does not match bootstrap protocol")
    value["runId"] = run_id
    if "configPath" in value:
        value["configPath"] = _require_absolute(value.get("configPath"), "configPath")
    if "resourceRoot" in value:
        value["resourceRoot"] = _require_absolute(value.get("resourceRoot"), "resourceRoot")
    return value


def _validate_bootstrap_command(header: Mapping[str, Any], args: argparse.Namespace, expected_type: str) -> dict[str, Any]:
    if not isinstance(header, Mapping):
        raise _BootstrapProtocolError("Runner command header must be an object")
    value = dict(header)
    if value.get("type") != expected_type:
        raise _BootstrapProtocolError(f"expected {expected_type} command")
    if value.get("protocol") != args.protocol:
        raise _BootstrapProtocolError("Runner command protocol does not match")
    if value.get("runId") != args.run_id:
        raise _BootstrapProtocolError("Runner command runId does not match")
    command_seq = value.get("commandSeq")
    if isinstance(command_seq, bool) or not isinstance(command_seq, int) or command_seq <= 0:
        raise _BootstrapProtocolError("Runner commandSeq must be positive")
    if expected_type == "start" and not isinstance(value.get("spec"), Mapping):
        raise _BootstrapProtocolError("start.spec must be an object")
    return value


def _bootstrap(args: argparse.Namespace) -> int:
    if args.protocol != 1:
        raise ValueError(f"unsupported Runner protocol: {args.protocol}")
    _install_parent_death_signal(args.expected_parent_pid)
    command_stream, event_stream, stderr_stream = _open_streams(args)
    # stderr_stream is already owned by the sidecar's pipe when supplied.  Keep a
    # reference alive for the whole run; sys.stderr remains the text diagnostic
    # channel and is never written to event_stream.
    _ = stderr_stream
    writer = _BootstrapWriter(event_stream)
    writer.send(
        {
            "type": "hello",
            "protocol": args.protocol,
            "runId": args.run_id,
            "pid": os.getpid(),
            "binaryLength": 0,
        }
    )

    attached = _bootstrap_read_frame(command_stream)
    if attached is None:
        raise _BootstrapProtocolError("attached command is missing")
    attached_header = _validate_bootstrap_command(attached.header, args, "attached")
    last_command_seq = int(attached_header["commandSeq"])

    start = _bootstrap_read_frame(command_stream)
    if start is None:
        raise _BootstrapProtocolError("start command is missing")
    start_header = _validate_bootstrap_command(start.header, args, "start")
    if int(start_header["commandSeq"]) <= last_command_seq:
        raise _BootstrapProtocolError("start command sequence is not increasing")
    spec_data = _validate_start_spec(start_header["spec"], run_id=args.run_id, protocol=args.protocol)

    # From this point onward the environment is authoritative for all lazily
    # imported config/task modules.
    config_path = spec_data.get("configPath")
    resource_root = spec_data.get("resourceRoot")
    if config_path is not None:
        os.environ["AALC_CONFIG_PATH"] = str(config_path)
    os.environ["AALC_RUN_ID"] = args.run_id
    if resource_root is not None:
        os.environ["AALC_RESOURCE_ROOT"] = str(resource_root)
    os.environ["AALC_EXECUTION_RUNNER"] = "1"
    policy = spec_data.get("platformPolicy", {})
    os.environ["AALC_RUNNER_POLICY"] = json.dumps(policy, ensure_ascii=False, separators=(",", ":"))

    # The protocol must remain on event_stream even if a third-party library uses
    # print().  Redirecting text stdout to stderr prevents accidental frame
    # corruption when event_stream happens to be the standard output pipe.
    try:
        sys.stdout = sys.stderr
        # Some libraries bypass ``sys.stdout`` through ``sys.__stdout__``.  Keep
        # that escape hatch on the diagnostic channel too; the event pipe must
        # never receive an accidental text write.
        sys.__stdout__ = sys.stderr
    except Exception:
        pass

    # Business imports begin only after start and environment setup.
    from module.execution.runner_host import RunnerTaskHost
    from module.execution.supervisor import ExecutionSpec

    spec = ExecutionSpec.from_mapping(spec_data, run_id=args.run_id)
    host = RunnerTaskHost(
        spec,
        command_stream=command_stream,
        event_stream=event_stream,
        initial_command_seq=int(start_header["commandSeq"]),
    )
    host.run()
    return 0


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        return _bootstrap(args)
    except (ValueError, RuntimeError, OSError) as exc:
        # Before streams are opened there is no safe protocol channel.  Once
        # opened, _bootstrap's caller cannot reliably recover its local writer, so
        # emit a concise stderr diagnostic and use a non-zero exit code.  The
        # supervisor classifies this as a failed/crashed Runner using exit status
        # and stderr tail.
        sys.stderr.write(f"Runner bootstrap failed: {exc}\n")
        return 2
    except BaseException:
        traceback.print_exc(file=sys.stderr)
        return 3


if __name__ == "__main__":  # pragma: no cover - exercised by subprocess smoke tests
    raise SystemExit(main())
