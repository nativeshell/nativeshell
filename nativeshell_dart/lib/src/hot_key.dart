import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'accelerator.dart';
import 'api_constants.dart';
import 'keyboard_map.dart';
import 'mutex.dart';

// Global HotKey
// Registered callback will be invoked when accelerator is pressed regardless
// of whether application has keyboard focus or not.
// Supported on macOS and Windows.
class HotKey {
  HotKey._({
    required _HotKeyHandle handle,
    required this.accelerator,
    required this.callback,
  }) : _handle = handle;

  _HotKeyHandle _handle;
  final Accelerator accelerator;
  final VoidCallback callback;

  static Future<HotKey> create({
    required Accelerator accelerator,
    required VoidCallback callback,
  }) {
    return _HotKeyManager.instance
        .createHotKey(accelerator: accelerator, callback: callback);
  }

  Future<void> dispose() async {
    _checkDisposed();
    _disposed = true;
    await _HotKeyManager.instance.destroyHotKey(this);
  }

  void _checkDisposed() {
    assert(!_disposed, 'HotKey is already disposed.');
  }

  bool _disposed = false;
}

class _HotKeyHandle {
  const _HotKeyHandle(this.value);

  final int value;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      (other is _HotKeyHandle && other.value == value);

  @override
  int get hashCode => value.hashCode;

  @override
  String toString() => 'HotKeyHandle($value)';
}

//
//
//

final _hotKeyChannel = MethodChannel(Channels.hotKeyManager);

class _HotKeyManager {
  static final instance = _HotKeyManager();
  final mutex = Mutex();

  _HotKeyManager() {
    _hotKeyChannel.setMethodCallHandler(_onMethodCall);
    KeyboardMap.onChange.addListener(_keyboardLayoutChanged);
  }

  final _hotKeys = <_HotKeyHandle, HotKey>{};

  Future<dynamic> _invoke(String method, dynamic arg) {
    return _hotKeyChannel.invokeMethod(method, arg);
  }

  Future<dynamic> _onMethodCall(MethodCall call) async {
    if (call.method == Methods.hotKeyOnPressed) {
      await mutex.protect(() async {
        final handle = _HotKeyHandle(call.arguments['handle'] as int);
        final key = _hotKeys[handle];
        if (key != null) {
          key.callback();
        }
      });
    }
  }

  Future<_HotKeyHandle> _registerHotKeyLocked(Accelerator accelerator) async {
    return _HotKeyHandle(await _invoke(Methods.hotKeyCreate, {
      'accelerator': accelerator.serialize(),
      'platformKey': KeyboardMap.current().getPlatformKeyCode(accelerator.key!),
    }));
  }

  Future<HotKey> _createHotKeyLocked({
    required Accelerator accelerator,
    required VoidCallback callback,
  }) async {
    final handle = await _registerHotKeyLocked(accelerator);
    final res =
        HotKey._(handle: handle, accelerator: accelerator, callback: callback);
    _hotKeys[res._handle] = res;
    return res;
  }

  Future<HotKey> createHotKey({
    required Accelerator accelerator,
    required VoidCallback callback,
  }) async {
    return mutex.protect(() =>
        _createHotKeyLocked(accelerator: accelerator, callback: callback));
  }

  Future<void> _destroyHotKeyLocked(_HotKeyHandle handle) async {
    await _invoke(Methods.hotKeyDestroy, {'handle': handle.value});
  }

  Future<void> destroyHotKey(HotKey hotKey) async {
    return mutex.protect(() => _destroyHotKeyLocked(hotKey._handle));
  }

  // Hot-keys are registered on for virtual keys; Re-register hot keys if
  // keyboard layout changed.
  void _keyboardLayoutChanged() {
    mutex.protect(() async {
      final oldKeys = _hotKeys.keys.toList(growable: false);
      for (final key in oldKeys) {
        final hotKey = _hotKeys.remove(key);
        if (hotKey != null) {
          await _destroyHotKeyLocked(hotKey._handle);
          hotKey._handle = await _registerHotKeyLocked(hotKey.accelerator);
          _hotKeys[hotKey._handle] = hotKey;
        }
      }
    });
  }
}
