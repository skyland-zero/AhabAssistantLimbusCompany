# Upstream synchronization

`origin/main` is the Python upstream and `fork` is the product fork. The Qt
`app/` tree remains deleted in the product branch; conflicts there are
expected. Business code under `module/automation`, `module/config`, `tasks`,
and `utils` is protected by `upstream-boundary.toml`.

## Current baseline

| | |
| --- | --- |
| Upstream tip already reviewed | `origin/main` `ec8aa175` (2026-11-19, PR #1024) |
| Previous baseline | `2fcb1b46` (2026-11-08, PR #1005) |
| Last round | 2026-11-23, batch A (assets `b95cb36d`) and B (code `45249825`) |
| Ledger | [`upstream-merge-ledger.md`](./upstream-merge-ledger.md), "2026-11-23 同步" |

Upstream rewrote `origin/main`'s history after 2026-09: the commit `dev-nsh`
merged on 2026-10-27 (`e968ead5`) is the fork's own OpenCV CI branch, not
upstream content. `git merge-base dev-nsh origin/main` therefore still reports
`d7a6b376` (2026-08-28). Take the baseline from this table and the ledger;
never infer progress from the merge base or `git log <base>..origin/main`.

## Steps

For every upstream release, and at least weekly:

0. Read [`upstream-merge-ledger.md`](./upstream-merge-ledger.md) first. Every
   upstream commit already judged in the ledger keeps its verdict (`已合并` /
   `部分合并` / `已存在` / `放弃`); do not re-litigate it unless the stated
   premise changed. New verdicts are appended per sync round.
1. Fetch `origin/main` and list what it has since the baseline in the ledger
   (`git log <baseline>..origin/main`).
2. Do not merge upstream wholesale. Judge every non-merge commit in the ledger,
   take resources (PNG/YAML) as whole files, and port code by hand so the
   Runner/sidecar architecture (private sessions, native scrcpy decoding,
   product-owned route catalogs) survives; keep the product deletion for Qt
   UI files. Register any edit under a protected prefix in
   `upstream-boundary.toml` with its rationale.
3. Run `uv run python scripts/check_upstream_boundary.py`, the Python tests, and
   `nsh check nsh-app` / `nsh test nsh-app` when the change touches anything the
   sidecar or the Runner consumes.
4. Review `git diff --stat` for each batch before committing; one commit per
   batch, with the upstream short hashes in the message.
5. Append the round's per-commit verdicts to
   [`upstream-merge-ledger.md`](./upstream-merge-ledger.md), including what was
   deliberately rejected and why, and update the baseline table above.

Enable Git rerere locally with `git config rerere.enabled true`. Do not use an
`ours` merge driver for source code because it can silently hide upstream bug
fixes.

