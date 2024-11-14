# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/developmentseed/cql-rs/compare/v0.3.1...main
[0.3.1]: https://github.com/developmentseed/cql-rs/releases/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/developmentseed/cql-rs/releases/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/developmentseed/cql-rs/releases/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/developmentseed/cql-rs/releases/tag/v0.1.0
