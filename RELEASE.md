# Release Guide

Use this guide when publishing a new release.

## Quick Checklist

- [ ] Decide the next SemVer version, for example `0.1.0`
- [ ] Update `crates/tauri-plugin-window-system/Cargo.toml`
- [ ] Update `packages/tauri-plugin-window-system-api/package.json`
- [ ] Update `packages/tauri-window-ui/package.json`
- [ ] Push the version bump to `main`
- [ ] Confirm `CARGO_REGISTRY_TOKEN` is set as a repository secret from a crates.io account with a verified email
- [ ] Confirm `NPM_TOKEN` is set as a repository secret
- [ ] Open GitHub Actions and run `Release`
- [ ] Enter the version without the `v` prefix, for example `0.1.0`
- [ ] Watch the workflow finish publish, tag creation, and GitHub Release creation

## Steps

1. Decide the next version.
   - Use SemVer, for example `0.1.0`, `0.1.1`, or `1.0.0`.
   - Do not include the `v` prefix in the workflow input.
2. Update the version fields in the repo.
   - `crates/tauri-plugin-window-system/Cargo.toml`
   - `packages/tauri-plugin-window-system-api/package.json`
   - `packages/tauri-window-ui/package.json`
3. Push the version bump to `main`.
4. Open GitHub Actions and run the `Release` workflow manually.
5. Enter the version, for example `0.1.0`.
6. Review the workflow logs.
   - The workflow checks version consistency first.
   - If the check passes, it runs tests, build, publish, tag creation, and GitHub Release creation.

## What The Workflow Does

- Verifies the crate and package versions match the input
- Checks the workspace with typecheck, build, and Rust tests
- Verifies the packaged tarball contents
- Publishes the Rust crate to crates.io
- Publishes the npm packages to npm
- Creates the `vX.Y.Z` tag and GitHub Release

## Required Secrets

- `CARGO_REGISTRY_TOKEN` from a crates.io account with a verified email address
- `NPM_TOKEN`

## crates.io Publish Requirement

crates.io requires the publishing account to have a verified email address. If `cargo publish` fails with a 400 response about email verification, verify the email on the account that owns `CARGO_REGISTRY_TOKEN` and then regenerate the token.
