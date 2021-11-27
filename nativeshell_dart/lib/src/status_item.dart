import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:nativeshell/src/util.dart';

import 'api_constants.dart';
import 'api_model_internal.dart';
import 'menu.dart';

class StatusItem {
  StatusItem._({
    required this.handle,
    this.onLeftMouseDown,
    this.onLeftMouseUp,
    this.onRightMouseDown,
    this.onRightMouseUp,
  });

  final StatusItemHandle handle;
  final VoidCallback? onLeftMouseDown;
  final VoidCallback? onLeftMouseUp;
  final VoidCallback? onRightMouseDown;
  final VoidCallback? onRightMouseUp;

  static Future<StatusItem> create({
    VoidCallback? onLeftMouseDown,
    VoidCallback? onLeftMouseUp,
    VoidCallback? onRightMouseDown,
    VoidCallback? onRightMouseUp,
  }) {
    return _StatusItemManager.instance.createStatusItem(
      onLeftMouseDown: onLeftMouseDown,
      onLeftMouseUp: onLeftMouseUp,
      onRightMouseDown: onRightMouseDown,
      onRightMouseUp: onRightMouseUp,
    );
  }

  Future<void> dispose() async {
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

  Future<void> setHighlighted(bool highlighted) async {
    return _StatusItemManager.instance.setHighlighted(this, highlighted);
  }

  Future<void> setImages(List<ImageInfo> images) {
    return _StatusItemManager.instance.setImages(this, images);
  }

  Future<StatusItemGeometry> getGeometry() {
    return _StatusItemManager.instance.getGeometry(this);
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

//
//
//

final _statusItemChannel = MethodChannel(Channels.statusItemManager);

class _StatusItemManager {
  static final instance = _StatusItemManager();
  final items = <int, StatusItem>{};

  _StatusItemManager() {
    _statusItemChannel.setMethodCallHandler(_onMethodCall);
  }

  Future<dynamic> _invoke(String method, dynamic arg) {
    return _statusItemChannel.invokeMethod(method, arg);
  }

  Future<dynamic> _onMethodCall(MethodCall call) async {
    if (call.method == Methods.statusItemOnAction) {
      final action = StatusItemAction.deserialize(call.arguments);
      final item = items[action.handle.value];
      if (item != null) {
        if (action.action == StatusItemActionType.leftMouseDown) {
          item.onLeftMouseDown?.call();
        } else if (action.action == StatusItemActionType.leftMouseUp) {
          item.onLeftMouseUp?.call();
        } else if (action.action == StatusItemActionType.rightMouseDown) {
          item.onRightMouseDown?.call();
        } else if (action.action == StatusItemActionType.rightMouseUp) {
          item.onRightMouseUp?.call();
        }
      }
    }
  }

  Future<StatusItem> createStatusItem({
    VoidCallback? onLeftMouseDown,
    VoidCallback? onLeftMouseUp,
    VoidCallback? onRightMouseDown,
    VoidCallback? onRightMouseUp,
  }) async {
    final handle =
        StatusItemHandle(await _invoke(Methods.statusItemCreate, {}));
    final item = StatusItem._(
        handle: handle,
        onLeftMouseDown: onLeftMouseDown,
        onLeftMouseUp: onLeftMouseUp,
        onRightMouseDown: onRightMouseDown,
        onRightMouseUp: onRightMouseUp);
    items[handle.value] = item;
    return item;
  }

  Future<void> destroyStatusItem(StatusItem item) async {
    items.remove(item.handle.value);
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

  Future<StatusItemGeometry> getGeometry(StatusItem item) async {
    final geometry = await _invoke(Methods.statusItemGetGeometry, {
      'handle': item.handle.value,
    });
    return StatusItemGeometry.deserialize(geometry);
  }

  Future<void> setHighlighted(StatusItem item, bool highlighted) async {
    await _invoke(Methods.statusItemSetHighlighted, {
      'handle': item.handle.value,
      'highlighted': highlighted,
    });
  }
}
