import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:nativeshell/nativeshell.dart';
import 'package:nativeshell/src/api_constants.dart';

// Global HotKey
// Registered callback will be invoked when accelerator is pressed regardless
// of whether application has keyboard focus or not.
// Supported on macOS and Windows.
class HotKey {
  HotKey._({
    required this.handle,
    required this.callback,
  });

  final HotKeyHandle handle;
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

class HotKeyHandle {
  const HotKeyHandle(this.value);

  final int value;

  @override
  bool operator ==(Object other) =>
      identical(this, other) || (other is HotKeyHandle && other.value == value);

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

  _HotKeyManager() {
    _hotKeyChannel.setMethodCallHandler(_onMethodCall);
  }

  final _hotKeys = <HotKeyHandle, HotKey>{};

  Future<dynamic> _invoke(String method, dynamic arg) {
    return _hotKeyChannel.invokeMethod(method, arg);
  }

  Future<dynamic> _onMethodCall(MethodCall call) async {
    if (call.method == Methods.hotKeyOnPressed) {
      final handle = HotKeyHandle(call.arguments['handle'] as int);
      final key = _hotKeys[handle];
      if (key != null) {
        key.callback();
      }
    }
  }

  Future<HotKey> createHotKey({
    required Accelerator accelerator,
    required VoidCallback callback,
  }) async {
    final handle = HotKeyHandle(await _invoke(Methods.hotKeyCreate, {
      'accelerator': accelerator.serialize(),
      'platformKey':
          KeyboardMap.current().getPlatformKeyCode(accelerator.key!.key),
    }));
    final res = HotKey._(handle: handle, callback: callback);
    _hotKeys[res.handle] = res;
    return res;
  }

  Future<void> destroyHotKey(HotKey hotKey) async {
    await _invoke(Methods.hotKeyDestroy, {'handle': hotKey.handle.value});
  }
}
