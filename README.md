# NativeShell (Experimental embedder for Flutter)

![](https://nativeshell.dev/screenshot-dev.png "Screenshot")

## Sponsors

<a href="https://superlist.com"><img src="https://nativeshell.dev/superlist.png" width="300" style="margin-top:20px"/></a>

## Features

- Leverages existing Flutter desktop embedder on each platform
- Unlike Flutter desktop embedders, NativeShell provides consistent platform agnostic API
- Multi-window support
- Window management
    - Adjusting window styles and geometry
    - Modal dialogs
    - Windows can be set to track content size and resize automatically when content changes
- Platform menus (popup menu, menu bar)
- Drag and Drop
- Written in Rust, Flutter build transparently integrated with cargo

## Status

- This is project in a very experimental stage

## Getting started

Prerequisites:

1. [Install Rust](https://www.rust-lang.org/tools/install)
2. [Install Flutter](https://flutter.dev/docs/get-started/install)
3. [Enable Flutter desktop support](https://flutter.dev/desktop#set-up)

Clone and run examples:

```bash
git clone https://github.com/nativeshell/examples.git
cd examples
cargo run
```

For more information read the [introductory post](https://matejknopp.com/post/introducing-nativeshell/) or go to [nativeshell.dev](https://nativeshell.dev).

## Community

Feel free to join us [on Slack](https://join.slack.com/t/superlist-community/shared_invite/zt-10cpx277q-uZ~pmjlTWg9QQzH64OK9_w) or [Discord](https://discord.gg/SrKMdxuuMK) and say hello 👋.
