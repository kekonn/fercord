# Fercord bot changelog

<!-- next-header -->

## [Unreleased] - ReleaseDate

## [0.3.5] - 2024-12-02
- chore: Update dependencies

## [0.3.4] - 2024-08-28
- chore: Update dependencies

## [0.3.3] - 2024-03-20
- chore: Update dependencies

## [0.3.2] - 2024-01-30
- chore: Update dependencies

## [0.3.1] - 2023-11-29
- chore: Split single bin into workspace
- chore: Update dependencies
- chore: Update build docker to use rust v1.73.0
- chore: Switch from sqlx to sqlx-oldapi
- chore: Move some dependencies to workspace level
- chore: Update dependencies and adopt new version of Poise

## [0.3.0] - 2023-10-03
- chore: Update release pipeline
- fix: Export sqlite database from container
- chore: Add extra PR checks
- feat: Add health check to container

## [0.2.0] - 2023-09-29

### Added
- Docker support
- Compile time multi-database support

### Changed
- Convert SQLx code to use the Any provider
- [BREAKING] Default database will be MariaDB moving forward

### Fixed

## [0.1.0](https://github.com/kekonn/fercord/releases/tag/v0.1.0) - 2023-07-18

### Added
- Reminders
- Setting a server timezone

## Security
- Fixed [CVE-2023-26964](https://github.com/kekonn/fercord/security/dependabot/1) by updating h2


<!-- next-url -->
[Unreleased]: https://github.com/kekonn/fercord/compare/fercord_bot-v0.3.5...HEAD
[0.3.5]: https://github.com/kekonn/fercord/compare/v0.3.4...fercord_bot-v0.3.5
[0.3.4]: https://github.com/kekonn/fercord/compare/v0.3.3...v0.3.4
[0.3.3]: https://github.com/kekonn/fercord/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/kekonn/fercord/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/kekonn/fercord/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/kekonn/fercord/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/kekonn/fercord/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/kekonn/fercord/compare/1c72dea07273f387914ffd122218e27a6a676a9a...v0.1.0