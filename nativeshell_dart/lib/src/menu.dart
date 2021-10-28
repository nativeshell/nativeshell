import 'package:flutter/material.dart';

import 'accelerator.dart';
import 'menu_internal.dart';

enum MenuItemRole {
  // macOS specific
  about,

  // macOS specific
  hide,

  // macOS specific
  hideOtherApplications,

  // macOS specific
  showAll,

  // macOS specific
  quitApplication,

  // macOS specific
  minimizeWindow,

  // macOS specific
  zoomWindow,

  // macOS specific
  bringAllToFront,
}

enum MenuRole {
  // macOS specific; Menus marked with window will have additional Window specific items in it
  window,

  // macOS specific; Services menu
  services,
}

enum CheckStatus {
  none,
  checkOn,
  checkOff,
  radioOn,
  radioOff,
}

class MenuItem {
  MenuItem({
    required this.title,
    required this.action,
    this.checkStatus = CheckStatus.none,
    this.accelerator,
  })  : separator = false,
        submenu = null,
        role = null;

  MenuItem.menu({
    required this.title,
    required this.submenu,
  })  : separator = false,
        action = null,
        checkStatus = CheckStatus.none,
        role = null,
        accelerator = null;

  MenuItem.children({
    required String title,
    required List<MenuItem> children,
    MenuRole? role,
  }) : this.builder(
          title: title,
          builder: () => children,
          role: role,
        );

  MenuItem.builder({
    required String title,
    required MenuBuilder builder,
    MenuRole? role,
  }) : this.menu(
          title: title,
          submenu: Menu(builder, role: role),
        );

  MenuItem.withRole({
    required MenuItemRole role,
    String? title,
    this.accelerator,
  })  : action = null,
        separator = false,
        checkStatus = CheckStatus.none,
        title = title ?? _titleForRole(role),
        role = role,
        submenu = null;

  MenuItem.separator()
      : title = '',
        action = null,
        separator = true,
        checkStatus = CheckStatus.none,
        role = null,
        submenu = null,
        accelerator = null;

  final String title;
  final MenuItemRole? role;

  final VoidCallback? action;

  final Menu? submenu;

  final bool separator;
  final CheckStatus checkStatus;

  bool get disabled => submenu == null && action == null;

  final Accelerator? accelerator;

  @override
  bool operator ==(dynamic other) =>
      identical(this, other) ||
      (other is MenuItem && separator && other.separator) ||
      (other is MenuItem &&
          title == other.title &&
          (submenu == null) == (other.submenu == null) &&
          role == other.role &&
          checkStatus == other.checkStatus &&
          accelerator == other.accelerator &&
          (action == null) == (other.action == null));

  @override
  int get hashCode => hashValues(title, separator, submenu != null);

  static String _titleForRole(MenuItemRole role) {
    switch (role) {
      case MenuItemRole.about:
        return 'About';
      case MenuItemRole.hide:
        return 'Hide';
      case MenuItemRole.hideOtherApplications:
        return 'Hide Others';
      case MenuItemRole.showAll:
        return 'Show All';
      case MenuItemRole.quitApplication:
        return 'Quit';
      case MenuItemRole.minimizeWindow:
        return 'Minimize';
      case MenuItemRole.zoomWindow:
        return 'Zoom';
      case MenuItemRole.bringAllToFront:
        return 'Bring All to Front';
    }
  }
}

typedef MenuBuilder = List<MenuItem> Function();

class Menu {
  Menu(
    this.builder, {
    this.role,
    this.onOpen,
  }) {
    state = MenuState(this);
  }

  final MenuBuilder builder;
  final MenuRole? role;
  final VoidCallback? onOpen;

  // Internal state of the menu
  late final MenuState state;

  void update() {
    state.update();
  }

  // macOS specific. Sets this menu as application menu. It will be shown
  // for every window that doesn't have window specific menu.
  Future<void> setAsAppMenu() {
    return state.setAsAppMenu();
  }
}

class MenuHandle {
  const MenuHandle(this.value);

  final int value;

  @override
  bool operator ==(Object other) =>
      identical(this, other) || (other is MenuHandle && other.value == value);

  @override
  int get hashCode => value.hashCode;

  @override
  String toString() => 'MenuHandle($value)';
}
