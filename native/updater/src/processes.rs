//! Stop the application and its helper processes before replacing files.

use std::path::Path;

use crate::{paths, Result, UpdaterError};

#[cfg(windows)]
pub fn terminate_install_processes(install_root: &Path) -> Result<()> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    let root = paths::normalize_for_compare(install_root);
    let root_prefix = format!("{root}\\");
    let self_pid = std::process::id();

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
            .map_err(|error| UpdaterError::process(format!("无法枚举进程：{error}")))?;
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        let mut has_entry = Process32FirstW(snapshot, &mut entry).is_ok();
        while has_entry {
            let pid = entry.th32ProcessID;
            if pid != 0 && pid != self_pid {
                let name_length = entry
                    .szExeFile
                    .iter()
                    .position(|value| *value == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = OsString::from_wide(&entry.szExeFile[..name_length])
                    .to_string_lossy()
                    .to_string();
                let by_name = name.eq_ignore_ascii_case("AALC.nsh.exe")
                    || name.eq_ignore_ascii_case("AALC.exe");
                if by_name || image_path_is_under(pid, &root, &root_prefix) {
                    terminate_process(pid);
                }
            }
            has_entry = Process32NextW(snapshot, &mut entry).is_ok();
        }
        let _ = CloseHandle(snapshot);
    }

    Ok(())
}

#[cfg(windows)]
unsafe fn image_path_is_under(pid: u32, root: &str, root_prefix: &str) -> bool {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };

    let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
        return false;
    };
    let mut buffer = [0_u16; 32_768];
    let mut size = buffer.len() as u32;
    let result = QueryFullProcessImageNameW(
        process,
        PROCESS_NAME_WIN32,
        PWSTR(buffer.as_mut_ptr()),
        &mut size,
    );
    let _ = CloseHandle(process);
    if result.is_err() {
        return false;
    }
    let image = String::from_utf16_lossy(&buffer[..size as usize])
        .replace('/', "\\")
        .to_lowercase();
    image == root || image.starts_with(root_prefix)
}

#[cfg(windows)]
unsafe fn terminate_process(pid: u32) {
    use windows::Win32::Foundation::{CloseHandle, WAIT_TIMEOUT};
    use windows::Win32::System::Threading::{
        OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
    };

    let access = PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE;
    let Ok(process) = OpenProcess(access, false, pid) else {
        return;
    };
    let _ = TerminateProcess(process, 1);
    let wait = WaitForSingleObject(process, 10_000);
    if wait == WAIT_TIMEOUT {
        let _ = TerminateProcess(process, 1);
        let _ = WaitForSingleObject(process, 5_000);
    }
    let _ = CloseHandle(process);
}

#[cfg(not(windows))]
pub fn terminate_install_processes(_install_root: &Path) -> Result<()> {
    Err(UpdaterError::process(
        "AALC Updater 的进程终止逻辑仅支持 Windows",
    ))
}
