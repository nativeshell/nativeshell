import 'package:flutter/painting.dart';
import 'package:flutter/services.dart';

import 'api_constants.dart';
import 'api_model.dart';
import 'api_model_internal.dart';
import 'menu.dart';
import 'screen.dart';
import 'status_item.dart';
import 'util.dart';

final _statusItemChannel = MethodChannel(Channels.statusItemManager);

class StatusItemManager {
  static final instance = StatusItemManager();
  final items = <int, StatusItem>{};

  StatusItemManager() {
    _statusItemChannel.setMethodCallHandler(_onMethodCall);
  }

  Future<void> init() async {
    await _invoke(Methods.statusItemInit);
  }

  Future<dynamic> _invoke(String method, [dynamic arg]) {
    return _statusItemChannel.invokeMethod(method, arg);
  }

  Future<dynamic> _onMethodCall(MethodCall call) async {
    if (call.method == Methods.statusItemOnAction) {
      final action = StatusItemAction.deserialize(call.arguments);
      final item = items[action.handle.value];
      if (item != null) {
        if (action.action == StatusItemActionType.leftMouseDown) {
          item.onLeftMouseDown?.call(action.position);
        } else if (action.action == StatusItemActionType.leftMouseUp) {
          item.onLeftMouseUp?.call(action.position);
        } else if (action.action == StatusItemActionType.rightMouseDown) {
          item.onRightMouseDown?.call(action.position);
        } else if (action.action == StatusItemActionType.rightMouseUp) {
          item.onRightMouseUp?.call(action.position);
        }
      }
    }
  }

  Future<StatusItem> createStatusItem(
      StatusItem Function(StatusItemHandle handle) factory) async {
    final handle =
        StatusItemHandle(await _invoke(Methods.statusItemCreate, {}));
    final item = factory(handle);
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

  Future<void> setHint(StatusItem item, String hint) async {
    await _invoke(Methods.statusItemSetHint, {
      'handle': item.handle.value,
      'hint': hint,
    });
  }

  Future<void> showMenu(
    StatusItem item,
    MenuHandle? menu, {
    required Offset offset,
  }) async {
    await _invoke(Methods.statusItemShowMenu, {
      'handle': item.handle.value,
      'menu': menu?.value,
      'offset': offset.serialize(),
    });
  }

  Future<Rect> getGeometry(StatusItem item) async {
    final geometry = await _invoke(Methods.statusItemGetGeometry, {
      'handle': item.handle.value,
    });
    return RectExt.deserialize(geometry);
  }

  // Screen might be null temporarily - this can happen when connecting or
  // disconnecting displays.
  Future<Screen?> getScreen(StatusItem item) async {
    final screenId = await _invoke(Methods.statusItemGetScreenId, {
      'handle': item.handle.value,
    }) as int;
    final screens = Screen.getAllScreens().cast<Screen?>();
    return screens.firstWhere(
      (screen) => screen!.id == screenId,
      orElse: () => null,
    );
  }

  Future<void> setHighlighted(StatusItem item, bool highlighted) async {
    await _invoke(Methods.statusItemSetHighlighted, {
      'handle': item.handle.value,
      'highlighted': highlighted,
    });
  }
}
