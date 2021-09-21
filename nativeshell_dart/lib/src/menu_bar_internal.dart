import 'dart:async';

import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'key_interceptor.dart';
import 'menu.dart';
import 'accelerator.dart';
import 'menu_bar.dart';
import 'menu_internal.dart';
import 'window.dart';
import 'window_widget.dart';

class MenuBarInternal extends StatefulWidget {
  final Menu menu;
  final MenuItemBuilder builder;

  const MenuBarInternal({
    Key? key,
    required this.menu,
    required this.builder,
  }) : super(key: key);

  @override
  State<StatefulWidget> createState() {
    return _MenuBarInternalState(menu);
  }
}

enum _State {
  inactive, // Menubar has no focus
  focused, // Menubar is focused, but no menu is expanded
  active, // Menubar is focused and menu is expanded
}

class _MenuBarInternalState extends State<MenuBarInternal>
    implements MenuMaterializer, MenuManagerDelegate {
  _MenuBarInternalState(Menu menu) : _elements = <MenuElement>[] {
    updateMenu(menu);
  }

  @override
  void didUpdateWidget(covariant MenuBarInternal oldWidget) {
    super.didUpdateWidget(oldWidget);
    updateMenu(widget.menu);
  }

  @override
  Widget build(BuildContext context) {
    if (_firstBuild) {
      // ...of(context) can't be called in initState
      WindowState.of(context).registerTapCallback(_onWindowTap);
      _firstBuild = false;
    }

    _keys.clear();

    final widgets = _elements.map((e) {
      final key = GlobalKey();
      _keys[e] = key;

      var itemState = MenuItemState.regular;
      if (_selectedElement == e) {
        itemState = MenuItemState.selected;
      } else if (_hoveredElement == e && _selectedElement == null) {
        itemState = MenuItemState.hovered;
      }
      if (e.item.submenu == null && e.item.action == null) {
        itemState = MenuItemState.disabled;
      }

      return MenuBarItem(
        key: key,
        item: e,
        menuBarState: this,
        itemState: itemState,
        showMnemonics: _showMnemonics,
      );
    }).toList();

    return MouseRegion(
      onExit: (e) {
        _onMouseExit();
      },
      child: Listener(
        onPointerDown: _onPointerDown,
        onPointerUp: _onPointerUp,
        onPointerHover: _onHover,
        onPointerMove: _onHover,
        child: Wrap(
          crossAxisAlignment: WrapCrossAlignment.start,
          children: widgets,
        ),
      ),
    );
  }

  void unfocus() {
    _showMnemonics = false;
    _selectedElement = null;
    _state = _State.inactive;
    if (mounted) {
      setState(() {});
    }
  }

  void focus({bool active = true}) {
    setState(() {
      _state = active ? _State.active : _State.focused;
    });
  }

  void _onPointerDown(PointerEvent event) {
    if (event.buttons != 1) {
      return;
    }
    final e = _elementForEvent(event);
    if (e != null) {
      focus();
      selectItem(e);
    }
  }

  void _onPointerUp(PointerEvent event) {
    if (_state == _State.active &&
        _selectedElement != null &&
        _selectedElement!.item.submenu == null) {
      final e = _elementForEvent(event);
      if (e != null && e == _selectedElement) {
        if (e.item.action != null) {
          e.item.action!();
        }
      }
      unfocus();
    }
  }

  void _onWindowTap(PointerEvent event) {
    if (_state != _State.inactive) {
      final e = _elementForEvent(event);
      if (e == null) {
        unfocus();
      }
    }
  }

  void _onHover(PointerEvent event) {
    if (event.localPosition == Offset.zero) {
      // FIXME(knopp) - This seems to be a bug in windows embedder? Investigate
      return;
    }
    final e = _elementForEvent(event);
    if (e != null && !e.item.disabled) {
      onItemHovered(e);
    }
  }

  MenuElement? _elementForEvent(PointerEvent event) {
    for (final e in _keys.entries) {
      final ro2 = e.value.currentContext!.findRenderObject()! as RenderBox;
      final transform = ro2.getTransformTo(null);
      final rect = Rect.fromLTWH(0, 0, ro2.size.width, ro2.size.height);
      final rectTransformed = MatrixUtils.transformRect(transform, rect);
      if (rectTransformed.contains(event.position)) {
        return e.key;
      }
    }
    return null;
  }

  void _onMouseExit() {
    setState(() {
      _hoveredElement = null;
    });
  }

  @override
  MenuMaterializer? createChildMaterializer() {
    return DefaultMaterializer();
  }

  @override
  FutureOr<MenuHandle> createOrUpdateMenuPost(
      MenuState menu, List<MenuElement> elements, MenuHandle? handle) {
    return handle!;
  }

  @override
  FutureOr<MenuHandle> createOrUpdateMenuPre(
      MenuState menu, List<MenuElement> elements) async {
    _elements = elements;
    final hadSelected = _selectedElement != null;

    if (!_elements.contains(_selectedElement)) {
      _selectedElement = null;
    }
    if (!_elements.contains(_hoveredElement)) {
      _hoveredElement = null;
    }
    if (hadSelected && _selectedElement == null) {
      unfocus();
    }
    if (mounted) {
      setState(() {});
    }
    return MenuHandle(0);
  }

  @override
  Future<void> destroyMenu(MenuHandle menu) async {
    if (mounted) {
      setState(() {
        _elements = <MenuElement>[];
      });
    }
  }

  Menu? _currentMenu;

  void updateMenu(Menu menu) async {
    if (_currentMenu == menu) {
      await menu.state.update();
      return;
    }

    if (_currentMenu != null) {
      accelerators.unregisterMenu(_currentMenu!);
      await _currentMenu!.state.unmaterialize();
    }

    _currentMenu = menu;
    await menu.state.materialize(this);
    accelerators.registerMenu(menu);
  }

  void selectItem(MenuElement item, {bool withKeyboard = false}) async {
    if (_menuVisible == item) {
      unfocus();
      return;
    }

    if (_menuVisible != null &&
        _selectedElement?.item.submenu != null &&
        _selectedElement?.item.submenu!.state.currentHandle != null &&
        item.item.submenu == null) {
      await Window.of(context)
          .hidePopupMenu(_selectedElement!.item.submenu!.state.currentHandle!);
    }

    setState(() {
      _selectedElement = item;
    });

    final cookie = ++_cookie;
    if (item.item.submenu != null && _state == _State.active) {
      await _displayMenu(item, withKeyboard, cookie);
    } else {
      _menuVisible = null;
    }
  }

  Future<void> _displayMenu(
      MenuElement item, bool withKeyboard, int cookie) async {
    if (item.item.submenu == null) {
      return;
    }
    final submenu = item.item.submenu!;

    _menuVisible = item;

    final win = Window.of(context);
    final box = _keys[item]!.currentContext!.findRenderObject() as RenderBox;
    final itemRect = Rect.fromLTWH(0, 0, box.size.width, box.size.height);
    final transform = box.getTransformTo(null);
    final transformed = MatrixUtils.transformRect(transform, itemRect);

    final menubarObject = context.findRenderObject() as RenderBox;
    final menubarRect = Rect.fromLTWH(
        0, 0, menubarObject.size.width, menubarObject.size.height);
    final trackingRect = MatrixUtils.transformRect(
        menubarObject.getTransformTo(null), menubarRect);

    final res = await win.showPopupMenuWithHandle(
      submenu.state.currentHandle!,
      transformed.bottomLeft,
      trackingRect: trackingRect,
      itemRect: transformed,
      preselectFirst: withKeyboard,
    );

    if (res.itemSelected) {
      unfocus();
    }
    await Future.delayed(Duration(milliseconds: 100));
    if (_cookie == cookie) {
      setState(() {
        _menuVisible = null;
        if (_state == _State.active) {
          _state = _State.focused;
        }
      });
    }
  }

  void onItemHovered(MenuElement item) {
    if (_hoveredElement != item) {
      setState(() {
        _hoveredElement = item;
      });
    }

    if (_selectedElement == item) {
      return;
    } else if (_state == _State.active) {
      selectItem(item);
    } else if (_state == _State.focused) {
      setState(() {
        _selectedElement = item;
      });
    }
  }

  bool get _hasEnabledElements {
    return _elements.any((element) => !element.item.disabled);
  }

  bool _onRawKeyEvent(RawKeyEvent event) {
    final hasEnabledElements = _hasEnabledElements;
    var focusRequested = false;

    if (event is RawKeyDownEvent &&
        event.logicalKey == LogicalKeyboardKey.altLeft) {
      _ignoreNextAltKeyUp = false;
      if (hasEnabledElements) {
        setState(() {
          _missedMnemonics = false;
          _showMnemonics = true;
        });
      }
      return false;
    }
    if (event is RawKeyUpEvent) {
      if (event.logicalKey == LogicalKeyboardKey.altLeft && _showMnemonics) {
        if (_state != _State.inactive || _missedMnemonics) {
          if (!_ignoreNextAltKeyUp) {
            unfocus();
          }
          _ignoreNextAltKeyUp = false;
        } else if (!_missedMnemonics && hasEnabledElements) {
          focus(active: false);
          focusRequested = true;
          setState(() {
            _selectedElement = _elements[0];
          });
        }
      }
    }
    if (event is RawKeyDownEvent &&
        event.character != null &&
        (_showMnemonics || _state != _State.inactive)) {
      for (final e in _elements) {
        final mnemonics = Mnemonics.parse(e.item.title);
        if (mnemonics.character != null &&
            mnemonics.character!.toLowerCase() ==
                event.character!.toLowerCase()) {
          if (e.item.submenu != null) {
            _showMnemonics = true;
            _missedMnemonics = false;
            _ignoreNextAltKeyUp = true;
            focus();
            selectItem(e, withKeyboard: true);
          } else if (e.item.action != null) {
            e.item.action!();
            if (_state == _State.focused) {
              unfocus();
            }
          }
          return true;
        }
      }
      if (event.character == ' ') {
        Window.of(context).showSystemMenu();
        unfocus();
        return true;
      }
    }

    if (event is RawKeyDownEvent && _state != _State.inactive) {
      if (event.logicalKey == LogicalKeyboardKey.escape &&
          (_menuVisible == null || _selectedElement?.item.submenu == null)) {
        unfocus();
      } else if (event.logicalKey == LogicalKeyboardKey.arrowLeft) {
        _moveToMenu(-1);
        return true;
      } else if (event.logicalKey == LogicalKeyboardKey.arrowRight) {
        _moveToMenu(1);
        return true;
      } else if (_selectedElement!.item.submenu != null &&
          (event.logicalKey == LogicalKeyboardKey.arrowDown ||
              event.logicalKey == LogicalKeyboardKey.arrowUp ||
              event.logicalKey == LogicalKeyboardKey.enter)) {
        if (_menuVisible == null && _selectedElement != null) {
          focus();
          selectItem(_selectedElement!, withKeyboard: true);
          return true;
        }
      } else if (_selectedElement!.item.submenu == null &&
          event.logicalKey == LogicalKeyboardKey.enter) {
        if (_selectedElement!.item.action != null) {
          _selectedElement!.item.action!();
          unfocus();
        }
      }
    }

    // hasFocus may be false for a bit after requesting focus
    if (_showMnemonics && !focusRequested) {
      _missedMnemonics = true;
    }

    if (_state != _State.inactive) {
      return true;
    } else {
      return false;
    }
  }

  @override
  void initState() {
    super.initState();
    MenuManager.instance().registerDelegate(this);
    // handle keyboard events before flutter event processing
    KeyInterceptor.instance
        .registerHandler(_onRawKeyEvent, stage: InterceptorStage.pre);
    _firstBuild = true;
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    // need to access this in dispose
    windowContext = WindowState.of(context);
  }

  @override
  void dispose() {
    MenuManager.instance().unregisterDelegate(this);
    KeyInterceptor.instance
        .unregisterHandler(_onRawKeyEvent, stage: InterceptorStage.pre);
    if (windowContext != null) {
      windowContext!.unregisterTapCallback(_onWindowTap);
    }
    super.dispose();
  }

  void _moveToMenu(int delta) {
    if (_selectedElement != null) {
      final index = _elements.indexOf(_selectedElement!);
      if (index != -1) {
        var nextIndex = index;
        for (var i = 0; i < _elements.length; ++i) {
          nextIndex = nextIndex + delta;
          if (nextIndex == _elements.length) {
            nextIndex = 0;
          } else if (nextIndex < 0) {
            nextIndex = _elements.length - 1;
          }
          if (!_elements[nextIndex].item.disabled) {
            break;
          }
        }
        if (nextIndex != index) {
          selectItem(_elements[nextIndex], withKeyboard: true);
        }
      }
    }
  }

  @override
  void moveToNextMenu() {
    _moveToMenu(1);
  }

  @override
  void moveToPreviousMenu() {
    _moveToMenu(-1);
  }

  var _state = _State.inactive;

  int _cookie = 0;

  List<MenuElement> _elements;

  // item currently hovered; this is mostly used to keep track of mouse hover
  // and used to restore selected item after losing focus
  MenuElement? _hoveredElement;

  // if unfocused, hovered item; if focused, either hovered or selected by keyboard,
  // depending on what event was latest
  MenuElement? _selectedElement;

  // currently expanded menu
  MenuElement? _menuVisible;

  // Used to retrieve render objects
  final _keys = <MenuElement, GlobalKey>{};

  // whether mnemonics are visible
  bool _showMnemonics = false;

  // key pressed while mnemonics is visible that didn't trigger a menu;
  // when this is true, releasing alt will not focus menubar
  bool _missedMnemonics = false;

  // whether build() is called for the first time
  bool _firstBuild = true;

  // Under normal circumstances, releasing ALT key unfocuses menu; However
  // this is not true when mnemonics key was pressed
  bool _ignoreNextAltKeyUp = false;

  WindowState? windowContext;
}

class Mnemonics {
  Mnemonics(this.text, this.mnemonicIndex);

  static Mnemonics parse(String s) {
    var index = -1;
    var mnemonic = false;
    final text = StringBuffer();
    for (final c in s.characters) {
      if (c == '&') {
        if (!mnemonic) {
          mnemonic = true;
          continue;
        } else {
          text.write('&');
          mnemonic = false;
          continue;
        }
      }
      if (mnemonic) {
        index = text.length;
        mnemonic = false;
      }
      text.write(c);
    }
    return Mnemonics(text.toString(), index);
  }

  String? get character {
    return mnemonicIndex != -1 ? text[mnemonicIndex] : null;
  }

  TextSpan asTextSpan(TextStyle baseStyle, [bool showMnemonics = true]) {
    final index = showMnemonics ? mnemonicIndex : -1;
    return TextSpan(children: [
      if (index > 0)
        TextSpan(
          text: text.substring(0, index),
          style: baseStyle,
        ),
      if (index != -1)
        TextSpan(
            text: text[index],
            style: baseStyle.copyWith(decoration: TextDecoration.underline)),
      if (index < text.length - 1)
        TextSpan(
          text: text.substring(index + 1),
          style: baseStyle,
        ),
    ]);
  }

  final String text;
  final int mnemonicIndex;
}

class MenuBarItem extends StatelessWidget {
  final MenuElement item;
  final _MenuBarInternalState menuBarState;
  final MenuItemState itemState;
  final bool showMnemonics;

  const MenuBarItem({
    Key? key,
    required this.item,
    required this.menuBarState,
    required this.itemState,
    required this.showMnemonics,
  }) : super(key: key);

  @override
  Widget build(BuildContext context) {
    final child = Builder(builder: (context) {
      final mnemonic = Mnemonics.parse(item.item.title);
      return RichText(
        text: mnemonic.asTextSpan(
            DefaultTextStyle.of(context).style, showMnemonics),
      );
    });

    return menuBarState.widget.builder(context, child, itemState);
  }
}
