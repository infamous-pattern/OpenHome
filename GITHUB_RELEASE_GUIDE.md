# GitHub release setup

This repository includes CI and tagged-release workflows.

## First publication

```bash
git init
git add .
git commit -m "Release OpenHome 2.0.0"
git branch -M main
git remote add origin <repository-url>
git push -u origin main

git tag -a v2.0.0 -m "OpenHome 2.0.0"
git push origin v2.0.0
```

Pushing `v2.0.0` starts `.github/workflows/release.yml`. The workflow validates the source, builds x86_64 and aarch64 binaries, creates the universal plugin package, generates source archives and checksums, and publishes the GitHub release.

The workflow uses the repository's built-in `GITHUB_TOKEN` with `contents: write`; no personal access token is required for the tagged release job.
