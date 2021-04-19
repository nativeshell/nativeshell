import 'dart:io';

import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';

import 'menu.dart';
import 'menu_bar_internal.dart';
import 'window.dart';

class MenuBar extends StatelessWidget {
  const MenuBar({
    Key? key,
    required this.menu,
  }) : super(key: key);

  final Menu menu;

  @override
  Widget build(BuildContext context) {
    if (Platform.isMacOS) {
      return _MacOSMenuBar(menu: menu);
    } else {
      return MenuBarInternal(menu: menu);
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
  void deactivate() {
    super.deactivate();
    final window = Window.of(context);
    if (window.currentWindowMenu == widget.menu) {
      window.setWindowMenu(null);
    }
  }

  @override
  void didUpdateWidget(covariant _MacOSMenuBar oldWidget) {
    super.didUpdateWidget(oldWidget);
    _firstBuild = true;
    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    if (_firstBuild) {
      Window.of(context).setWindowMenu(widget.menu);
    }
    return Container(width: 0, height: 0);
  }

  bool _firstBuild = true;
}
