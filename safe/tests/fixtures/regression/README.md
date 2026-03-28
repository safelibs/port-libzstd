# Offline Regression Fixtures

Phase 6 reuses the checked-in upstream cache under
`original/libzstd-1.5.5+dfsg2/tests/regression/cache/` first.

This directory exists so the phase-6 wrapper has a stable place to overlay any
additional local-only fixtures if the upstream cache ever becomes incomplete.

`results-safe.csv` stores the phase-6 offline regression result table for the
current safe source fingerprint recorded in `results-safe.sha256`. The wrapper
reuses that snapshot when the relevant safe-side sources are unchanged and
falls back to recomputing the matrix when they drift.
