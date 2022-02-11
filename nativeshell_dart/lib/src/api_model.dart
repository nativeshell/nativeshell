import 'dart:ui';
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

  Geometry copyWith({
    Offset? frameOrigin,
    Size? frameSize,
    Offset? contentOrigin,
    Size? contentSize,
    Size? minFrameSize,
    Size? maxFrameSize,
    Size? minContentSize,
    Size? maxContentSize,
  }) {
    return Geometry(
      frameOrigin: frameOrigin ?? this.frameOrigin,
      frameSize: frameSize ?? this.frameSize,
      contentOrigin: contentOrigin ?? this.contentOrigin,
      contentSize: contentSize ?? this.contentSize,
      minFrameSize: minFrameSize ?? this.minFrameSize,
      maxFrameSize: maxFrameSize ?? this.maxFrameSize,
      minContentSize: minContentSize ?? this.minContentSize,
      maxContentSize: maxContentSize ?? this.maxContentSize,
    );
  }

  Geometry translate(double dx, double dy) {
    return copyWith(
      frameOrigin: frameOrigin?.translate(dx, dy),
      contentOrigin: contentOrigin?.translate(dx, dy),
    );
  }

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
  /// Normal window frame (includes title and can be resizable).
  regular,

  /// Window frame without title (can be resizable).
  noTitle,

  /// No window frame, can not be resizable.
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
    this.alwaysOnTop = false,
    this.alwaysOnTopLevel,
    this.trafficLightOffset,
  });

  final WindowFrame frame;

  final bool canResize;
  final bool canClose;
  final bool canMinimize;
  final bool canMaximize; // ignored on mac
  final bool canFullScreen;
  final bool alwaysOnTop;

  /// macOS only, corresponds to NSWindowLevel / CGWindowLevel
  /// - 0 - normal window
  /// - 3 - floating window, torn off menu
  /// - 8 - modal panel
  /// - 19 - utility window
  /// - 20 - dock window
  /// - 24 - main menu
  /// - 25 - status window
  /// - 101 - popup menu
  /// - 102 - overlay window
  /// - 200 - help window
  /// - 500 - dragging window
  /// - 1000 - screen saver
  final int? alwaysOnTopLevel;

  /// macOS only and only applicable for WindowFrame.noTitle;
  /// Controls the offset of window traffic light.
  final Offset? trafficLightOffset;

  dynamic serialize() => {
        'frame': enumToString(frame),
        'canResize': canResize,
        'canClose': canClose,
        'canMinimize': canMinimize,
        'canMaximize': canMaximize,
        'canFullScreen': canFullScreen,
        'alwaysOnTop': alwaysOnTop,
        'alwaysOnTopLevel': alwaysOnTopLevel,
        'trafficLightOffset': trafficLightOffset?.serialize(),
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
        canFullScreen: map['canFullScreen'],
        alwaysOnTop: map['alwaysOnTop'],
        alwaysOnTopLevel: map['alwaysOnTopLevel'],
        trafficLightOffset:
            OffsetExt.maybeDeserialize(map['trafficLightOffset']));
  }

  @override
  String toString() {
    return serialize().toString();
  }
}

// MacOS specific;
class WindowCollectionBehavior {
  final bool canJoinAllSpaces;
  final bool moveToActiveSpace;
  final bool managed;
  final bool transient;
  final bool stationary;
  final bool participatesInCycle;
  final bool ignoresCycle;
  final bool fullScreenPrimary;
  final bool fullScreenAuxiliary;
  final bool fullScreenNone;
  final bool allowsTiling;
  final bool disallowsTiling;

  WindowCollectionBehavior({
    this.canJoinAllSpaces = false,
    this.moveToActiveSpace = false,
    this.managed = false,
    this.transient = false,
    this.stationary = false,
    this.participatesInCycle = false,
    this.ignoresCycle = false,
    this.fullScreenPrimary = false,
    this.fullScreenAuxiliary = false,
    this.fullScreenNone = false,
    this.allowsTiling = false,
    this.disallowsTiling = false,
  });

  dynamic serialize() => {
        'canJoinAllSpaces': canJoinAllSpaces,
        'moveToActiveSpace': moveToActiveSpace,
        'managed': managed,
        'transient': transient,
        'stationary': stationary,
        'participatesInCycle': participatesInCycle,
        'ignoresCycle': ignoresCycle,
        'fullScreenPrimary': fullScreenPrimary,
        'fullScreenAuxiliary': fullScreenAuxiliary,
        'fullScreenNone': fullScreenNone,
        'allowsTiling': allowsTiling,
        'disallowsTiling': disallowsTiling,
      };
}

enum BoolTransition {
  no,
  noToYes,
  yes,
  yesToNo,
}

class WindowStateFlags {
  bool get maximized =>
      maximizedTransition == BoolTransition.yes ||
      maximizedTransition == BoolTransition.noToYes;

  bool get minimized =>
      minimizedTransition == BoolTransition.yes ||
      minimizedTransition == BoolTransition.noToYes;

  bool get fullScreen =>
      fullScreenTransition == BoolTransition.yes ||
      fullScreenTransition == BoolTransition.noToYes;

  final bool active;

  final BoolTransition maximizedTransition;
  final BoolTransition minimizedTransition;
  final BoolTransition fullScreenTransition;

  WindowStateFlags({
    required this.maximizedTransition,
    required this.minimizedTransition,
    required this.fullScreenTransition,
    required this.active,
  });

  static WindowStateFlags deserialize(dynamic value) {
    final map = value as Map;
    return WindowStateFlags(
        maximizedTransition: enumFromString(
            BoolTransition.values, map['maximized'], BoolTransition.no),
        minimizedTransition: enumFromString(
            BoolTransition.values, map['minimized'], BoolTransition.no),
        fullScreenTransition: enumFromString(
            BoolTransition.values, map['fullScreen'], BoolTransition.no),
        active: map['active']);
  }

  dynamic serialize() => {
        'maximized': enumToString(maximizedTransition),
        'minimized': enumToString(minimizedTransition),
        'fullScreen': enumToString(fullScreenTransition),
        'active': active,
      };

  @override
  String toString() {
    return serialize().toString();
  }
}
