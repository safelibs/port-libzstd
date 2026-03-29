# Offline Version Fixtures

This directory keeps the version-compatibility suite offline while reusing
checked-in upstream assets first.

- `manifest.toml` points modern compatibility checks at the upstream golden
  fixtures under `original/libzstd-1.5.5+dfsg2/tests/`.
- `http-v1.*.zst` are checked-in historical release fixtures generated once
  from upstream 1.x release tags against the reused
  `tests/golden-compression/http` sample.
- `hello*` and `helloworld*` are small local round-trip fixtures for the
  packaged safe CLI.
