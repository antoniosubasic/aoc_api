# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
