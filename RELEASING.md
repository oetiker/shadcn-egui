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

Entries go under `## [Unreleased]` in `CHANGES.md`, in the subsections
`### New`, `### Changed`, `### Removed` and `### Fixed`. Every pull request
adds an entry there or carries the label `no-changelog`.

After Publish release finishes, the workflow "Web gallery on GitHub Pages"
builds the gallery from the new release and deploys it. To deploy again, or
to deploy a branch, run that workflow by hand; its `ref` input takes a tag
or a branch and defaults to the newest release.

## Publishing to crates.io

The publish workflow (`.github/workflows/release-publish.yml`) uploads the
crates with crates.io Trusted Publishing: no token is stored on GitHub.
Every crate names `oetiker/shadcn-egui` and the workflow
`release-publish.yml` as its trusted publisher (crates.io, the crate's
Settings, Trusted Publishing). The job skips every crate whose version is
already on crates.io, so **Re-run failed jobs** finishes a publish that
stopped part-way. The GitHub release stays a draft until every crate is up.

Trusted Publishing cannot create a crate. A release that brings a new
crate (one that adds a font or icon set) publishes that crate from your
machine with `scripts/publish-crates.sh`:

1. Merge the release pull request. The publish workflow tags `vX.Y.Z` and
   its crates.io job fails on the new crates.
2. Check out the tag and publish with a crates.io token that has the scopes
   `publish-new` and `publish-update`:

   ```bash
   git checkout vX.Y.Z
   CARGO_REGISTRY_TOKEN=... scripts/publish-crates.sh
   ```

   The script publishes `shadcn-egui-core`, then the font and icon crates,
   then `shadcn-egui`, and skips every crate whose version is already on
   crates.io. It refuses to start when a workspace crate is missing from
   that order or when the crates disagree on the version.
3. Register the trusted publisher on each new crate.
4. Use **Re-run failed jobs** on the publish run. It finds every crate on
   crates.io and makes the GitHub release public.

crates.io limits new crates per account: a burst of 5, then one per 10
minutes; new versions of existing crates: a burst of 30, then one per
minute (read 2026-10-11 in the crates.io source; the live values can
differ). A local run that creates many crates hits HTTP 429. The script
then waits until the time crates.io names ("try again after …", else 10
minutes) and retries the same crate, so 33 new crates take about 4 h
40 min and the run ends with `all published`. `NO_WAIT=1` makes it exit
with status 2 instead; re-run it later, it skips what is out. Before such a
release you can ask crates.io (help@crates.io) to raise the account's
new-crate limit for the workspace.

The publish job does not wait: when a release of all 33 crates runs into
the limit for new versions near the end, the job fails. **Re-run failed
jobs** a few minutes later publishes the rest; the job asks crates.io for
each crate first and leaves out every version that is already there.

To see what a publish would do:

```bash
PLAN_ONLY=1 scripts/publish-crates.sh   # the order, offline
DRY_RUN=1 scripts/publish-crates.sh     # skip/publish per crate, asks crates.io
```

### 0.4.0

All 33 crates of 0.4.0 were published from a local machine before the
feature branch reached `main`, from the branch commits `bf368ae` and
`6795db9`; their `.cargo_vcs_info.json` records these commits. The release
then lands like this:

1. Merge the 0.4 pull request into `main` with a **merge commit**, not a
   squash or rebase merge, so the recorded commits stay in the history of
   `main`.
2. The merge commit's **Tag and publish** check fails with "CHANGES.md
   says 0.3.2 but these disagree": `CHANGES.md` has no 0.4.0 heading yet,
   while `Cargo.toml` and `Cargo.lock` already say 0.4.0. This is expected
   and tags nothing.
3. Create release PR refuses a commit that carries a failed check, so land
   one more commit on `main`, a pull request labelled `no-changelog`.
4. Run **Create release PR** on it and choose **feature** (0.3.2 to 0.4.0;
   bugfix would give 0.3.3). Merge the release pull request.
5. The crates.io job of that publish run fails until the trusted publisher
   is registered: its first step exchanges the job's identity for a
   crates.io token, and that exchange needs the registration. Register
   `oetiker/shadcn-egui` and `release-publish.yml` on all 33 crates, then
   use **Re-run failed jobs**. The job finds 0.4.0 of every crate on
   crates.io, publishes nothing and lets the GitHub release go public.

## Checks before a release

The CI runs them; to run them locally:

```bash
scripts/check-version-files.sh
scripts/check-packages.sh
scripts/check-package-size.sh
cargo publish --dry-run --workspace --jobs 4
```

## Adding a font or an icon set

A new font comes from `tools/fontgen` (row in `tools/fontgen/FONTS.tsv`,
then regenerate), a new icon set from `tools/icongen` (row in
`tools/icongen/SETS.tsv`). Its crate also needs two entries in
`.github/repo-infra.json`:

- under `version_files`, one for its line in `[workspace.dependencies]` of
  `Cargo.toml` and one for its entry in `Cargo.lock`, copied from a crate of
  the same family;
- its name under `rust`, `tested_elsewhere`.

`scripts/check-version-files.sh` and the CI name a crate that lacks them.
The crate is new on crates.io, so the next release publishes it from your
machine: see "Publishing to crates.io".

## The Claude Code plugin

`.claude-plugin/plugin.json` and the plugin entry in
`.claude-plugin/marketplace.json` carry the crate version, and the
marketplace entry installs from the tag `v<version>`. Create release PR
bumps all three through `version_files` in `.github/repo-infra.json`. Users
get a new skill version only with a release.
