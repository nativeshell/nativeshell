# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.14] - 2022-03-28

- Support for Flutter 3

## [0.1.13] - 2021-10-11

- Added `StatusItem`
- Added `WindowStateFlags` to allow monitoring window state

## [0.1.10] - 2021-10-11

- Breaking change: Renamed `location` field on `DragEvent` to `position`.
- Added `DragEvent.globalPosition`
- Added `DropMonitor` widget

## [0.1.9] - 2021-08-21

- Breaking change: Added WindowLayoutProbe widget required for WindowSizingMode.atLeastIntrinsicSize and WindowSizingMode.sizeToContents
- Added HotKey class for registering global hotkeys
- Added KeyboardMap class (mapping between physical and logical keys + keyboard layout change notification)
- Bugfixes

## [0.1.8] - 2021-06-16

- Performance improvements when opening new windows.
- Window sizing fixes on Linux.

## [0.1.7] - 2021-06-16

- Added Menu.onOpen callback.

## [0.1.6] - 2021-06-13

- Made `WindowState.windowSizingMode` abstract so that it must be specified explicitely.
- All `DragDataKey` constructuctor arguments are now named.
- `DragDataDecode` returns nullable result.
- Added `IntrinsicSizedBox` widget.

## [0.1.5] - 2021-06-07

- Fix window sizing regression.

## [0.1.4] - 2021-06-07

- Replaced `WindowState.autoSizeWindow` with `WindowState.windowSizingMode`.
- Removed `WindowState.requestUpdateConstraints()`, as it is no longer necessary to call it with `WindowSizingMode.atLeastIntrinsicSize` (default value).

## [0.1.3] - 2021-06-03

- Rename `WindowBuilder` to `WindowState`.

## [0.1.0] - 2021-05-29

- Initial release

