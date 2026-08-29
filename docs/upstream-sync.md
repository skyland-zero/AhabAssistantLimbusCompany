# Upstream synchronization

`origin/main` is the Python upstream and `fork` is the product fork. The Qt
`app/` tree remains deleted in the product branch; conflicts there are
expected. Business code under `module/automation`, `module/config`, `tasks`,
and `utils` is protected by `upstream-boundary.toml`.

For every upstream release, and at least weekly:

1. Fetch `origin/main` and create `upstream-sync/YYYY-MM-DD` from the product
   branch.
2. Merge `origin/main` with a merge commit. Keep the product deletion for Qt
   UI files; resolve protected business files by taking upstream first and
   replaying only their documented compatibility hook.
3. Run `uv run python scripts/check_upstream_boundary.py`, Python tests, and
   the full GPUI fmt/check/test/clippy suite.
4. Review `git diff --stat origin/main...HEAD` before merging the sync branch.

Enable Git rerere locally with `git config rerere.enabled true`. Do not use an
`ours` merge driver for source code because it can silently hide upstream bug
fixes.

