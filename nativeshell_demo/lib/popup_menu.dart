import 'dart:async';

import 'package:flutter/material.dart';
import 'package:nativeshell/nativeshell.dart';

class PopupMenu extends StatefulWidget {
  @override
  State<StatefulWidget> createState() {
    return _PopupMenuState();
  }
}

class _PopupMenuState extends State<PopupMenu> {
  int _counter = 10;

  List<MenuItem> _buildMenuItems() => [
        MenuItem.children(title: 'Window Titlebar', children: [
          MenuItem(title: 'Regular', checked: true, action: () {}),
          MenuItem(title: 'Custom (not wired yet)', action: () {}),
        ]),
        MenuItem.separator(),
        MenuItem(title: 'Menu Item 2', action: () {}),
        MenuItem(title: 'Menu Counter $_counter', action: null),
        MenuItem.separator(),
        MenuItem.children(title: 'Submenu', children: [
          MenuItem(title: 'Submenu Item 1', action: () {}),
          MenuItem(title: 'Submenu Item 2', action: () {}),
        ]),
      ];

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onSecondaryTapDown: (e) async {
        final menu = Menu(_buildMenuItems);

        // Menu can be updated while visible
        final timer = Timer.periodic(Duration(milliseconds: 500), (timer) {
          ++_counter;
          menu.update();
        });

        await Window.of(context).showPopupMenu(menu, e.globalPosition);

        timer.cancel();
      },
      child: Container(
        padding: EdgeInsets.all(10),
        decoration: BoxDecoration(border: Border.all(color: Colors.green)),
        child: Text('Right-click for popup menu'),
      ),
    );
  }
}
