# Offline Regression Fixtures

Phase 6 reuses the checked-in upstream cache under
`original/libzstd-1.5.5+dfsg2/tests/regression/cache/` first.

This directory exists so the phase-6 wrapper has a stable place to overlay any
additional local-only fixtures if the upstream cache ever becomes incomplete.
No network fetches or corpus regeneration are part of the release gate.

The wrapper is rooted on the prebuilt Phase 4 artifacts:

- `safe/out/install/release-default/` supplies the `zstd` binary under test.
- `safe/out/original-cli/lib/` supplies the helper-library view used by the
  whitebox harness build.
- `safe/out/deb/default/metadata.env` is treated as the freshness contract for
  the staged Debian source tree and package outputs.

`results-memoized.csv` is the checked-in safe-side regression baseline for the
current tracked source tree. `run-upstream-regression.sh` still stages the
upstream cache and uses `tests/regression/regression.out` to drive row
coverage, but it compares the computed results against this safe baseline
first. The companion `results-memoized.source-sha256` is the freshness key used
when the wrapper decides whether the snapshot can be reused directly instead of
recomputing the matrix.
