import 'package:flutter/services.dart';
import 'package:nativeshell/src/api_model_internal.dart' as model;

import 'event.dart';
import 'keyboard_map_internal.dart';

class KeyboardMap {
  // Event fired when current system keyboard layout changes.
  static final onChange = VoidEvent();

  // Retrieves current keyboard map.
  static KeyboardMap current() => KeyboardMapManager.instance.currentMap;

  int? getPlatformKeyCode(KeyboardKey key) {
    if (key is PhysicalKeyboardKey) {
      return _physicalToKey[key.usbHidUsage]?.platform;
    } else if (key is LogicalKeyboardKey) {
      return _logicalToKey[key.keyId]?.platform;
    } else {
      return null;
    }
  }

  PhysicalKeyboardKey? getPhysicalKeyForPlatformKeyCode(int code) {
    final key = _platformToKey[code];
    return key != null ? PhysicalKeyboardKey(key.physical) : null;
  }

  PhysicalKeyboardKey? getPhysicalKeyForLogicalKey(
      LogicalKeyboardKey logicalKey) {
    final key = _logicalToKey[logicalKey.keyId];
    return key != null ? PhysicalKeyboardKey(key.physical) : null;
  }

  LogicalKeyboardKey? getLogicalKeyForPhysicalKey(
    PhysicalKeyboardKey physicalKey, {
    bool shift = false,
    bool alt = false,
    bool meta = false,
  }) {
    final key = _physicalToKey[physicalKey.usbHidUsage];

    if (key == null) {
      return null;
    }

    if (meta && key.logicalMeta != null) {
      return LogicalKeyboardKey(key.logicalMeta!);
    } else if (shift && alt && key.logicalAltShift != null) {
      return LogicalKeyboardKey(key.logicalAltShift!);
    } else if (shift && !alt && key.logicalShift != null) {
      return LogicalKeyboardKey(key.logicalShift!);
    } else if (!shift && alt && key.logicalAlt != null) {
      return LogicalKeyboardKey(key.logicalAlt!);
    } else if (!shift && !alt && key.logical != null) {
      return LogicalKeyboardKey(key.logical!);
    } else {
      return null;
    }
  }

  final Map<int, model.KeyboardKey> _platformToKey;
  final Map<int, model.KeyboardKey> _physicalToKey;
  final Map<int, model.KeyboardKey> _logicalToKey;

  KeyboardMap(this._platformToKey, this._physicalToKey, this._logicalToKey);
}
