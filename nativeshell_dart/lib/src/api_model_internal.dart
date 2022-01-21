import 'dart:typed_data';
import 'dart:ui';

import 'api_model.dart';
import 'menu.dart';
import 'status_item.dart';
import 'util.dart';

class PopupMenuRequest {
  PopupMenuRequest({
    required this.handle,
    required this.position,
    this.trackingRect,
    this.itemRect,
    this.preselectFirst = false,
  });

  final MenuHandle handle;
  final Offset position;
  final Rect? trackingRect;
  final Rect? itemRect;
  final bool preselectFirst;

  dynamic serialize() => {
        'handle': handle.value,
        'position': position.serialize(),
        'trackingRect': trackingRect?.serialize(),
        'itemRect': itemRect?.serialize(),
        'preselectFirst': preselectFirst,
      };
}

class WindowActivationRequest {
  WindowActivationRequest({
    required this.activateApplication,
  });

  final bool activateApplication;

  dynamic serialize() => {
        'activateApplication': activateApplication,
      };
}

class WindowDeactivationRequest {
  WindowDeactivationRequest({
    required this.deactivateApplication,
  });

  final bool deactivateApplication;

  dynamic serialize() => {
        'deactivateApplication': deactivateApplication,
      };
}

class PopupMenuResponse {
  PopupMenuResponse({
    required this.itemSelected,
  });

  static PopupMenuResponse deserialize(dynamic value) {
    final map = value as Map;
    return PopupMenuResponse(itemSelected: map['itemSelected']);
  }

  dynamic serialize() => {
        'itemSelected': itemSelected,
      };

  @override
  String toString() => serialize().toString();

  final bool itemSelected;
}

class HidePopupMenuRequest {
  HidePopupMenuRequest({
    required this.handle,
  });

  final MenuHandle handle;

  dynamic serialize() => {
        'handle': handle.value,
      };
}

class ImageData {
  ImageData({
    required this.width,
    required this.height,
    required this.bytesPerRow,
    required this.data,
    this.devicePixelRatio,
  });

  final int width;
  final int height;
  final int bytesPerRow;
  final Uint8List data;
  final double? devicePixelRatio;

  static Future<ImageData> fromImage(
    Image image, {
    double? devicePixelRatio,
  }) async {
    final bytes = await image.toByteData(format: ImageByteFormat.rawRgba);
    return ImageData(
        width: image.width,
        height: image.height,
        bytesPerRow: image.width * 4,
        data: bytes!.buffer.asUint8List(),
        devicePixelRatio: devicePixelRatio);
  }

  dynamic serialize() => {
        'width': width,
        'height': height,
        'bytesPerRow': bytesPerRow,
        'data': data,
        'devicePixelRatio': devicePixelRatio,
      };
}

class KeyboardKey {
  KeyboardKey({
    required this.platform,
    required this.physical,
    this.logical,
    this.logicalShift,
    this.logicalAlt,
    this.logicalAltShift,
    this.logicalMeta,
  });

  final int platform;
  final int physical;
  final int? logical;
  final int? logicalShift;
  final int? logicalAlt;
  final int? logicalAltShift;
  final int? logicalMeta;

  static KeyboardKey deserialize(dynamic value) {
    final map = value as Map;
    return KeyboardKey(
        platform: map['platform'],
        physical: map['physical'],
        logical: map['logical'],
        logicalShift: map['logicalShift'],
        logicalAlt: map['logicalAlt'],
        logicalAltShift: map['logicalAltShift'],
        logicalMeta: map['logicalMeta']);
  }
}

class KeyboardMap {
  KeyboardMap({
    required this.keys,
  });

  final List<KeyboardKey> keys;

  static KeyboardMap deserialize(dynamic value) {
    final map = value as Map;
    final keys = map['keys'] as List;
    return KeyboardMap(keys: keys.map(KeyboardKey.deserialize).toList());
  }
}

enum StatusItemActionType {
  leftMouseDown,
  leftMouseUp,
  rightMouseDown,
  rightMouseUp,
}

class StatusItemAction {
  StatusItemAction({
    required this.handle,
    required this.action,
    required this.position,
  });

  final StatusItemHandle handle;
  final StatusItemActionType action;
  final Offset position;

  static StatusItemAction deserialize(dynamic value) {
    final map = value as Map;
    return StatusItemAction(
      handle: StatusItemHandle(map['handle']),
      action: enumFromString(StatusItemActionType.values, map['action']),
      position: OffsetExt.deserialize(map['position']),
    );
  }
}
