---
changenote: minor
---

Remove `.changenote/releases.json` from new releases.
Breaking change: file readers must use verified Action `release-commit` and `releases` outputs.
Upgrade the Action before enabling this CLI.
The first nonempty `version` run removes the legacy file; failed writes use the existing best-effort recovery.
Local `tag` requires complete history and targets the release commit, not a later `HEAD`.
Keep compatible new tooling for file-free publication retries.
