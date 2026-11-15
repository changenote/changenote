//! Literal Android release metadata in a single-line Java properties subset.
use anyhow::{Context, Result, ensure};
use semver::Version;
use std::cmp::Reverse;
use std::ops::Range;

const WHITESPACE: [char; 3] = [' ', '\t', '\u{c}'];

pub(crate) struct GradleProperties {
    content: String,
    version_span: Range<usize>,
    code_span: Range<usize>,
    code: i32,
}

impl GradleProperties {
    pub(crate) fn parse(content: String) -> Result<Self> {
        ensure!(
            !content.starts_with('\u{feff}'),
            "gradle.properties version files do not support a BOM"
        );
        let mut version_span = None;
        let mut code_span = None;
        let mut offset = 0;
        for raw in content.split_inclusive('\n') {
            let start = offset;
            offset += raw.len();
            let line = raw
                .strip_suffix('\n')
                .map_or(raw, |line| line.strip_suffix('\r').unwrap_or(line));
            ensure!(
                !line.contains('\r'),
                "gradle.properties version files require LF or CRLF line endings"
            );
            let property = line.trim_start_matches(WHITESPACE);
            if property.is_empty() || property.starts_with(['#', '!']) {
                continue;
            }
            // Reject escapes throughout active lines, including escaped aliases
            // of the target keys and continuations from unrelated properties.
            ensure!(
                !property.contains('\\'),
                "gradle.properties version files do not support escapes or continuations"
            );
            let key_end = property
                .find([' ', '\t', '\u{c}', '=', ':'])
                .unwrap_or(property.len());
            let key = &property[..key_end];
            let slot = match key {
                "appVersionName" => &mut version_span,
                "appVersionCode" => &mut code_span,
                _ => continue,
            };
            ensure!(slot.is_none(), "duplicate {key} property");
            // Properties permits whitespace before an optional = or : separator.
            // Only leading value whitespace is ignored; trailing whitespace is data.
            let rest = property[key_end..].trim_start_matches(WHITESPACE);
            let value = rest
                .strip_prefix(['=', ':'])
                .unwrap_or(rest)
                .trim_start_matches(WHITESPACE);
            let end = start + line.len();
            *slot = Some(end - value.len()..end);
        }
        let version_span = version_span.context("gradle.properties requires appVersionName")?;
        let code_span = code_span.context("gradle.properties requires appVersionCode")?;
        let text = &content[code_span.clone()];
        let code: i32 = text
            .parse()
            .context("appVersionCode must be a positive 32-bit decimal integer")?;
        ensure!(
            code > 0 && code.to_string() == text,
            "appVersionCode must be a positive canonical decimal integer"
        );
        Ok(Self {
            content,
            version_span,
            code_span,
            code,
        })
    }

    pub(crate) fn version(&self) -> &str {
        &self.content[self.version_span.clone()]
    }

    pub(crate) fn with_version(mut self, version: &Version) -> Result<String> {
        let version = version.to_string();
        if self.version() == version {
            return Ok(self.content);
        }
        let code = self
            .code
            .checked_add(1)
            .context("appVersionCode cannot be incremented beyond 2147483647")?;
        let mut replacements = [
            (self.version_span, version),
            (self.code_span, code.to_string()),
        ];
        replacements.sort_by_key(|(span, _)| Reverse(span.start));
        for (span, value) in replacements {
            self.content.replace_range(span, &value);
        }
        Ok(self.content)
    }
}

#[cfg(test)]
mod tests {
    use super::GradleProperties;
    use semver::Version;

    #[test]
    fn preserves_bytes_with_all_separators_and_either_property_order() {
        for separator in ["=", ":", " ", " \t=\u{c}", "\u{c}:\t", "\t"] {
            for reverse in [false, true] {
                let name = format!("\tappVersionName{separator}9.9.9");
                let code = format!(" appVersionCode{separator}99");
                let (first, second) = if reverse { (code, name) } else { (name, code) };
                let before = format!(
                    "# 設定 \\\r\n! appVersionName=ignored\r\n{first}\r\nother: valeur # literal! \r\n{second}"
                );
                let parsed = GradleProperties::parse(before.clone()).expect("parse properties");
                assert_eq!(parsed.version(), "9.9.9");
                assert_eq!(
                    parsed
                        .with_version(&Version::new(10, 0, 0))
                        .expect("update properties"),
                    before.replace("9.9.9", "10.0.0").replace("99", "100")
                );
            }
        }
    }

    #[test]
    fn unchanged_version_does_not_increment_even_at_the_code_limit() {
        let before = "appVersionName=1.2.3\nappVersionCode=2147483647\n";
        let parsed = GradleProperties::parse(before.to_owned()).expect("parse properties");
        assert_eq!(
            parsed
                .with_version(&Version::new(1, 2, 3))
                .expect("keep existing version"),
            before
        );
    }
}
