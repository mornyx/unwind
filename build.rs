#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
const REGISTERS_FILE: &'static str = "src/linux/aarch64/registers.S";

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const REGISTERS_FILE: &'static str = "src/linux/x64/registers.S";

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const REGISTERS_FILE: &'static str = "src/macos/aarch64/registers.S";

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const REGISTERS_FILE: &'static str = "src/macos/x64/registers.S";

fn main() {
    cc::Build::new().file(REGISTERS_FILE).compile("registers");
}
