import 'package:flutter/rendering.dart';
import 'api_model.dart';
import 'event.dart';
import 'screen_internal.dart';

class Screen {
  Screen({
    required this.id,
    required this.main,
    required this.frame,
    required this.visibleFrame,
    required this.scalingFactor,
  });

  static List<Screen> getAllScreens() => ScreenManager.instance.screens;
  static final onScreensChanged = VoidEvent();

  static Future<Offset> logicalToSystem(Offset logical) async =>
      ScreenManager.instance.logicalToSystem(logical);
  static Future<Offset> systemToLogical(Offset system) async =>
      ScreenManager.instance.systemToLogical(system);

  final int id;
  final bool main;
  final Rect frame;
  final Rect visibleFrame;
  final double scalingFactor;

  static Screen deserialize(dynamic screen) {
    final map = screen as Map;
    return Screen(
      id: map['id'],
      main: map['main'],
      frame: RectExt.deserialize(map['frame']),
      visibleFrame: RectExt.deserialize(map['visibleFrame']),
      scalingFactor: map['scalingFactor'],
    );
  }

  dynamic serialize() {
    return {
      'id': id,
      'main': main,
      'frame': frame.serialize(),
      'visibleFrame': visibleFrame.serialize(),
      'scalingFactor': scalingFactor,
    };
  }

  @override
  String toString() => serialize().toString();
}
