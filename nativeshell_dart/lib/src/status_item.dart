import 'package:flutter/material.dart';
import 'package:nativeshell/src/status_item_internal.dart';

import 'api_model.dart';
import 'menu.dart';
import 'screen.dart';

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
    return StatusItemManager.instance.createStatusItem((handle) => StatusItem._(
          handle: handle,
          onLeftMouseDown: onLeftMouseDown,
          onLeftMouseUp: onLeftMouseUp,
          onRightMouseDown: onRightMouseDown,
          onRightMouseUp: onRightMouseUp,
        ));
  }

  Future<void> dispose() async {
    _checkDisposed();
    _disposed = true;
    await StatusItemManager.instance.destroyStatusItem(this);
  }

  Future<void> setImage(AssetImage image) {
    return StatusItemManager.instance.setImage(this, image);
  }

  Future<void> setImages(List<ImageInfo> images) {
    return StatusItemManager.instance.setImages(this, images);
  }

  Future<void> showMenu(Menu menu) async {
    final handle = await menu.state.materialize();
    await StatusItemManager.instance.showMenu(this, handle);
    await menu.state.unmaterialize();
  }

  Future<void> setHighlighted(bool highlighted) async {
    return StatusItemManager.instance.setHighlighted(this, highlighted);
  }

  Future<StatusItemGeometry> getGeometry() {
    return StatusItemManager.instance.getGeometry(this);
  }

  Future<Screen?> getScreen() {
    return StatusItemManager.instance.getScreen(this);
  }

  void _checkDisposed() {
    assert(!_disposed, 'StatusItem is already disposed.');
  }

  bool _disposed = false;
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
