"""Platform process-tree supervision adapters.

The sidecar owns one adapter instance per Runner.  The public surface is small so
the state machine can be tested with a fake backend without requiring a Windows Job
Object or a live process.  Production adapters keep the same fail-closed contract:
if a child cannot be attached to its containment boundary, launch fails and the
process is terminated rather than running unsupervised.
"""

from __future__ import annotations

import ctypes
import errno
import os
import signal
import subprocess
import sys
import threading
import time
from collections.abc import Mapping, Sequence
from ctypes import wintypes
from dataclasses import dataclass
from typing import Any, Protocol


class SupervisionError(RuntimeError):
    """A Runner could not be placed in its required process boundary."""

    code = "RUNNER_SUPERVISION_FAILED"


@dataclass(frozen=True, slots=True)
class ProcessIdentity:
    """Identity used to avoid mistaking a reused PID for this Runner."""

    pid: int
    create_time: float | None = None
    run_id: str | None = None
    parent_pid: int | None = None
    parent_create_time: float | None = None

    def matches(self, other: "ProcessIdentity | None") -> bool:
        if other is None or self.pid != other.pid:
            return False
        if self.create_time is not None and other.create_time is not None:
            # Windows process times and psutil's values can differ by a tiny amount
            # in conversion; a millisecond tolerance is enough to avoid false
            # mismatches without weakening PID reuse protection.
            if abs(self.create_time - other.create_time) > 0.001:
                return False
        if self.run_id is not None and other.run_id is not None and self.run_id != other.run_id:
            return False
        if self.parent_pid is not None and other.parent_pid is not None and self.parent_pid != other.parent_pid:
            return False
        if self.parent_create_time is not None and other.parent_create_time is not None:
            if abs(self.parent_create_time - other.parent_create_time) > 0.001:
                return False
        return True


class ManagedProcess(Protocol):
    """The process object required by :class:`RunnerSupervisor`."""

    pid: int

    def poll(self) -> int | None: ...

    def wait(self, timeout: float | None = None) -> int: ...

    def terminate(self) -> None: ...

    def kill(self) -> None: ...

    def close(self) -> None: ...


class ProcessAdapter(Protocol):
    """Factory for a contained Runner process."""

    def launch(
        self,
        argv: Sequence[str],
        *,
        cwd: str | os.PathLike[str] | None = None,
        env: Mapping[str, str] | None = None,
        stdin: Any = None,
        stdout: Any = None,
        stderr: Any = None,
        pass_fds: Sequence[int] = (),
    ) -> ManagedProcess: ...

    def terminate(self, process: ManagedProcess, *, grace: float = 3.0) -> bool: ...

    def kill(self, process: ManagedProcess) -> None: ...


def process_identity(process: ManagedProcess, *, run_id: str | None = None) -> ProcessIdentity:
    """Best-effort creation-time lookup for a managed process."""

    pid = int(process.pid)
    create_time: float | None = None
    parent_pid: int | None = None
    parent_create_time: float | None = None
    try:
        import psutil  # type: ignore[import-not-found]

        try:
            observed = psutil.Process(pid)
            create_time = float(observed.create_time())
            parent_pid = int(observed.ppid()) or None
            if parent_pid is not None:
                try:
                    parent_create_time = float(psutil.Process(parent_pid).create_time())
                except (OSError, ValueError, psutil.Error):
                    pass
        except (OSError, ValueError, psutil.Error):
            pass
    except ImportError:
        pass
    return ProcessIdentity(
        pid=pid,
        create_time=create_time,
        run_id=run_id,
        parent_pid=parent_pid,
        parent_create_time=parent_create_time,
    )


class _PopenProcess:
    """Thin adapter around subprocess.Popen with stable methods."""

    def __init__(self, popen: subprocess.Popen[bytes], *, owner: Any = None) -> None:
        self._popen = popen
        self._owner = owner
        self.pid = int(popen.pid)

    @property
    def process(self) -> subprocess.Popen[bytes]:
        return self._popen

    def poll(self) -> int | None:
        return self._popen.poll()

    def wait(self, timeout: float | None = None) -> int:
        return int(self._popen.wait(timeout=timeout))

    def terminate(self) -> None:
        self._popen.terminate()

    def kill(self) -> None:
        self._popen.kill()

    def close(self) -> None:
        # Closing a job after the process has exited is safe and is what makes the
        # KILL_ON_JOB_CLOSE fallback harmless during normal completion.
        owner = self._owner
        self._owner = None
        if owner is not None:
            close = getattr(owner, "close", None)
            if callable(close):
                close()


class PosixProcessGroup:
    """Linux process-group adapter (also usable for Unix test environments)."""

    def __init__(self, *, platform_name: str | None = None, term_signal: int = signal.SIGTERM) -> None:
        self.platform_name = platform_name or sys.platform
        self.term_signal = term_signal

    def launch(
        self,
        argv: Sequence[str],
        *,
        cwd: str | os.PathLike[str] | None = None,
        env: Mapping[str, str] | None = None,
        stdin: Any = None,
        stdout: Any = None,
        stderr: Any = None,
        pass_fds: Sequence[int] = (),
    ) -> ManagedProcess:
        if not argv:
            raise ValueError("argv cannot be empty")
        try:
            process = subprocess.Popen(
                list(argv),
                cwd=cwd,
                env=dict(env) if env is not None else None,
                stdin=stdin,
                stdout=stdout,
                stderr=stderr,
                start_new_session=True,
                close_fds=True,
                pass_fds=tuple(int(fd) for fd in pass_fds),
            )
        except OSError as exc:
            raise SupervisionError(f"failed to create Runner process group: {exc}") from exc
        return _PopenProcess(process)

    @staticmethod
    def _group_alive(process: ManagedProcess) -> bool:
        return process.poll() is None

    def _signal_group(self, process: ManagedProcess, sig: int) -> None:
        if not self._group_alive(process):
            return
        try:
            os.killpg(os.getpgid(int(process.pid)), sig)
        except ProcessLookupError:
            return
        except OSError as exc:
            # ESRCH means the group died between poll and killpg.  Other errors
            # are useful supervision failures and should not be hidden.
            if exc.errno != errno.ESRCH:
                raise SupervisionError(f"failed to signal Runner process group: {exc}") from exc

    def terminate(self, process: ManagedProcess, *, grace: float = 3.0) -> bool:
        self._signal_group(process, self.term_signal)
        deadline = time.monotonic() + max(0.0, float(grace))
        while process.poll() is None and time.monotonic() < deadline:
            time.sleep(min(0.02, max(0.0, deadline - time.monotonic())))
        if process.poll() is not None:
            return True
        self.kill(process)
        return process.poll() is not None

    def kill(self, process: ManagedProcess) -> None:
        self._signal_group(process, signal.SIGKILL)


class MacOSProcessGroup(PosixProcessGroup):
    """Best-effort macOS group adapter.

    macOS has no Linux ``PR_SET_PDEATHSIG`` equivalent; the command-pipe EOF and
    normal sidecar killpg path still provide useful containment.  The adapter is
    named separately so callers can report that limitation explicitly.
    """

    best_effort = True


class JobBackend(Protocol):
    """Injectable Windows Job operations used by the adapter and its tests."""

    def create(self) -> Any: ...

    def assign(self, job: Any, process: Any) -> None: ...

    def resume(self, process: Any) -> None: ...

    def terminate(self, job: Any, exit_code: int = 1) -> None: ...

    def close(self, job: Any) -> None: ...


_HANDLE = ctypes.c_void_p
_INVALID_HANDLE_VALUE = ctypes.c_void_p(-1).value
_HANDLE_FLAG_INHERIT = 0x00000001
_DUPLICATE_SAME_ACCESS = 0x00000002
_CREATE_SUSPENDED = 0x00000004
_CREATE_NO_WINDOW = 0x08000000
_CREATE_BREAKAWAY_FROM_JOB = 0x01000000
_CREATE_UNICODE_ENVIRONMENT = 0x00000400
_EXTENDED_STARTUPINFO_PRESENT = 0x00080000
_STARTF_USESTDHANDLES = 0x00000100
_PROC_THREAD_ATTRIBUTE_HANDLE_LIST = 0x00020002
_JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x00002000
_JOB_OBJECT_EXTENDED_LIMIT_INFORMATION = 9
_WAIT_OBJECT_0 = 0x00000000
_WAIT_TIMEOUT = 0x00000102
_WAIT_FAILED = 0xFFFFFFFF
_STILL_ACTIVE = 259
_ERROR_INSUFFICIENT_BUFFER = 122
_JOB_OBJECT_BASIC_ACCOUNTING_INFORMATION = 1


class _SecurityAttributes(ctypes.Structure):
    _fields_ = [
        ("nLength", wintypes.DWORD),
        ("lpSecurityDescriptor", _HANDLE),
        ("bInheritHandle", wintypes.BOOL),
    ]


class _StartupInfo(ctypes.Structure):
    _fields_ = [
        ("cb", wintypes.DWORD),
        ("lpReserved", wintypes.LPWSTR),
        ("lpDesktop", wintypes.LPWSTR),
        ("lpTitle", wintypes.LPWSTR),
        ("dwX", wintypes.DWORD),
        ("dwY", wintypes.DWORD),
        ("dwXSize", wintypes.DWORD),
        ("dwYSize", wintypes.DWORD),
        ("dwXCountChars", wintypes.DWORD),
        ("dwYCountChars", wintypes.DWORD),
        ("dwFillAttribute", wintypes.DWORD),
        ("dwFlags", wintypes.DWORD),
        ("wShowWindow", wintypes.WORD),
        ("cbReserved2", wintypes.WORD),
        ("lpReserved2", _HANDLE),
        ("hStdInput", _HANDLE),
        ("hStdOutput", _HANDLE),
        ("hStdError", _HANDLE),
    ]


class _StartupInfoEx(ctypes.Structure):
    _fields_ = [
        ("StartupInfo", _StartupInfo),
        ("lpAttributeList", _HANDLE),
    ]


class _ProcessInformation(ctypes.Structure):
    _fields_ = [
        ("hProcess", _HANDLE),
        ("hThread", _HANDLE),
        ("dwProcessId", wintypes.DWORD),
        ("dwThreadId", wintypes.DWORD),
    ]


class _IoCounters(ctypes.Structure):
    _fields_ = [
        ("ReadOperationCount", ctypes.c_ulonglong),
        ("WriteOperationCount", ctypes.c_ulonglong),
        ("OtherOperationCount", ctypes.c_ulonglong),
        ("ReadTransferCount", ctypes.c_ulonglong),
        ("WriteTransferCount", ctypes.c_ulonglong),
        ("OtherTransferCount", ctypes.c_ulonglong),
    ]


class _JobBasicLimitInformation(ctypes.Structure):
    _fields_ = [
        ("PerProcessUserTimeLimit", ctypes.c_longlong),
        ("PerJobUserTimeLimit", ctypes.c_longlong),
        ("LimitFlags", wintypes.DWORD),
        ("MinimumWorkingSetSize", ctypes.c_size_t),
        ("MaximumWorkingSetSize", ctypes.c_size_t),
        ("ActiveProcessLimit", wintypes.DWORD),
        ("Affinity", ctypes.c_size_t),
        ("PriorityClass", wintypes.DWORD),
        ("SchedulingClass", wintypes.DWORD),
    ]


class _JobExtendedLimitInformation(ctypes.Structure):
    _fields_ = [
        ("BasicLimitInformation", _JobBasicLimitInformation),
        ("IoInfo", _IoCounters),
        ("ProcessMemoryLimit", ctypes.c_size_t),
        ("JobMemoryLimit", ctypes.c_size_t),
        ("PeakProcessMemoryUsed", ctypes.c_size_t),
        ("PeakJobMemoryUsed", ctypes.c_size_t),
    ]


class _JobBasicAccountingInformation(ctypes.Structure):
    _fields_ = [
        ("TotalUserTime", ctypes.c_longlong),
        ("TotalKernelTime", ctypes.c_longlong),
        ("ThisPeriodTotalUserTime", ctypes.c_longlong),
        ("ThisPeriodTotalKernelTime", ctypes.c_longlong),
        ("TotalPageFaultCount", wintypes.DWORD),
        ("TotalProcesses", wintypes.DWORD),
        ("ActiveProcesses", wintypes.DWORD),
        ("TotalTerminatedProcesses", wintypes.DWORD),
    ]


def _win32_failure(operation: str) -> OSError:
    error = int(ctypes.get_last_error()) or 1
    formatter = getattr(ctypes, "FormatError", None)
    detail = formatter(error) if callable(formatter) else f"Win32 error {error}"
    return OSError(error, f"{operation} failed: {detail}")


class _CtypesJobBackend:
    """Native Win32 Job Object and suspended-process implementation.

    The backend deliberately owns all of the handles it creates.  The process
    is created suspended, assigned to a non-inheritable Job Object, and only
    then resumed.  Standard handles are duplicated as inheritable child
    handles and passed through ``PROC_THREAD_ATTRIBUTE_HANDLE_LIST``; this
    avoids inheriting unrelated application handles.
    """

    native = True

    def __init__(self) -> None:
        if os.name != "nt":
            raise SupervisionError("native Windows Job supervision is unavailable on this platform")
        self._kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        self._bind_functions()

    def _bind_functions(self) -> None:
        kernel32 = self._kernel32
        kernel32.CreateJobObjectW.argtypes = [ctypes.POINTER(_SecurityAttributes), wintypes.LPCWSTR]
        kernel32.CreateJobObjectW.restype = _HANDLE
        kernel32.SetInformationJobObject.argtypes = [_HANDLE, wintypes.INT, ctypes.c_void_p, wintypes.DWORD]
        kernel32.SetInformationJobObject.restype = wintypes.BOOL
        kernel32.QueryInformationJobObject.argtypes = [
            _HANDLE,
            wintypes.INT,
            ctypes.c_void_p,
            wintypes.DWORD,
            ctypes.POINTER(wintypes.DWORD),
        ]
        kernel32.QueryInformationJobObject.restype = wintypes.BOOL
        kernel32.AssignProcessToJobObject.argtypes = [_HANDLE, _HANDLE]
        kernel32.AssignProcessToJobObject.restype = wintypes.BOOL
        kernel32.TerminateJobObject.argtypes = [_HANDLE, wintypes.UINT]
        kernel32.TerminateJobObject.restype = wintypes.BOOL
        kernel32.CloseHandle.argtypes = [_HANDLE]
        kernel32.CloseHandle.restype = wintypes.BOOL
        kernel32.SetHandleInformation.argtypes = [_HANDLE, wintypes.DWORD, wintypes.DWORD]
        kernel32.SetHandleInformation.restype = wintypes.BOOL
        kernel32.ResumeThread.argtypes = [_HANDLE]
        kernel32.ResumeThread.restype = wintypes.DWORD
        kernel32.WaitForSingleObject.argtypes = [_HANDLE, wintypes.DWORD]
        kernel32.WaitForSingleObject.restype = wintypes.DWORD
        kernel32.GetExitCodeProcess.argtypes = [_HANDLE, ctypes.POINTER(wintypes.DWORD)]
        kernel32.GetExitCodeProcess.restype = wintypes.BOOL
        kernel32.TerminateProcess.argtypes = [_HANDLE, wintypes.UINT]
        kernel32.TerminateProcess.restype = wintypes.BOOL
        kernel32.DuplicateHandle.argtypes = [
            _HANDLE,
            _HANDLE,
            _HANDLE,
            ctypes.POINTER(_HANDLE),
            wintypes.DWORD,
            wintypes.BOOL,
            wintypes.DWORD,
        ]
        kernel32.DuplicateHandle.restype = wintypes.BOOL
        kernel32.GetCurrentProcess.argtypes = []
        kernel32.GetCurrentProcess.restype = _HANDLE
        kernel32.InitializeProcThreadAttributeList.argtypes = [
            _HANDLE,
            wintypes.DWORD,
            wintypes.DWORD,
            ctypes.POINTER(ctypes.c_size_t),
        ]
        kernel32.InitializeProcThreadAttributeList.restype = wintypes.BOOL
        kernel32.UpdateProcThreadAttribute.argtypes = [
            _HANDLE,
            wintypes.DWORD,
            ctypes.c_size_t,
            ctypes.c_void_p,
            ctypes.c_size_t,
            ctypes.c_void_p,
            ctypes.POINTER(ctypes.c_size_t),
        ]
        kernel32.UpdateProcThreadAttribute.restype = wintypes.BOOL
        kernel32.DeleteProcThreadAttributeList.argtypes = [_HANDLE]
        kernel32.DeleteProcThreadAttributeList.restype = None
        kernel32.CreateProcessW.argtypes = [
            wintypes.LPCWSTR,
            wintypes.LPWSTR,
            ctypes.POINTER(_SecurityAttributes),
            ctypes.POINTER(_SecurityAttributes),
            wintypes.BOOL,
            wintypes.DWORD,
            ctypes.c_void_p,
            wintypes.LPCWSTR,
            ctypes.POINTER(_StartupInfo),
            ctypes.POINTER(_ProcessInformation),
        ]
        kernel32.CreateProcessW.restype = wintypes.BOOL

    @staticmethod
    def _value(handle: Any) -> int:
        value = getattr(handle, "value", handle)
        return int(value or 0)

    @classmethod
    def _handle(cls, handle: Any) -> _HANDLE:
        value = cls._value(handle)
        return _HANDLE(value)

    def create(self) -> int:
        job = self._kernel32.CreateJobObjectW(None, None)
        job_value = self._value(job)
        if not job_value:
            raise _win32_failure("CreateJobObjectW")
        try:
            # KILL_ON_JOB_CLOSE is the containment fallback if the sidecar dies.
            # We intentionally do not set either BREAKAWAY flag.
            info = _JobExtendedLimitInformation()
            info.BasicLimitInformation.LimitFlags = _JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            if not self._kernel32.SetInformationJobObject(
                self._handle(job_value),
                _JOB_OBJECT_EXTENDED_LIMIT_INFORMATION,
                ctypes.byref(info),
                ctypes.sizeof(info),
            ):
                raise _win32_failure("SetInformationJobObject")
            if not self._kernel32.SetHandleInformation(self._handle(job_value), _HANDLE_FLAG_INHERIT, 0):
                raise _win32_failure("SetHandleInformation(job)")
            return job_value
        except Exception:
            try:
                self.close(job_value)
            except Exception:
                pass
            raise

    def assign(self, job: Any, process: Any) -> None:
        process_handle = getattr(process, "_process_handle", process)
        if not self._kernel32.AssignProcessToJobObject(self._handle(job), self._handle(process_handle)):
            raise _win32_failure("AssignProcessToJobObject")

    def resume(self, process: Any) -> None:
        thread_handle = getattr(process, "_thread_handle", process)
        previous_count = int(self._kernel32.ResumeThread(self._handle(thread_handle)))
        if previous_count == _WAIT_FAILED:
            raise _win32_failure("ResumeThread")

    def terminate(self, job: Any, exit_code: int = 1) -> None:
        if not self._kernel32.TerminateJobObject(self._handle(job), int(exit_code)):
            raise _win32_failure("TerminateJobObject")

    def close(self, job: Any) -> None:
        value = self._value(job)
        if value:
            self._kernel32.CloseHandle(self._handle(value))

    def close_handle(self, handle: Any) -> None:
        value = self._value(handle)
        if value:
            self._kernel32.CloseHandle(self._handle(value))

    def terminate_process(self, process_handle: Any, exit_code: int = 1) -> None:
        if not self._kernel32.TerminateProcess(self._handle(process_handle), int(exit_code)):
            error = int(ctypes.get_last_error())
            # A process that exited between poll and terminate is already in the
            # desired state and should not turn normal cleanup into a failure.
            if error not in {0, 5, 87}:
                raise _win32_failure("TerminateProcess")

    def wait_handle(self, process_handle: Any, timeout: float | None = None) -> int:
        if timeout is None:
            milliseconds = 0xFFFFFFFF
        else:
            milliseconds = min(0xFFFFFFFE, max(0, int(float(timeout) * 1000)))
        result = int(self._kernel32.WaitForSingleObject(self._handle(process_handle), milliseconds))
        if result == _WAIT_TIMEOUT:
            raise TimeoutError("process wait timed out")
        if result == _WAIT_FAILED:
            raise _win32_failure("WaitForSingleObject")
        return result

    def exit_code(self, process_handle: Any) -> int:
        code = wintypes.DWORD()
        if not self._kernel32.GetExitCodeProcess(self._handle(process_handle), ctypes.byref(code)):
            raise _win32_failure("GetExitCodeProcess")
        return int(code.value)

    def active_processes(self, job: Any) -> int:
        info = _JobBasicAccountingInformation()
        returned = wintypes.DWORD()
        if not self._kernel32.QueryInformationJobObject(
            self._handle(job),
            _JOB_OBJECT_BASIC_ACCOUNTING_INFORMATION,
            ctypes.byref(info),
            ctypes.sizeof(info),
            ctypes.byref(returned),
        ):
            raise _win32_failure("QueryInformationJobObject")
        return int(info.ActiveProcesses)

    @staticmethod
    def _source_handle(value: Any) -> int:
        if isinstance(value, ctypes.c_void_p):
            result = _CtypesJobBackend._value(value)
            if not result:
                raise ValueError("invalid Windows handle")
            return result
        if isinstance(value, int):
            if value < 0:
                raise ValueError("special subprocess handles require an explicit pipe")
            fd = value
        else:
            fileno = getattr(value, "fileno", None)
            if not callable(fileno):
                raise TypeError("stdio value must be a file descriptor or file object")
            fd = int(fileno())
        try:
            import msvcrt

            handle = int(msvcrt.get_osfhandle(fd))
        except (ImportError, OSError, ValueError) as exc:
            raise OSError(f"cannot obtain Windows handle for fd {fd}") from exc
        if handle == _INVALID_HANDLE_VALUE:
            raise OSError(f"fd {fd} has no valid Windows handle")
        return handle

    @staticmethod
    def _open_devnull(index: int) -> int:
        flags = os.O_RDONLY if index == 0 else os.O_WRONLY
        return os.open("NUL", flags)

    def _duplicate_stdio(self, stdin: Any, stdout: Any, stderr: Any) -> tuple[list[int], list[int], list[int]]:
        values = [stdin, stdout, stderr]
        duplicated: list[int] = []
        std_handles = [0, 0, 0]
        opened_fds: list[int] = []
        current = self._kernel32.GetCurrentProcess()
        try:
            if isinstance(stderr, int) and stderr == getattr(subprocess, "STDOUT", -2):
                values[2] = stdout
            for index, value in enumerate(values):
                if value is None:
                    continue
                if isinstance(value, int) and value == getattr(subprocess, "PIPE", -1):
                    raise SupervisionError("Windows Job adapter requires an explicit pipe for subprocess.PIPE")
                if isinstance(value, int) and value == getattr(subprocess, "DEVNULL", -3):
                    value = self._open_devnull(index)
                    opened_fds.append(value)
                source = self._source_handle(value)
                target = _HANDLE()
                if not self._kernel32.DuplicateHandle(
                    self._handle(current),
                    self._handle(source),
                    self._handle(current),
                    ctypes.byref(target),
                    0,
                    True,
                    _DUPLICATE_SAME_ACCESS,
                ):
                    raise _win32_failure("DuplicateHandle")
                target_value = self._value(target)
                duplicated.append(target_value)
                std_handles[index] = target_value
            return std_handles, duplicated, opened_fds
        except Exception:
            for handle in duplicated:
                self.close_handle(handle)
            for fd in opened_fds:
                try:
                    os.close(fd)
                except OSError:
                    pass
            raise

    def _attribute_list(self, handles: list[int]) -> tuple[_HANDLE, Any, Any]:
        if not handles:
            return _HANDLE(), None, None
        size = ctypes.c_size_t()
        first = self._kernel32.InitializeProcThreadAttributeList(None, 1, 0, ctypes.byref(size))
        first_error = int(ctypes.get_last_error())
        if size.value <= 0 or (first and first_error not in {0, _ERROR_INSUFFICIENT_BUFFER}):
            raise _win32_failure("InitializeProcThreadAttributeList(size)")
        storage = (ctypes.c_byte * size.value)()
        attribute_list = self._handle(ctypes.addressof(storage))
        if not self._kernel32.InitializeProcThreadAttributeList(attribute_list, 1, 0, ctypes.byref(size)):
            raise _win32_failure("InitializeProcThreadAttributeList")
        handle_array = (_HANDLE * len(handles))(*[self._handle(handle) for handle in handles])
        if not self._kernel32.UpdateProcThreadAttribute(
            attribute_list,
            0,
            _PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
            ctypes.cast(handle_array, ctypes.c_void_p),
            ctypes.sizeof(handle_array),
            None,
            None,
        ):
            self._kernel32.DeleteProcThreadAttributeList(attribute_list)
            raise _win32_failure("UpdateProcThreadAttribute(handle list)")
        return attribute_list, storage, handle_array

    @staticmethod
    def _environment_buffer(env: Mapping[str, str] | None) -> Any:
        if env is None:
            return None
        entries = sorted((f"{key}={value}" for key, value in env.items()), key=str.casefold)
        # ``create_unicode_buffer`` contributes the final NUL; one explicit NUL
        # terminates the final ``KEY=VALUE`` entry and yields the required double
        # NUL terminator (including for an empty environment).
        return ctypes.create_unicode_buffer("\0".join(entries) + "\0")

    def launch_process(
        self,
        argv: Sequence[str],
        *,
        cwd: str | os.PathLike[str] | None = None,
        env: Mapping[str, str] | None = None,
        stdin: Any = None,
        stdout: Any = None,
        stderr: Any = None,
        creationflags: int = _CREATE_SUSPENDED | _CREATE_NO_WINDOW,
    ) -> ManagedProcess:
        if not argv:
            raise ValueError("argv cannot be empty")
        job = self.create()
        process_info = _ProcessInformation()
        duplicated: list[int] = []
        opened_fds: list[int] = []
        attribute_list = _HANDLE()
        attribute_storage = None
        attribute_handles = None
        process_handle = 0
        thread_handle = 0
        try:
            std_handles, duplicated, opened_fds = self._duplicate_stdio(stdin, stdout, stderr)
            attribute_list, attribute_storage, attribute_handles = self._attribute_list(duplicated)
            startup = _StartupInfoEx()
            startup.StartupInfo.cb = ctypes.sizeof(_StartupInfoEx)
            if any(std_handles):
                startup.StartupInfo.dwFlags |= _STARTF_USESTDHANDLES
                startup.StartupInfo.hStdInput = self._handle(std_handles[0])
                startup.StartupInfo.hStdOutput = self._handle(std_handles[1])
                startup.StartupInfo.hStdError = self._handle(std_handles[2])
            startup.lpAttributeList = attribute_list
            command_line = ctypes.create_unicode_buffer(subprocess.list2cmdline([str(item) for item in argv]))
            environment = self._environment_buffer(env)
            environment_pointer = ctypes.cast(environment, ctypes.c_void_p) if environment is not None else None
            # Never permit a caller to remove the suspended-create/no-window
            # containment prerequisites from the native production path.
            flags = (
                int(creationflags) & ~_CREATE_BREAKAWAY_FROM_JOB
            ) | _CREATE_SUSPENDED | _CREATE_NO_WINDOW | _EXTENDED_STARTUPINFO_PRESENT
            if environment is not None:
                flags |= _CREATE_UNICODE_ENVIRONMENT
            startup_pointer = ctypes.cast(ctypes.byref(startup), ctypes.POINTER(_StartupInfo))
            # With no explicitly supplied stdio handles, disable inheritance
            # entirely.  Passing TRUE without an attribute handle list would
            # inherit any unrelated inheritable handle in the sidecar.
            if not self._kernel32.CreateProcessW(
                None,
                command_line,
                None,
                None,
                bool(duplicated),
                flags,
                environment_pointer,
                str(cwd) if cwd is not None else None,
                startup_pointer,
                ctypes.byref(process_info),
            ):
                raise _win32_failure("CreateProcessW")
            process_handle = self._value(process_info.hProcess)
            thread_handle = self._value(process_info.hThread)
            # Assignment occurs before the first instruction is resumed.  If it
            # fails, terminate and close every native handle before surfacing it.
            self.assign(job, process_handle)
            self.resume(thread_handle)
            return _NativeProcess(
                self,
                process_handle,
                thread_handle,
                int(process_info.dwProcessId),
                owner=_JobOwner(self, job),
            )
        except Exception as exc:
            if process_handle:
                try:
                    self.terminate_process(process_handle, 1)
                    self.wait_handle(process_handle, 1.0)
                except Exception:
                    pass
            if thread_handle:
                self.close_handle(thread_handle)
            if process_handle:
                self.close_handle(process_handle)
            try:
                self.close(job)
            except Exception:
                # Preserve the launch/assignment failure while still making a
                # best-effort close attempt.  A close API error must not skip
                # the deterministic failed-launch classification.
                pass
            if isinstance(exc, SupervisionError):
                raise
            raise SupervisionError(f"failed to create or assign Runner to Windows Job: {exc}") from exc
        finally:
            if attribute_list:
                self._kernel32.DeleteProcThreadAttributeList(attribute_list)
            # Keep the backing storage and handle array alive through the native
            # call above, then close all parent-side duplicates and NUL fds.
            _ = attribute_storage, attribute_handles
            for handle in duplicated:
                self.close_handle(handle)
            for fd in opened_fds:
                try:
                    os.close(fd)
                except OSError:
                    pass


class _NativeProcess:
    """Managed process backed by native process/thread handles."""

    def __init__(
        self,
        backend: _CtypesJobBackend,
        process_handle: int,
        thread_handle: int,
        pid: int,
        *,
        owner: Any = None,
    ) -> None:
        self._backend = backend
        self._process_handle = process_handle
        self._thread_handle = thread_handle
        self._owner = owner
        self.pid = int(pid)
        self._closed = False
        self._exit_code: int | None = None
        self._lock = threading.RLock()

    def poll(self) -> int | None:
        # Do not hold the state lock while probing the native handle.  The
        # supervisor's watcher performs a blocking ``wait`` on another thread;
        # serialising that wait with ``poll`` would make every timeout/force
        # path block until the Runner exits on its own.
        with self._guard():
            if self._exit_code is not None:
                return self._exit_code
            process_handle = self._process_handle
        try:
            self._backend.wait_handle(process_handle, 0.0)
        except TimeoutError:
            return None
        code = self._backend.exit_code(process_handle)
        with self._guard():
            if self._exit_code is None:
                self._exit_code = code
            return self._exit_code

    def wait(self, timeout: float | None = None) -> int:
        with self._guard():
            if self._exit_code is not None:
                return self._exit_code
            process_handle = self._process_handle
        self._backend.wait_handle(process_handle, timeout)
        code = self._backend.exit_code(process_handle)
        with self._guard():
            if self._exit_code is None:
                self._exit_code = code
            return self._exit_code

    def terminate(self) -> None:
        self._backend.terminate_process(self._process_handle, 1)

    def kill(self) -> None:
        self._backend.terminate_process(self._process_handle, 1)

    def close(self) -> None:
        lock = self._lock
        if lock is None:
            self._close_unlocked()
            return
        with lock:
            self._close_unlocked()

    def _close_unlocked(self) -> None:
        if self._closed:
            return
        self._closed = True
        owner, self._owner = self._owner, None
        try:
            if owner is not None:
                close = getattr(owner, "close", None)
                if callable(close):
                    close()
        finally:
            self._backend.close_handle(self._thread_handle)
            self._backend.close_handle(self._process_handle)

    def _guard(self):
        return self._lock


class _PyWin32JobBackend:
    """Small pywin32 Job Object backend.

    Process creation remains in ``subprocess`` so callers can use ordinary pipe
    objects.  The process is created suspended; assignment happens before the
    native process is resumed, which is the important race-free ordering.  Python
    does not expose the suspended main-thread handle from Popen, so the backend
    uses the documented ``NtResumeProcess`` system call as a narrow fallback.  A
    native launcher can be injected for environments that prohibit that call.
    """

    def __init__(self) -> None:
        try:
            import ctypes

            import win32job  # type: ignore[import-not-found]
        except ImportError as exc:  # pragma: no cover - only non-Windows fallback
            raise SupervisionError("pywin32 is required for Windows Job supervision") from exc
        self._ctypes = ctypes
        self._win32job = win32job

    def create(self) -> Any:
        # pywin32 requires an explicit empty name (the Win32 API accepts NULL for
        # an unnamed object, but its wrapper rejects ``None``).
        job = self._win32job.CreateJobObject(None, "")
        info = self._win32job.QueryInformationJobObject(
            job,
            self._win32job.JobObjectExtendedLimitInformation,
        )
        info["BasicLimitInformation"]["LimitFlags"] |= self._win32job.JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        self._win32job.SetInformationJobObject(
            job,
            self._win32job.JobObjectExtendedLimitInformation,
            info,
        )
        return job

    def assign(self, job: Any, process: Any) -> None:
        handle = getattr(process, "_handle", process)
        self._win32job.AssignProcessToJobObject(job, handle)

    def resume(self, process: Any) -> None:
        # ``NtResumeProcess`` is exported by ntdll on supported Windows versions.
        # It resumes the suspended process before any of its children can run.
        handle = getattr(process, "_handle", process)
        ntdll = self._ctypes.WinDLL("ntdll")
        status = int(ntdll.NtResumeProcess(self._ctypes.c_void_p(handle)))
        if status != 0:
            raise OSError(status, "NtResumeProcess failed")

    def terminate(self, job: Any, exit_code: int = 1) -> None:
        self._win32job.TerminateJobObject(job, int(exit_code))

    def close(self, job: Any) -> None:
        try:
            import win32api  # type: ignore[import-not-found]

            win32api.CloseHandle(job)
        except (ImportError, OSError):
            pass


class WindowsJobAdapter:
    """Windows Job Object process adapter with injectable backend.

    The ``backend`` is intentionally part of the constructor.  Unit tests can
    assert ``create → assign → resume`` and simulated nested-job failures without
    depending on host policy or a real Windows process tree.
    """

    def __init__(self, *, backend: JobBackend | None = None, creationflags: int | None = None) -> None:
        # The ctypes backend is the production path.  An injected backend keeps
        # the state machine deterministic in unit tests and allows downstream
        # platforms to provide a compatible implementation.
        self.backend = backend if backend is not None else _CtypesJobBackend()
        self.creationflags = creationflags

    def launch(
        self,
        argv: Sequence[str],
        *,
        cwd: str | os.PathLike[str] | None = None,
        env: Mapping[str, str] | None = None,
        stdin: Any = None,
        stdout: Any = None,
        stderr: Any = None,
        pass_fds: Sequence[int] = (),
    ) -> ManagedProcess:
        if not argv:
            raise ValueError("argv cannot be empty")
        if pass_fds:
            # Windows handle inheritance is explicit in the production launcher;
            # subprocess cannot map POSIX fd semantics here.
            raise SupervisionError("pass_fds is not supported by Windows Job adapter")
        flags = self.creationflags
        if flags is None:
            flags = _CREATE_SUSPENDED | _CREATE_NO_WINDOW
        native_launch = getattr(self.backend, "launch_process", None)
        if callable(native_launch):
            return native_launch(
                argv,
                cwd=cwd,
                env=env,
                stdin=stdin,
                stdout=stdout,
                stderr=stderr,
                creationflags=int(flags),
            )
        job = None
        process: subprocess.Popen[bytes] | None = None
        try:
            job = self.backend.create()
            process = subprocess.Popen(
                list(argv),
                cwd=cwd,
                env=dict(env) if env is not None else None,
                stdin=stdin,
                stdout=stdout,
                stderr=stderr,
                creationflags=flags,
                close_fds=True,
            )
            self.backend.assign(job, process)
            self.backend.resume(process)
            return _PopenProcess(process, owner=_JobOwner(self.backend, job))
        except Exception as exc:
            if process is not None:
                try:
                    process.kill()
                except OSError:
                    pass
                try:
                    process.wait(timeout=1.0)
                except Exception:
                    pass
            if job is not None:
                try:
                    self.backend.close(job)
                except Exception:
                    pass
            if isinstance(exc, SupervisionError):
                raise
            raise SupervisionError(f"failed to assign Runner to Windows Job: {exc}") from exc

    def terminate(self, process: ManagedProcess, *, grace: float = 3.0) -> bool:
        owner = getattr(process, "_owner", None)
        job = owner.job if isinstance(owner, _JobOwner) else None
        if job is not None:
            try:
                self.backend.terminate(job)
            except Exception as exc:
                raise SupervisionError(f"TerminateJobObject failed: {exc}") from exc
        else:
            process.terminate()
        deadline = time.monotonic() + max(0.0, float(grace))
        while self._tree_alive(process, job) and time.monotonic() < deadline:
            time.sleep(min(0.02, max(0.0, deadline - time.monotonic())))
        return not self._tree_alive(process, job)

    def kill(self, process: ManagedProcess) -> None:
        owner = getattr(process, "_owner", None)
        job = owner.job if isinstance(owner, _JobOwner) else None
        if job is not None:
            self.backend.terminate(job, exit_code=1)
            # TerminateJobObject is asynchronous.  Native callers need the same
            # bounded tree-death observation as terminate(); injected fakes that
            # do not expose active_processes retain their immediate semantics.
            if callable(getattr(self.backend, "active_processes", None)):
                deadline = time.monotonic() + 1.0
                while self._tree_alive(process, job) and time.monotonic() < deadline:
                    time.sleep(0.02)
        else:
            process.kill()

    def _tree_alive(self, process: ManagedProcess, job: Any) -> bool:
        if process.poll() is None:
            return True
        if job is None:
            return False
        active_processes = getattr(self.backend, "active_processes", None)
        if not callable(active_processes):
            return False
        try:
            return int(active_processes(job)) > 0
        except Exception:
            # Once the Job handle is closing, a failed query means there is no
            # reliable tree state left to observe.  The process handle has
            # already reported exit, so allow finalization to proceed.
            return False


@dataclass(slots=True)
class _JobOwner:
    backend: JobBackend
    job: Any

    def close(self) -> None:
        job, self.job = self.job, None
        if job is not None:
            self.backend.close(job)


def create_process_adapter(platform_name: str | None = None, *, backend: JobBackend | None = None) -> ProcessAdapter:
    """Return the adapter for ``platform_name`` (defaults to the host platform)."""

    name = platform_name or sys.platform
    if name.startswith("win"):
        return WindowsJobAdapter(backend=backend)
    if name == "darwin":
        return MacOSProcessGroup(platform_name=name)
    return PosixProcessGroup(platform_name=name)


# Names used by callers that prefer an explicit containment vocabulary.
LinuxProcessGroup = PosixProcessGroup
PlatformProcessSupervisor = ProcessAdapter


__all__ = [
    "JobBackend",
    "LinuxProcessGroup",
    "MacOSProcessGroup",
    "ManagedProcess",
    "PlatformProcessSupervisor",
    "PosixProcessGroup",
    "ProcessAdapter",
    "ProcessIdentity",
    "SupervisionError",
    "WindowsJobAdapter",
    "create_process_adapter",
    "process_identity",
]
