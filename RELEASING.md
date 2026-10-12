# Releasing shadcn-egui

A release publishes every crate of the workspace to crates.io at one
version: `shadcn-egui-core`, the font crates `shadcn-egui-font-*`, the icon
crates `shadcn-egui-icons-*` and `shadcn-egui`. All crates share the
version in the root `Cargo.toml`.

## Cutting a release

1. Check that `main` is green.
2. In the repository's Actions tab, run **Create release PR** and choose
   bugfix, feature or major. Before 1.0, a breaking change is a feature
   release. The workflow rolls `## [Unreleased]` in `CHANGES.md` into the
   new version, writes the version into `Cargo.toml` (`[workspace.package]`
   and every `version = "..."` in `[workspace.dependencies]`) and
   `Cargo.lock`, runs CI and opens a pull request.
3. Merge that pull request. The publish workflow tags `vX.Y.Z`, publishes
   the crates and then makes the GitHub release public.

Do not push tags by hand, and do not add release headers to `CHANGES.md`:
the workflows do both.

## Publishing to crates.io

The publish job runs `scripts/publish-crates.sh` with the repository
secret `CARGO_REGISTRY_TOKEN` (a crates.io token with the scopes
`publish-new` and `publish-update`). The script publishes
`shadcn-egui-core`, then the font and icon crates, then `shadcn-egui`, and
skips every crate whose version is already on crates.io. It refuses to
start when a workspace crate is missing from that order or when the crates
disagree on the version.

crates.io limits new crates per account: a burst of 5, then one per 10
minutes; new versions of existing crates: a burst of 30, then one per
minute (read 2026-10-11 in the crates.io source; the live values can
differ). A release that creates many crates, like 0.4.0 or one that adds a
font or icon set, stops with HTTP 429 and exit status 2. Wait, then use
**Re-run failed jobs** on the publish run until its log ends with
`all published`. Before such a release you can ask crates.io
(help@crates.io) to raise the account's new-crate limit for the workspace.

To see what a publish would do:

```bash
PLAN_ONLY=1 scripts/publish-crates.sh   # the order, offline
DRY_RUN=1 scripts/publish-crates.sh     # skip/publish per crate, asks crates.io
```

## Checks before a release

The CI runs them; to run them locally:

```bash
scripts/check-packages.sh
scripts/check-package-size.sh
cargo publish --dry-run --workspace --jobs 4
```

## Adding a font or an icon set

A new font comes from `tools/fontgen` (row in `tools/fontgen/FONTS.tsv`,
then regenerate), a new icon set from `tools/icongen` (row in
`tools/icongen/SETS.tsv`). Its crate is new on crates.io, so the next
release creates a crate: see the rate limit above.

## The Claude Code plugin

`.claude-plugin/plugin.json` and `.claude-plugin/marketplace.json` stay at
version `0.3.2` and are not part of the crate release. Do not bump them
until the skill is rewritten for the crate.
