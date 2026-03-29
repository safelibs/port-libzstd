# Offline Regression Fixtures

Phase 6 reuses the checked-in upstream cache under
`original/libzstd-1.5.5+dfsg2/tests/regression/cache/` first.

This directory exists so the phase-6 wrapper has a stable place to overlay any
additional local-only fixtures if the upstream cache ever becomes incomplete.

`results-memoized.csv` is a checked harness output snapshot for the current
tracked source tree. `run-upstream-regression.sh` only uses it when the
companion `results-memoized.source-sha256` matches the current tracked inputs,
and it still compares that snapshot against the preserved upstream
`tests/regression/results.csv` baseline before accepting it.
