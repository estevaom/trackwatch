# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-08-02

### Changed

- Made Tidal API credentials optional - the app now works without a `.env` file
- Tidal integration is now an optional enhancement rather than a requirement

### Added

- `provider_factory` module for cleaner provider initialization
- Comprehensive test coverage for provider factory

### Fixed

- Application no longer exits when Tidal credentials are missing
- Improved error handling for missing environment variables

## [0.1.0] - 2025-07-13

### Added

- Initial release of trackwatch
- Terminal music visualizer with ASCII album artwork
- Support for any MPRIS2-compatible media player
- Lyrics display with synchronization
- Album art color palette extraction
- Tidal API integration for metadata enrichment
- Local caching for lyrics and album art

