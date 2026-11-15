fn main() {
    changenote_cli::main(env!("CARGO_PKG_VERSION"), include_str!("../SKILL.md"));
}
