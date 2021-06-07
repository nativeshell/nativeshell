# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.4]

- Replaced `WindowState.autoSizeWindow` with `WindowState.windowSizingMode`.
- Removed `WindowState.requestUpdateConstraints()`, as it is no longer necessary to call it with `WindowSizingMode.atLeastIntrinsicSize` (default value).

## [0.1.3] - 2021-06-03

- Rename `WindowBuilder` to `WindowState`.

## [0.1.0] - 2021-05-29

- Initial release

