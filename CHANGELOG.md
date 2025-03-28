# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.7-beta.0] - 2025-03-28

### Added

- Experimental DuckDB SQL ([#70](https://github.com/developmentseed/cql2-rs/pull/70))

## [0.3.6] - 2025-03-27

### Changed

- Update examples ([#75](https://github.com/developmentseed/cql2-rs/pull/75))
- Further reductions for and/or ([#78](https://github.com/developmentseed/cql2-rs/pull/78))

### Added

- Expose `Expr.matches()` in Python ([#76](https://github.com/developmentseed/cql2-rs/pull/76))
- Expose `Expr.reduce()` in Python ([#79](https://github.com/developmentseed/cql2-rs/pull/79))

## [0.3.5] - 2025-03-12

### Fixed

- Timestamp math ([#67](https://github.com/developmentseed/cql2-rs/pull/67))

## [0.3.4] - 2025-02-21

### Added

- Enable combining expressions via addition ([#68](https://github.com/developmentseed/cql2-rs/pull/68))

## [0.3.3] - 2024-02-18

### Added

- WASM ([#59](https://github.com/developmentseed/cql2-rs/pull/59))
- Match cql2 against JSON ([#55](https://github.com/developmentseed/cql2-rs/pull/55))

## [0.3.2] - 2024-12-09

### Fixed

- Packaging ([#51](https://github.com/developmentseed/cql2-rs/pull/51))

## [0.3.1] - 2024-11-14

### Fixed

- Invalid parse while combining AND and OR ([#47](https://github.com/developmentseed/cql2-rs/pull/47))

## [0.3.0] - 2024-10-14

### Changed

- Use free functions (instead of staticmethods) in the Python API ([#41](https://github.com/developmentseed/cql2-rs/pull/41))

## [0.2.0] - 2024-10-10

### Added

- Python bindings 🐍 ([#30](https://github.com/developmentseed/cql2-rs/pull/30))
- Docs ([#36](https://github.com/developmentseed/cql2-rs/pull/36))

### Changed

- `SqlQuery` attributes are now public ([#30](https://github.com/developmentseed/cql2-rs/pull/30))
- `Expr::to_json`, `Expr::to_json_pretty`, and `Expr::to_value` now return `Error` instead of `serde_json::Error` ([#37](https://github.com/developmentseed/cql2-rs/pull/37))
- Removed `Error::BoonCompile` ([#38](https://github.com/developmentseed/cql2-rs/pull/38))

## [0.1.0] - 2024-10-08

Initial release.

[Unreleased]: https://github.com/developmentseed/cql2-rs/compare/v0.3.7-beta.0...main
[0.3.7-beta.0]: https://github.com/developmentseed/cql2-rs/compare/v0.3.6...v0.3.7-beta.0
[0.3.6]: https://github.com/developmentseed/cql2-rs/compare/v0.3.5...v0.3.6
[0.3.5]: https://github.com/developmentseed/cql2-rs/compare/v0.3.4...v0.3.5
[0.3.4]: https://github.com/developmentseed/cql2-rs/compare/v0.3.3...v0.3.4
[0.3.3]: https://github.com/developmentseed/cql2-rs/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/developmentseed/cql2-rs/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/developmentseed/cql2-rs/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/developmentseed/cql2-rs/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/developmentseed/cql2-rs/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/developmentseed/cql2-rs/tag/v0.1.0

<!-- markdownlint-disable-file MD024 -->
