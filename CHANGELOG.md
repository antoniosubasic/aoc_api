# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [5.0.1](https://github.com/antoniosubasic/aoc_api/compare/v5.0.0...v5.0.1) - 2026-09-29

### Changed

- drop the unused `async` on the fake transport

### Dependencies

- *(deps)* bump reqwest from 0.13.4 to 0.13.5 in the cargo group
- *(deps)* bump rustls from 0.23.42 to 0.23.45
- *(deps)* bump thiserror from 2.0.20 to 2.0.21 in the cargo group

## [5.0.0](https://github.com/antoniosubasic/aoc_api/compare/v4.1.0...v5.0.0) - 2026-08-19

### Added

- implement `Transport` for references and smart pointers

### Changed

- reject a non-transport where the session is built
- name the transport type parameter on the endpoints

### Dependencies

- *(deps)* bump h2 from 0.4.15 to 0.4.17

### Documentation

- say on the session methods what they can return

### Fixed

- keep a sample index of zero off the wire

## [4.1.0](https://github.com/antoniosubasic/aoc_api/compare/v4.0.0...v4.1.0) - 2026-08-04

### Added

- surface the endpoints as free functions

## [4.0.0](https://github.com/antoniosubasic/aoc_api/compare/v3.0.4...v4.0.0) - 2026-08-04

### Added

- [**breaking**] rebuild the library into modules behind a transport seam
- [**breaking**] require callers to provide their own User-Agent identification
- [**breaking**] mark the remaining public enums non_exhaustive
- report an answer the site never asked for as a verdict
- accept a session cookie pasted with its name

### Documentation

- rewrite the readme for the new api
- name the identification in the error sections that omitted it

### Fixed

- identify this crate to adventofcode.com on every request
- report an expired cookie as such on a page that still returns 200
- count the event that is currently running

### Other

- add tooling configuration
- add agent instructions
