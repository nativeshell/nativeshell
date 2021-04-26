import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/widgets.dart';
import 'package:nativeshell/nativeshell.dart';
import 'package:nativeshell_demo/file_open_dialog.dart';
import 'package:nativeshell_demo/util.dart';
import 'package:path/path.dart';

import 'drag_drop.dart';
import 'menu_bar_window.dart';
import 'modal.dart';
import 'popup_menu.dart';
import 'veil.dart';

class HomeWindow extends WindowBuilder {
  @override
  Widget build(BuildContext context) {
    return Home();
  }

  @override
  bool get autoSizeWindow => true;

  @override
  Future<void> initializeWindow(
      LocalWindow window, Size intrinsicContentSize) async {
    await super.initializeWindow(window, intrinsicContentSize);
    await window.setStyle(WindowStyle(canResize: false));
    if (Platform.isMacOS) {
      await window.setWindowMenu(Menu(buildMenu));
    }
  }

  static HomeWindow? fromInitData(dynamic initData) {
    if (initData is Map && initData['class'] == 'homeWindow') {
      return HomeWindow();
    }
    return null;
  }

  static dynamic toInitData() => {
        'class': 'homeWindow',
      };

  List<MenuItem> buildMenu() => [
        MenuItem.children(title: 'Main', children: [
          MenuItem.menu(
              title: 'Services',
              submenu: Menu(() => [], role: MenuRole.services)),
          MenuItem.separator(),
          MenuItem.withRole(role: MenuItemRole.hide),
          MenuItem.withRole(role: MenuItemRole.hideOtherApplications),
          MenuItem.withRole(role: MenuItemRole.showAll),
          MenuItem.separator(),
          MenuItem.withRole(
              role: MenuItemRole.quitApplication,
              title: 'Quit NativeShell Examples'),
        ])
      ];
}

class Home extends StatefulWidget {
  @override
  State<StatefulWidget> createState() {
    return _HomeState();
  }
}

class _HomeState extends State<Home> {
  @override
  Widget build(BuildContext context) {
    return IntrinsicWidth(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Container(
            padding: EdgeInsets.all(20),
            color: Colors.blueGrey.shade800,
            child: Text('Nativeshell Examples'),
          ),
          Container(
            padding: EdgeInsets.all(20),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                TextButton(
                    onPressed: () async {
                      final res = await Veil.show(context, () async {
                        final win = await Window.create(
                            ModalWindowBuilder.toInitData());
                        return await win.showModal();
                      });
                      setState(() {
                        _modalWindowResult = res;
                      });
                    },
                    child: Text('Show Modal')),
                if (_modalWindowResult != null)
                  Text('  Result: $_modalWindowResult')
              ],
            ),
          ),
          Padding(
            padding: EdgeInsets.all(20).copyWith(top: 0),
            child: TextButton(
              onPressed: () async {
                if (_dragDropWindow != null) {
                  await _dragDropWindow!.close();
                  _dragDropWindow = null;
                } else {
                  _dragDropWindow =
                      await Window.create(DragDropWindow.toInitData());
                  _dragDropWindow!.closeEvent.addListener(() async {
                    _dragDropWindow = null;
                    setState(() {});
                  });
                }
                setState(() {});
              },
              child: _dragDropWindow == null
                  ? Text('Show Drag & Drop Example')
                  : Text('Hide Drag & Drop Example'),
            ),
          ),
          Padding(
            padding: const EdgeInsets.all(20.0).copyWith(top: 0),
            child: PopupMenu(),
          ),
          Padding(
            padding: const EdgeInsets.all(20.0).copyWith(top: 0),
            child: TextButton(
              onPressed: () async {
                await Window.create(MenuBarWindow.toInitData());
              },
              child: Text('MenuBar'),
            ),
          ),
          Padding(
            padding: const EdgeInsets.all(20.0).copyWith(top: 0),
            child: TextButton(
              onPressed: () async {
                final request =
                    FileOpenRequest(parentWindow: Window.of(context).handle);
                final file = await showFileOpenDialog(request);
                setState(() {
                  if (file != null) {
                    final name = basename(file);
                    fileDialogResult = 'Chosen: $name';
                  } else {
                    fileDialogResult = null;
                  }
                });
              },
              child: Text('Open file dialog'),
            ),
          ),
          AnimatedVisibility(
            visible: fileDialogResult != null,
            duration: Duration(milliseconds: 200),
            alignment: Alignment.center,
            direction: Axis.vertical,
            child: Padding(
                padding: const EdgeInsets.all(20.0).copyWith(top: 0),
                child: Center(child: Text(fileDialogResult ?? ''))),
          ),
        ],
      ),
    );
  }

  dynamic _modalWindowResult;

  String? fileDialogResult;

  Window? _dragDropWindow;
}
