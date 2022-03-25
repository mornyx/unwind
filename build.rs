fn main() {
    let mut build = cc::Build::new();
    build.file("src/registers/registers.S");
    #[cfg(target_arch = "x86_64")]
    build.define("UNWIND_ARCH_X86_64", "");
    #[cfg(target_arch = "aarch64")]
    build.define("UNWIND_ARCH_AARCH64", "");
    #[cfg(target_os = "linux")]
    build.define("UNWIND_OS_LINUX", "");
    #[cfg(target_os = "macos")]
    build.define("UNWIND_OS_MACOS", "");
    build.compile("registers");
}
