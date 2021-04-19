import 'dart:ui';

import 'menu.dart';
import 'util.dart';

extension OffsetExt on Offset {
  Map serialize() => {'x': dx, 'y': dy};

  static Offset deserialize(dynamic position) {
    final map = position as Map;
    return Offset(map['x'], map['y']);
  }

  static Offset? maybeDeserialize(dynamic position) {
    return position != null ? OffsetExt.deserialize(position) : null;
  }
}

extension SizeExt on Size {
  Map serialize() => {'width': width, 'height': height};

  static Size deserialize(dynamic position) {
    final map = position as Map;
    return Size(map['width'], map['height']);
  }

  static Size? maybeDeserialize(dynamic size) {
    return size != null ? SizeExt.deserialize(size) : null;
  }
}

extension RectExt on Rect {
  Map serialize() => {
        'x': left,
        'y': top,
        'width': width,
        'height': height,
      };
  static Rect deserialize(dynamic rect) {
    final map = rect as Map;
    return Rect.fromLTWH(map['x'], map['y'], map['width'], map['height']);
  }

  static Rect? maybeDeserialize(dynamic rect) {
    return rect != null ? RectExt.deserialize(rect) : null;
  }
}

enum GeometryPreference {
  preferFrame,
  preferContent,
}

class Geometry {
  Geometry({
    this.frameOrigin,
    this.frameSize,
    this.contentOrigin,
    this.contentSize,
    this.minFrameSize,
    this.maxFrameSize,
    this.minContentSize,
    this.maxContentSize,
  });

  final Offset? frameOrigin;
  final Size? frameSize;
  final Offset? contentOrigin;
  final Size? contentSize;
  final Size? minFrameSize;
  final Size? maxFrameSize;
  final Size? minContentSize;
  final Size? maxContentSize;

  Map serialize() => {
        'frameOrigin': frameOrigin?.serialize(),
        'frameSize': frameSize?.serialize(),
        'contentOrigin': contentOrigin?.serialize(),
        'contentSize': contentSize?.serialize(),
        'minFrameSize': minFrameSize?.serialize(),
        'maxFrameSize': maxFrameSize?.serialize(),
        'minContentSize': minContentSize?.serialize(),
        'maxContentSize': maxContentSize?.serialize(),
      };

  static Geometry deserialize(dynamic value) {
    final map = value as Map;
    return Geometry(
      frameOrigin: OffsetExt.maybeDeserialize(map['frameOrigin']),
      frameSize: SizeExt.maybeDeserialize(map['frameSize']),
      contentOrigin: OffsetExt.maybeDeserialize(map['contentOrigin']),
      contentSize: SizeExt.maybeDeserialize(map['contentSize']),
      minFrameSize: SizeExt.maybeDeserialize(map['minFrameSize']),
      maxFrameSize: SizeExt.maybeDeserialize(map['maxFrameSize']),
      minContentSize: SizeExt.maybeDeserialize(map['minContentSize']),
      maxContentSize: SizeExt.maybeDeserialize(map['maxContentSize']),
    );
  }

  @override
  String toString() {
    return serialize().toString();
  }
}

class GeometryFlags {
  GeometryFlags({
    required this.frameOrigin,
    required this.frameSize,
    required this.contentOrigin,
    required this.contentSize,
    required this.minFrameSize,
    required this.maxFrameSize,
    required this.minContentSize,
    required this.maxContentSize,
  });

  final bool frameOrigin;
  final bool frameSize;
  final bool contentOrigin;
  final bool contentSize;
  final bool minFrameSize;
  final bool maxFrameSize;
  final bool minContentSize;
  final bool maxContentSize;

  Map serialize() => {
        'frameOrigin': frameOrigin,
        'frameSize': frameSize,
        'contentOrigin': contentOrigin,
        'contentSize': contentSize,
        'minFrameSize': minFrameSize,
        'maxFrameSize': maxFrameSize,
        'minContentSize': minContentSize,
        'maxContentSize': maxContentSize,
      };

  static GeometryFlags deserialize(dynamic value) {
    final map = value as Map;
    return GeometryFlags(
        frameOrigin: map['frameOrigin'],
        frameSize: map['frameSize'],
        contentOrigin: map['contentOrigin'],
        contentSize: map['contentSize'],
        minFrameSize: map['minFrameSize'],
        maxFrameSize: map['maxFrameSize'],
        minContentSize: map['minContentSize'],
        maxContentSize: map['maxContentSize']);
  }

  @override
  String toString() {
    return serialize().toString();
  }
}

enum WindowFrame {
  regular,
  noTitle,
  noFrame,
}

class WindowStyle {
  WindowStyle({
    this.frame = WindowFrame.regular,
    this.canResize = true,
    this.canClose = true,
    this.canMinimize = true,
    this.canMaximize = true,
    this.canFullScreen = true,
  });

  final WindowFrame frame;

  final bool canResize;
  final bool canClose;
  final bool canMinimize;
  final bool canMaximize; // ignored on mac
  final bool canFullScreen;

  dynamic serialize() => {
        'frame': enumToString(frame),
        'canResize': canResize,
        'canClose': canClose,
        'canMinimize': canMinimize,
        'canMaximize': canMaximize,
        'canFullScreen': canFullScreen,
      };

  static WindowStyle deserialize(dynamic value) {
    final map = value as Map;
    return WindowStyle(
        frame: enumFromString(
            WindowFrame.values, map['frame'], WindowFrame.regular),
        canResize: map['canResize'],
        canClose: map['canClose'],
        canMinimize: map['canMinimize'],
        canMaximize: map['canMaximize'],
        canFullScreen: map['canFullScreen']);
  }

  @override
  String toString() {
    return serialize().toString();
  }
}

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
