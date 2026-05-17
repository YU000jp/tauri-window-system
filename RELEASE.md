# Release Guide

Use this guide when publishing a new release.

## Quick Checklist

- [ ] Decide the next SemVer version, for example `0.1.0`
- [ ] Confirm `CARGO_REGISTRY_TOKEN` is set as a repository secret from a crates.io account with a verified email
- [ ] Confirm `NPM_TOKEN` is set as a repository secret from an npm granular access token with Bypass 2FA enabled
- [ ] Open GitHub Actions and run `Release`
- [ ] Enter the version without the `v` prefix, for example `0.1.0`
- [ ] Watch `verify`, `publish-rust`, `publish-npm`, and `tag-and-release` finish successfully

## Steps

1. Decide the next version.
   - Use SemVer, for example `0.1.0`, `0.1.1`, or `1.0.0`.
   - Do not include the `v` prefix in the workflow input.
2. Open GitHub Actions and run the `Release` workflow manually.
3. Enter the version, for example `0.1.0`.
4. Review the workflow logs.
   - The workflow checks version consistency first.
   - If the check passes, it runs `verify`, `publish-rust`, `publish-npm`, and `tag-and-release` in order.

## What The Workflow Does

- Verifies the crate and package versions match the input
- Synchronizes the crate and package versions to the workflow input before validation and publish
- Checks the workspace with typecheck, build, and Rust tests
- Verifies the packaged tarball contents
- Uploads npm `dist` artifacts from `verify` and restores them in `publish-npm`
- Publishes the Rust crate to crates.io
- Publishes the npm packages to npm
- Creates the `vX.Y.Z` tag and GitHub Release

## Required Secrets

- `CARGO_REGISTRY_TOKEN` from a crates.io account with a verified email address
- `NPM_TOKEN` from an npm granular access token with Bypass 2FA enabled

## npm Publish Requirement

npm requires either account-level 2FA or a granular access token with Bypass 2FA enabled for publish operations. For CI, use a granular write token with Bypass 2FA enabled and store it as `NPM_TOKEN`.

## crates.io Publish Requirement

crates.io requires the publishing account to have a verified email address. If `cargo publish` fails with a 400 response about email verification, verify the email on the account that owns `CARGO_REGISTRY_TOKEN` and then regenerate the token.
