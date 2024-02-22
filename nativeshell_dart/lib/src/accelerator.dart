// ignore_for_file: deprecated_member_use

import 'package:flutter/services.dart';
import 'key_interceptor.dart';
import 'keyboard_map.dart';
import 'menu.dart';

// Defines a keyboard shortcut.
// Can be conveniently constructed:
//
// import 'package:nativeshell/accelerators.dart';
// final accelerator1 = cmdOrCtrl + shift + 'e';
// final accelerator2 = alt + f1;
class Accelerator {
  const Accelerator({
    this.key,
    this.alt = false,
    this.control = false,
    this.meta = false,
    this.shift = false,
  });

  final KeyboardKey? key;
  final bool alt;
  final bool control;
  final bool meta;
  final bool shift;

  String get label {
    final k = key;
    if (k is LogicalKeyboardKey) {
      return k.keyLabel;
    } else if (k is PhysicalKeyboardKey) {
      // on macOS CMD (meta) on some keyboards (SVK) resulsts in US key code.
      // So we need to take that into account when generating labels
      // (used for key equivalents on NSMenuItem) otherwise the shortcut won't
      // be matched.
      final logical =
          KeyboardMap.current().getLogicalKeyForPhysicalKey(k, meta: meta);
      if (logical != null) {
        return logical.keyLabel;
      }
    }
    return '??';
  }

  Accelerator operator +(dynamic that) {
    if (that is num) {
      that = '$that';
    }

    if (that is KeyboardKey) {
      that = Accelerator(key: that);
    }

    if (that is String) {
      assert(that.codeUnits.length == 1);
      final lower = that.toLowerCase();
      return this +
          Accelerator(
              key: _keyForCodeUnit(lower.codeUnits[0]), shift: lower != that);
    } else if (that is Accelerator) {
      return Accelerator(
          key: that.key ?? key,
          alt: alt || that.alt,
          shift: shift || that.shift,
          control: control || that.control,
          meta: meta || that.meta);
    } else {
      throw ArgumentError(
          'Argument must be String, Accelerator or single digit number');
    }
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      (other is Accelerator &&
          alt == other.alt &&
          control == other.control &&
          meta == other.meta &&
          shift == other.shift &&
          key == other.key);

  @override
  int get hashCode => Object.hash(alt, control, meta, shift, key);

  bool matches(RawKeyEvent event) {
    final key = this.key;
    if (key != null) {
      final physicalKey = key is PhysicalKeyboardKey
          ? key
          : KeyboardMap.current()
              .getPhysicalKeyForLogicalKey(key as LogicalKeyboardKey);
      return event.isAltPressed == alt &&
          event.isControlPressed == control &&
          event.isMetaPressed == meta &&
          event.isShiftPressed == shift &&
          physicalKey == event.physicalKey;
    } else {
      return false;
    }
  }

  LogicalKeyboardKey _keyForCodeUnit(int codeUnit) {
    final keyId = LogicalKeyboardKey.unicodePlane |
        (codeUnit & LogicalKeyboardKey.valueMask);
    return LogicalKeyboardKey.findKeyByKeyId(keyId) ??
        LogicalKeyboardKey(
          keyId,
        );
  }

  dynamic serialize() => key != null
      ? {
          'label': label,
          'alt': alt,
          'shift': shift,
          'meta': meta,
          'control': control,
        }
      : null;
}

class AcceleratorRegistry {
  AcceleratorRegistry._() {
    KeyInterceptor.instance
        .registerHandler(_handleKeyEvent, stage: InterceptorStage.pre);
  }

  void register(Accelerator accelerator, VoidCallback callback) {
    _accelerators[accelerator] = callback;
  }

  void unregister(Accelerator accelerator) {
    _accelerators.remove(accelerator);
  }

  void registerMenu(Menu menu) {
    _menus.add(menu);
  }

  void unregisterMenu(Menu menu) {
    _menus.remove(menu);
  }

  bool _handleKeyEvent(RawKeyEvent event) {
    var handled = false;
    if (event is RawKeyDownEvent) {
      for (final a in _accelerators.entries) {
        if (a.key.matches(event)) {
          a.value();
          handled = true;
          break;
        }
      }

      if (!handled) {
        for (final m in _menus) {
          final action = m.state.actionForEvent(event);
          if (action != null) {
            action();
            handled = true;
            break;
          }
        }
      }
    }

    return handled;
  }

  final _accelerators = <Accelerator, VoidCallback>{};
  final _menus = <Menu>[];
}

final accelerators = AcceleratorRegistry._();
