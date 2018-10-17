cfg_if! {
    if #[cfg(all(unix, not(target_os = "macos")))] {
        mod unix;
        pub use self::unix::*;
    } else if #[cfg(target_os = "macos")] {
        mod macos;
        pub use self::macos::*;
    } else {
        compile_error!("Unsupported platform");
    }
}
