Change Log
=======

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

# [unreleased]

- Removed `unsafe` for Async TX constructor again, the special case does not warrant an `unsafe`
  attribute.

# [v0.2.1] 2026-06-08

- Fix for MSRV: v1.87.

# [v0.2.0] 2026-06-08

- TX futures borrow buffer for their lifetime now.
- Constructor is now `unsafe`.
- Async TX write method now returns a future.

# [v0.1.1] 2025-11-28

Minor `Cargo.toml` tweaks

# [v0.1.0] 2025-11-28

Initial release.

[unreleased]: https://egit.irs.uni-stuttgart.de/rust/axi-uartlite/compare/v0.2.0...HEAD
[v0.2.1]: https://egit.irs.uni-stuttgart.de/rust/axi-uartlite/compare/v0.2.0...v0.2.1
[v0.2.0]: https://egit.irs.uni-stuttgart.de/rust/axi-uarglite/compare/v0.1.1...v0.2.0
[v0.1.1]: https://egit.irs.uni-stuttgart.de/rust/axi-uartlite/compare/v0.1.0...v0.1.1
[v0.1.0]: https://egit.irs.uni-stuttgart.de/rust/axi-uartlite/tag/v0.1.0
