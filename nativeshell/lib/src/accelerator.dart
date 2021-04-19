import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'key_interceptor.dart';
import 'menu.dart';

class AcceleratorKey {
  AcceleratorKey(this.key, this.label);

  final LogicalKeyboardKey key;
  final String label;
}

class Accelerator {
  const Accelerator({
    this.key,
    this.alt = false,
    this.control = false,
    this.meta = false,
    this.shift = false,
  });

  final AcceleratorKey? key;
  final bool alt;
  final bool control;
  final bool meta;
  final bool shift;

  Accelerator operator +(dynamic that) {
    if (that is num) {
      that = '$that';
    }

    if (that is String) {
      assert(that.codeUnits.length == 1);
      final lower = that.toLowerCase();
      return this +
          Accelerator(
              key: AcceleratorKey(
                  _keyForCodeUnit(lower.codeUnits[0]), that.toUpperCase()),
              shift: lower != that);
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
  bool operator ==(dynamic other) =>
      identical(this, other) ||
      (other is Accelerator &&
          alt == other.alt &&
          control == other.control &&
          meta == other.meta &&
          shift == other.shift &&
          key?.key == other.key?.key);

  @override
  int get hashCode => hashValues(alt, control, meta, shift, key?.key);

  bool matches(RawKeyEventEx event) {
    final key = this.key?.key;
    return event.altPressed == alt &&
        event.controlPressed == control &&
        event.metaPressed == meta &&
        event.shiftPressed == shift &&
        key != null &&
        (key == event.keyWithoutModifiers || key == event.keyWithoutModifiers2);
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
          'label': key!.label,
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

  bool _handleKeyEvent(RawKeyEventEx event) {
    var handled = false;
    if (event.event is RawKeyDownEvent) {
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
