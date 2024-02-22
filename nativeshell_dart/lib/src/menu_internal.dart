// ignore_for_file: deprecated_member_use

import 'dart:async';
import 'dart:collection';

import 'package:flutter/services.dart';

import 'keyboard_map.dart';
import 'mutex.dart';
import 'util.dart';
import 'api_constants.dart';
import 'menu.dart';

class MenuState {
  MenuState(this.menu);

  final Menu menu;

  static final _mutex = Mutex();

  Future<MenuHandle> materialize([MenuMaterializer? materializer]) async {
    final handle = await _mutex.protect(() async {
      _materializer = materializer;
      return _materializeLocked();
    });
    return handle;
  }

  Future<void> unmaterialize() async {
    await _mutex.protect(() => _unmaterializeLocked());
  }

  Future<void> update() async {
    final res = _mutex.protect(() => _updateLocked());
    return res;
  }

  // fired when replacing app menu; used to release handle in materialize
  static MenuState? _currentAppMenu;

  // macOS specific. Sets this menu as application menu. It will be shown
  // for every window that doesn't have window specific menu.
  Future<void> setAsAppMenu() async {
    if (_currentAppMenu != null) {
      await _currentAppMenu!.unmaterialize();
    }
    _currentAppMenu = this;
    final handle = await materialize();

    await MenuManager.instance().setAppMenu(handle);
  }

  Future<MenuHandle> _materializeLocked() async {
    if (_currentHandle != null) {
      return _currentHandle!;
    } else {
      final removed = <MenuState>[];
      final preserved = <MenuState>[];
      final added = <MenuState>[];

      _currentElements =
          _mergeElements(menu.builder(), removed, preserved, added);

      _materializer ??= DefaultMaterializer();

      final handle =
          await _materializer!.createOrUpdateMenuPre(this, _currentElements);

      final childMaterializer = _materializer!.createChildMaterializer();
      if (childMaterializer != null) {
        for (final element in _currentElements) {
          if (element.item.submenu != null) {
            await element.item.submenu!.state
                ._materializeSubmenu(this, childMaterializer);
          }
        }
      }

      // Listen for keyboard layout changes on top level menu. This is necessary
      // to refresh menu if using accelerators with PhysicalKeys to display
      // correct shortcut to user.
      if (_materializeParent == null) {
        KeyboardMap.onChange.addListener(_keyboardLayoutChanged);
      }

      _currentHandle = await _materializer!
          .createOrUpdateMenuPost(this, _currentElements, handle);
      return _currentHandle!;
    }
  }

  void _keyboardLayoutChanged() {
    update();
  }

  Future<void> _materializeSubmenu(
      MenuState parent, MenuMaterializer materializer) async {
    assert(
        _materializeParent == null ||
            identical(_materializeParent!._transferTarget, parent),
        'Menu can not be moved to another parent while materialized');
    _materializeParent = parent;
    _materializer = materializer;
    await _materializeLocked();
  }

  MenuState? _materializeParent;
  MenuMaterializer? _materializer;

  Future<void> _unmaterializeLocked() {
    if (_materializeParent == null) {
      assert(this == _transferTarget); // top level menu should not be traferred
      KeyboardMap.onChange.removeListener(_keyboardLayoutChanged);
    }
    return _transferTarget._doUnmaterialize();
  }

  Future<void> _doUnmaterialize() async {
    assert(_currentHandle != null && _materializer != null);
    if (_currentHandle != null && _materializer != null) {
      for (final element in _currentElements) {
        if (element.item.submenu?.state._materializeParent == this) {
          await element.item.submenu!.state._unmaterializeLocked();
        }
      }
      _materializeParent = null;
      await _materializer!.destroyMenu(_currentHandle!);
      _materializer = null;
      _currentHandle = null;
    }
    _pastActions.clear();
    _currentElements.clear();
  }

  Future<void> _updateLocked() async {
    final removed = <MenuState>[];
    final preserved = <MenuState>[];
    final added = <MenuState>[];

    if (_materializer == null) {
      return;
    }

    _currentElements =
        _mergeElements(menu.builder(), removed, preserved, added);

    final handle =
        await _materializer!.createOrUpdateMenuPre(this, _currentElements);

    for (final menu in preserved) {
      await menu._updateLocked();
    }
    for (final menu in removed) {
      await menu._unmaterializeLocked();
    }
    if (_currentHandle != null) {
      for (final menu in added) {
        if (_currentHandle != null) {
          await menu._materializeLocked();
        }
      }
    }

    if (_currentHandle != null && _materializer != null) {
      _currentHandle = await _materializer!
          .createOrUpdateMenuPost(this, _currentElements, handle);
    }
  }

  bool _onAction(int itemId) {
    for (final e in _currentElements) {
      if (e.id == itemId && e.item.action != null) {
        e.item.action!();
        return true;
      }
    }
    for (final e in _currentElements) {
      if (e.item.submenu != null && e.item.submenu!.state.onAction(itemId)) {
        return true;
      }
    }
    return false;
  }

  bool onAction(int itemId) {
    if (_transferTarget._onAction(itemId)) {
      return true;
    }
    final pastAction = _pastActions[itemId];
    if (pastAction != null) {
      pastAction();
      return true;
    }
    return false;
  }

  VoidCallback? actionForEvent(RawKeyEvent event) {
    for (final e in _transferTarget._currentElements) {
      if (e.item.action != null &&
          e.item.accelerator != null &&
          e.item.accelerator!.matches(event)) {
        return e.item.action;
      }
      if (e.item.submenu != null) {
        final r = e.item.submenu!.state.actionForEvent(event);
        if (r != null) {
          return r;
        }
      }
    }
    return null;
  }

  List<MenuElement> _currentElements = [];

  // temporarily save action for removed item; this is to ensure that
  // when item is removed right after user selects it, we can still deliver the
  // callback
  final _pastActions = <int, VoidCallback>{};

  MenuHandle? get currentHandle => _transferTarget._currentHandle;

  MenuHandle? _currentHandle;

  List<MenuElement> _mergeElements(
      List<MenuItem> items,
      List<MenuState> outRemoved,
      List<MenuState> outPreserved,
      List<MenuState> outAdded) {
    final res = <MenuElement>[];

    _pastActions.clear();

    final currentByItem = HashMap<MenuItem, MenuElement>.fromEntries(
        _currentElements.map((e) => MapEntry(e.item, e)));

    final currentByMenu = HashMap.fromEntries(_currentElements
        .where((element) => element.item.submenu != null)
        .map((e) => MapEntry(e.item.submenu!, e)));

    // Preserve separators in the order they came; This is useful for cocoa which
    // can not convert existing item to separator
    final currentSeparators =
        _currentElements.where((element) => element.item.separator).toList();

    for (final i in items) {
      MenuElement? existing;
      if (i.separator) {
        if (currentSeparators.isNotEmpty) {
          existing = currentSeparators.removeAt(0);
        }
      } else if (_currentElements.isNotEmpty) {
        // if there is item with this exact submenu, use it
        existing = currentByMenu.remove(i.submenu);

        // otherwise take item with same name but possible different submenu,
        // as long as new item has not been materialized
        if (i.submenu?.state.currentHandle == null) {
          existing ??= currentByItem.remove(i);
        }
      }
      if (existing != null) {
        // We have existing item, but make sure that we can actually reuse it.
        // It must match perfectly and the accelerator label must not have
        // changed (i.e. due to keyboard layout change).
        final id = existing.item == i &&
                existing.acceleratorLabel == i.accelerator?.label
            ? existing.id
            : _nextItemId++;
        res.add(MenuElement(id: id, item: i));
        currentByItem.remove(existing.item);

        if (existing.item.submenu != null) {
          if (!identical(existing.item.submenu, i.submenu)) {
            i.submenu!.state._transferFrom(existing.item.submenu!.state);
          }
          outPreserved.add(i.submenu!.state);
        }
      } else {
        res.add(MenuElement(id: _nextItemId++, item: i));
        if (i.submenu != null) {
          outAdded.add(i.submenu!.state);
        }
      }
    }

    // items not used anymore
    for (final i in currentByItem.values) {
      final submenu = i.item.submenu;
      if (submenu != null) {
        outRemoved.add(submenu.state);
      } else if (i.item.action != null) {
        _pastActions[i.id] = i.item.action!;
      }
    }

    return res;
  }

  MenuState? _transferedTo;

  MenuState get _transferTarget {
    return _transferedTo != null ? _transferedTo!._transferTarget : this;
  }

  void _transferFrom(MenuState oldMenu) {
    assert(_currentHandle == null);
    assert(_currentElements.isEmpty);
    _currentHandle = oldMenu._currentHandle;
    oldMenu._currentHandle = null;

    _currentElements = oldMenu._currentElements;
    oldMenu._currentElements = <MenuElement>[];

    _materializeParent = oldMenu._materializeParent;
    oldMenu._materializeParent = null;

    _materializer = oldMenu._materializer;
    oldMenu._materializer = null;

    _pastActions.addEntries(oldMenu._pastActions.entries);
    oldMenu._pastActions.clear();

    oldMenu._transferedTo = this;

    MenuManager.instance().didTransferMenu(this);
  }
}

class MenuElement {
  MenuElement({
    required this.id,
    required this.item,
  }) : acceleratorLabel = item.accelerator?.label;

  final int id;

  final MenuItem item;
  final String? acceleratorLabel;

  @override
  bool operator ==(Object other) =>
      identical(this, other) || (other is MenuElement && other.id == id);

  @override
  int get hashCode => id.hashCode;

  Map serialize() => {
        'id': id,
        'title': item.title,
        'submenu': item.submenu?.state.currentHandle?.value,
        'enabled': item.action != null || item.submenu != null,
        'separator': item.separator,
        'checkStatus': enumToString(item.checkStatus),
        'role': item.role != null ? enumToString(item.role) : null,
        'accelerator': item.accelerator?.serialize(),
      };
}

abstract class MenuMaterializer {
  FutureOr<MenuHandle?> createOrUpdateMenuPre(
      MenuState menu, List<MenuElement> elements);

  FutureOr<MenuHandle> createOrUpdateMenuPost(
      MenuState menu, List<MenuElement> elements, MenuHandle? handle);

  Future<void> destroyMenu(MenuHandle menu);

  MenuMaterializer? createChildMaterializer();
}

class DefaultMaterializer extends MenuMaterializer {
  @override
  FutureOr<MenuHandle?> createOrUpdateMenuPre(
      MenuState menu, List<MenuElement> elements) {
    return null;
  }

  @override
  FutureOr<MenuHandle> createOrUpdateMenuPost(
      MenuState menu, List<MenuElement> elements, MenuHandle? handle) async {
    final serialized = {
      'role': menu.menu.role != null ? enumToString(menu.menu.role!) : null,
      'items': elements.map((e) => e.serialize()).toList(),
    };

    final handle = menu.currentHandle;

    final res = MenuHandle(
        await MenuManager.instance()._invoke(Methods.menuCreateOrUpdate, {
      'handle': handle?.value,
      'menu': serialized,
    }));
    if (handle != null && handle != res) {
      MenuManager.instance()._activeMenus.remove(handle);
    }
    MenuManager.instance()._activeMenus[res] = menu;
    return res;
  }

  @override
  Future<void> destroyMenu(MenuHandle menuHandle) async {
    await MenuManager.instance()._invoke(Methods.menuDestroy, {
      'handle': menuHandle.value,
    });
    MenuManager.instance()._activeMenus.remove(menuHandle);
  }

  @override
  MenuMaterializer createChildMaterializer() {
    return DefaultMaterializer();
  }
}

final _menuChannel = MethodChannel(Channels.menuManager);

abstract class MenuManagerDelegate {
  void moveToPreviousMenu();
  void moveToNextMenu();
}

class MenuManager {
  static MenuManager instance() => _instance;

  static final _instance = MenuManager();

  MenuManager() {
    _menuChannel.setMethodCallHandler(_onMethodCall);
  }

  Future<dynamic> _invoke(String method, dynamic arg) {
    return _menuChannel.invokeMethod(method, arg);
  }

  Future<dynamic> _onMethodCall(MethodCall call) async {
    if (call.method == Methods.menuOnAction) {
      final handle = MenuHandle(call.arguments['handle'] as int);
      final id = call.arguments['id'] as int;
      final menu = _activeMenus[handle];
      if (menu != null) {
        menu.onAction(id);
      }
    } else if (call.method == Methods.menuOnOpen) {
      final handle = MenuHandle(call.arguments['handle'] as int);
      final menu = _activeMenus[handle];
      if (menu != null) {
        if (menu.menu.onOpen != null) {
          menu.menu.onOpen!();
        }
      }
    } else if (call.method == Methods.menubarMoveToPreviousMenu) {
      for (final d in _delegates) {
        d.moveToPreviousMenu();
      }
    } else if (call.method == Methods.menubarMoveToNextMenu) {
      for (final d in _delegates) {
        d.moveToNextMenu();
      }
    }
  }

  Future<void> setAppMenu(MenuHandle handle) async {
    return _menuChannel.invokeMethod(Methods.menuSetAppMenu, {
      'handle': handle.value,
    });
  }

  void registerDelegate(MenuManagerDelegate delegate) {
    _delegates.add(delegate);
  }

  void unregisterDelegate(MenuManagerDelegate delegate) {
    _delegates.remove(delegate);
  }

  void didTransferMenu(MenuState menu) {
    if (menu.currentHandle != null) {
      _activeMenus[menu.currentHandle!] = menu;
    }
  }

  final _activeMenus = <MenuHandle, MenuState>{};
  final _delegates = <MenuManagerDelegate>[];
}

int _nextItemId = 1;
