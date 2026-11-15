---
changenote: minor
---

Move the Rust CLI into a four-member Cargo workspace while preserving Cargo and Python distribution builds.
Keep the public `changenote` entry points thin, with private `changenote-core`, `changenote-cli`, and `changenote-skill` implementation crates.
Cargo publication assembles a self-contained public crate; the private crates are not published separately.
To install from a source checkout, use `cargo install --locked --path crates/changenote`; registry installation commands are unchanged.
The source skill moves from `skills/changenote/SKILL.md` to `crates/changenote-skill/SKILL.md`.
Update direct source-path references, or run `changenote skill` to retrieve the bundled Markdown without a source checkout.
