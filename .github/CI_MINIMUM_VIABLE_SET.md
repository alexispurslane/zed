# Minimum Viable CI for a Solo Project

Deleted 44 CI workflows built for large-team infra. Here's what to bring back.

## 1. CI — test on every PR and push

```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: namespacelabs/nscloud-cache-action@v1
        with:
          cache: rust
          path: ~/.rustup
      - run: ./script/linux
      - run: ./script/download-wasi-sdk
      - run: cargo fmt --all -- --check
      - run: ./script/clippy
      - uses: taiki-e/install-action@nextest
      - run: cargo nextest run --workspace --no-fail-fast --no-tests=warn
```

One job, no orchestration, no matrix. Add `cargo machete` if you want unused-dep checking.

## 2. Release — build and publish on version tags

```yaml
name: Release
on:
  push:
    tags: ["v*"]

jobs:
  bundle-linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: namespacelabs/nscloud-cache-action@v1
        with:
          cache: rust
          path: ~/.rustup
      - run: ./script/linux
      - run: ./script/download-wasi-sdk
      - run: ./script/bundle-linux
      - uses: actions/upload-artifact@v4
        with:
          name: xenomorphic-linux-x86_64.tar.gz
          path: target/release/xenomorphic-linux-x86_64.tar.gz

  bundle-mac:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v5
      - uses: namespacelabs/nscloud-cache-action@v1
        with:
          cache: rust
          path: ~/.rustup
      - run: ./script/bundle-mac aarch64-apple-darwin
      - uses: actions/upload-artifact@v4
        with:
          name: Xenomorphic-aarch64.dmg
          path: target/aarch64-apple-darwin/release/Xenomorphic-aarch64.dmg

  publish:
    needs: [bundle-linux, bundle-mac]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
        with:
          path: artifacts
      - run: gh release upload "$GITHUB_REF_NAME" artifacts/*
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

Add macOS code signing/notarization if you have an Apple Developer certificate.

## 3. (Optional) Nightly — scheduled build for bleeding-edge users

Same shape as Release, triggered by cron + push to the `nightly` tag. Skip if you don't need it.

---

## What not to bring back

Everything else was team/infra plumbing: issue triage, duplicate detection, PR review automation, Slack/Discord notifications, Kubernetes deploys, version bumping, extension ecosystem, AI agents, compliance checks, perf benching, and autofix. You don't have reviewers, on-call rotations, project boards, or an extension marketplace.
