import 'dart:async';

import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter/src/widgets/framework.dart';
import 'package:nativeshell/accelerators.dart';
import 'package:nativeshell/nativeshell.dart';

int counter = 0;

class MenuBarWindow extends WindowBuilder {
  List<MenuItem> buildMenu() => [
        MenuItem.children(title: '&Fist Item', children: [
          MenuItem(title: 'Fist Item $counter', action: null),
          MenuItem(
              title: 'Second Item',
              action: () {
                print('Second');
              },
              accelerator: cmdOrCtrl + 'r'),
        ]),
        MenuItem.children(title: 'Second &item', children: [
          MenuItem(title: 'Fist && Item', action: () {}),
          MenuItem(
              title: 'S&econd Item',
              accelerator: shift + tab,
              action: () {
                print('SECOND');
              }),
        ]),
        MenuItem(
            title: 'A&ction Item',
            action: () {
              print('Action');
            }),
        MenuItem(title: 'Action Item Disabled', action: null),
        MenuItem.children(title: '&Third item', children: [
          MenuItem(
              title: 'Fist Item',
              action: () {
                print('FIRST!');
              }),
          MenuItem.children(title: 'Second Item', children: [
            MenuItem(
                title: '&Fist Item',
                action: () {
                  print('>> First');
                }),
            MenuItem(
                title: 'Second Item',
                accelerator: alt + shift + '3',
                action: () {
                  print('>> Second');
                }),
          ]),
          MenuItem(
              title: 'Third Item',
              action: () {
                print('Third!');
              }),
        ])
      ];

  final focus = FocusNode();

  @override
  Widget build(BuildContext context) {
    final menu = Menu(buildMenu);
    Timer.periodic(Duration(milliseconds: 100), (timer) {
      ++counter;
      menu.update();
    });
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Container(
          color: Colors.blueGrey,
          child: MenuBar(menu: menu),
        ),
        Expanded(
          child: Center(
            child: Material(
              child: TextField(
                autofocus: true,
                focusNode: focus,
              ),
            ),
          ),
        ),
      ],
    );
  }

  static MenuBarWindow? fromInitData(dynamic initData) {
    if (initData is Map && initData['class'] == 'menuBar') {
      return MenuBarWindow();
    }
    return null;
  }

  static dynamic toInitData() => {
        'class': 'menuBar',
      };
}
