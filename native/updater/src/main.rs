//! `AALC Updater.exe` entry point.

fn main() {
    // Manual runs print UTF-8 help/log lines; keep a GBK console from
    // rendering them as replacement characters.
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Console::SetConsoleOutputCP;
        let _ = SetConsoleOutputCP(65001);
    }

    if let Err(error) = aalc_updater::run() {
        aalc_updater::report_fatal(&error);
        std::process::exit(error.code);
    }
}
