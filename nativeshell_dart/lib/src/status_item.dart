import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:nativeshell/src/util.dart';

import 'api_constants.dart';
import 'api_model_internal.dart';
import 'menu.dart';

class StatusItem {
  StatusItem._({
    required this.handle,
  });

  final StatusItemHandle handle;

  static Future<StatusItem> create() {
    return _StatusItemManager.instance.createStatusItem();
  }

  void dispose() async {
    _checkDisposed();
    await setMenu(null);
    _disposed = true;
    await _StatusItemManager.instance.destroyStatusItem(this);
  }

  Future<void> setImage(AssetImage image) {
    return _StatusItemManager.instance.setImage(this, image);
  }

  Future<void> setMenu(Menu? menu) async {
    if (_menu == menu) {
      return;
    }
    final prev = _menu;
    _menu = menu;
    final handle = await _menu?.state.materialize();
    await _StatusItemManager.instance.setMenu(this, handle);
    await prev?.state.unmaterialize();
  }

  Future<void> setImages(List<ImageInfo> images) {
    return _StatusItemManager.instance.setImages(this, images);
  }

  void _checkDisposed() {
    assert(!_disposed, 'StatusItem is already disposed.');
  }

  bool _disposed = false;
  Menu? _menu;
}

class StatusItemHandle {
  const StatusItemHandle(this.value);

  final int value;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      (other is StatusItemHandle && other.value == value);

  @override
  int get hashCode => value.hashCode;

  @override
  String toString() => 'StatusItemHandle($value)';
}

final _statusItemChannel = MethodChannel(Channels.statusItemManager);

class _StatusItemManager {
  static final instance = _StatusItemManager();

  _StatusItemManager() {
    _statusItemChannel.setMethodCallHandler(_onMethodCall);
  }

  Future<dynamic> _invoke(String method, dynamic arg) {
    return _statusItemChannel.invokeMethod(method, arg);
  }

  Future<dynamic> _onMethodCall(MethodCall call) async {}

  Future<StatusItem> createStatusItem() async {
    final handle =
        StatusItemHandle(await _invoke(Methods.statusItemCreate, {}));
    return StatusItem._(handle: handle);
  }

  Future<void> destroyStatusItem(StatusItem item) async {
    await _invoke(Methods.statusItemDestroy, {'handle': item.handle.value});
  }

  Future<void> setImages(StatusItem item, List<ImageInfo> images) async {
    final imageData = <ImageData>[];
    for (final image in images) {
      imageData.add(await ImageData.fromImage(image.image,
          devicePixelRatio: image.scale));
    }
    final req = {
      'handle': item.handle.value,
      'image': imageData.map((e) => e.serialize()).toList(),
    };
    await _invoke(Methods.statusItemSetImage, req);
  }

  Future<void> setImage(StatusItem item, AssetImage image) async {
    final images = await loadAllImages(image);
    return setImages(item, images);
  }

  Future<void> setMenu(StatusItem item, MenuHandle? menu) async {
    await _invoke(Methods.statusItemSetMenu, {
      'handle': item.handle.value,
      'menu': menu?.value,
    });
  }
}
