---
changenote: minor
---

Add offline pnpm and Bun text lockfile synchronization for `package.json` releases and local tool diagnostics.
Keep one Node.js lockfile per owning directory; mixed lockfiles now fail before release writes.
Migrate binary `bun.lockb` files to `bun.lock` before versioning.
Reject child pnpm lockfiles below a workspace root before writes. Use the workspace-root lockfile with shared locking and remove stale child lockfiles.
