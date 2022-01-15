import 'package:flutter/rendering.dart';
import 'api_model.dart';
import 'event.dart';
import 'screen_internal.dart';

class Screen {
  Screen({
    required this.id,
    required this.frame,
    required this.workArea,
    required this.scalingFactor,
  });

  static List<Screen> getAllScreens() => ScreenManager.instance.screens;
  static final onScreensChanged = VoidEvent();

  // Returns screen that currently has keyboard focus. In rare circumstances
  // this may be null (i.e. when calling during screen configuration change)
  static Future<Screen?> getMainScreen() =>
      ScreenManager.instance.getMainScreen();

  static Future<Offset> logicalToSystem(Offset logical) async =>
      ScreenManager.instance.logicalToSystem(logical);
  static Future<Offset> systemToLogical(Offset system) async =>
      ScreenManager.instance.systemToLogical(system);

  final int id;
  final Rect frame;
  final Rect workArea;
  final double scalingFactor;

  static Screen deserialize(dynamic screen) {
    final map = screen as Map;
    return Screen(
      id: map['id'],
      frame: RectExt.deserialize(map['frame']),
      workArea: RectExt.deserialize(map['workArea']),
      scalingFactor: map['scalingFactor'],
    );
  }

  dynamic serialize() {
    return {
      'id': id,
      'frame': frame.serialize(),
      'workArea': workArea.serialize(),
      'scalingFactor': scalingFactor,
    };
  }

  @override
  String toString() => serialize().toString();

  @override
  bool operator ==(other) =>
      identical(this, other) ||
      (other is Screen &&
          other.id == id &&
          other.frame == frame &&
          other.workArea == workArea &&
          other.scalingFactor == scalingFactor);

  @override
  int get hashCode => Object.hash(id, frame, workArea, scalingFactor);
}
