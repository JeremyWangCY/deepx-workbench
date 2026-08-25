# Contributing

## Development requirements

- Node.js 22 or newer
- pnpm 10 or newer
- Rust stable with `rustfmt` and `clippy`
- WebView2 Runtime on Windows

## Workflow

1. Create a feature branch from `master`.
2. Keep commits focused and use Conventional Commits.
3. Run the validation commands before pushing:
   ```powershell
   pnpm install --frozen-lockfile
   pnpm typecheck
   cargo fmt --all --check --manifest-path src-tauri/Cargo.toml
   cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
   pnpm tauri build --no-bundle
   ```
4. Update `CHANGELOG.md` for user-visible changes.
5. Open a pull request with the behavior change, test evidence, and any compatibility impact.

## Commit format

Use Conventional Commits:

```text
feat(shell): restore the existing window from the tray
fix(runtime): migrate plugins to the shared DSH home
docs(release): document the v0.1.0 installation path
chore(ci): validate tagged releases
```

## Releases

Maintainers push an annotated Semantic Version tag after the changelog and all
version manifests agree. Pushing `vX.Y.Z` starts the Windows release workflow,
which verifies version consistency, builds the NSIS installer, and publishes a
GitHub Release.

```powershell
git tag -a vX.Y.Z -m "DeepX Workbench vX.Y.Z"
git push origin vX.Y.Z
```

Do not reuse a published tag. Increment the version and create a new tag.
