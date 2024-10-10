# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Python bindings 🐍 ([#30](https://github.com/developmentseed/cql2-rs/pull/30))
- Docs ([#36](https://github.com/developmentseed/cql2-rs/pull/36))

### Changed

- `SqlQuery` attributes are now public ([#30](https://github.com/developmentseed/cql2-rs/pull/30))
- `Expr::to_json`, `Expr::to_json_pretty`, and `Expr::to_value` now return `Error` instead of `serde_json::Error` ([#37](https://github.com/developmentseed/cql2-rs/pull/37))

## [0.1.0] - 2024-10-08

Initial release.

[Unreleased]: https://github.com/developmentseed/cql-rs/compare/v0.1.0...main
[0.1.0]: https://github.com/developmentseed/cql-rs/releases/tag/v0.1.0
