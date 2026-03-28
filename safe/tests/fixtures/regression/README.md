# Offline Regression Fixtures

Phase 6 reuses the checked-in upstream cache under
`original/libzstd-1.5.5+dfsg2/tests/regression/cache/` first.

This directory exists so the phase-6 wrapper has a stable place to overlay any
additional local-only fixtures if the upstream cache ever becomes incomplete.
