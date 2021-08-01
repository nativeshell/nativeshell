import 'package:flutter/services.dart';

import 'api_constants.dart';
import 'api_model_internal.dart' as model;
import 'keyboard_map.dart';

final _keyboardMapChannel = MethodChannel(Channels.keyboardMapManager);

class KeyboardMapManager {
  static final instance = KeyboardMapManager._();

  Future<dynamic> _invoke(String method, dynamic arg) {
    return _keyboardMapChannel.invokeMethod(method, arg);
  }

  Future<dynamic> _onMethodCall(MethodCall call) async {
    if (call.method == Methods.keyboardMapOnChanged) {
      _update(model.KeyboardMap.deserialize(call.arguments));
      KeyboardMap.onChange.fire();
    }
  }

  KeyboardMapManager._() {
    _keyboardMapChannel.setMethodCallHandler(_onMethodCall);
  }

  Future<void> init() async {
    final keyboard = await _invoke(Methods.keyboardMapGet, null);
    _update(model.KeyboardMap.deserialize(keyboard));
  }

  void _update(model.KeyboardMap map) {
    final platformToKey = <int, model.KeyboardKey>{};
    final physicalToKey = <int, model.KeyboardKey>{};
    final logicalToKey = <int, model.KeyboardKey>{};

    for (final key in map.keys) {
      platformToKey[key.platform] = key;
      physicalToKey[key.physical] = key;
      if (key.logicalAltShift != null) {
        logicalToKey[key.logicalAltShift!] = key;
      }
      if (key.logicalAlt != null) {
        logicalToKey[key.logicalAlt!] = key;
      }
      if (key.logicalShift != null) {
        logicalToKey[key.logicalShift!] = key;
      }
      if (key.logicalMeta != null) {
        logicalToKey[key.logicalMeta!] = key;
      }
      if (key.logical != null) {
        logicalToKey[key.logical!] = key;
      }
    }

    _currentMap = KeyboardMap(platformToKey, physicalToKey, logicalToKey);
  }

  late KeyboardMap _currentMap;

  KeyboardMap get currentMap => _currentMap;
}
