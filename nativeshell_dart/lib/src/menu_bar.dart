import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import 'menu.dart';
import 'menu_bar_internal.dart';
import 'window.dart';

enum MenuItemState {
  regular,
  hovered,
  selected,
  disabled,
}

typedef MenuItemBuilder = Widget Function(
    BuildContext context, Widget child, MenuItemState state);

class MenuBar extends StatelessWidget {
  const MenuBar({
    Key? key,
    required this.menu,
    required this.itemBuilder,
  }) : super(key: key);

  final Menu menu;
  final MenuItemBuilder itemBuilder;

  @override
  Widget build(BuildContext context) {
    if (defaultTargetPlatform == TargetPlatform.macOS) {
      return _MacOSMenuBar(menu: menu);
    } else {
      return MenuBarInternal(
        menu: menu,
        builder: itemBuilder,
      );
    }
  }
}

class _MacOSMenuBar extends StatefulWidget {
  final Menu menu;

  const _MacOSMenuBar({
    Key? key,
    required this.menu,
  }) : super(key: key);

  @override
  State<StatefulWidget> createState() {
    return _MacOSMenuBarState();
  }
}

class _MacOSMenuBarState extends State<_MacOSMenuBar> {
  @override
  void initState() {
    super.initState();
  }

  @override
  void reassemble() {
    super.reassemble;
    widget.menu.update();
  }

  @override
  void deactivate() {
    super.deactivate();
    final window = Window.of(context);
    if (window.currentWindowMenu == widget.menu) {
      window.setWindowMenu(_previousMenu);
    }
  }

  @override
  void didUpdateWidget(covariant _MacOSMenuBar oldWidget) {
    super.didUpdateWidget(oldWidget);
    _firstBuild = true;
    setState(() {});
  }

  void _updateMenu() async {
    final menu = await Window.of(context).setWindowMenu(widget.menu);
    // only remember the first 'original' menu;
    if (!_havePreviousMenu) {
      _previousMenu = menu;
      _havePreviousMenu = true;
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_firstBuild) {
      _updateMenu();
    }
    return Container(width: 0, height: 0);
  }

  bool _firstBuild = true;
  bool _havePreviousMenu = false;
  Menu? _previousMenu;
}
