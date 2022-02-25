fn main() {
    cc::Build::new().file("src/macos/context.s").compile("macos-context");
}
