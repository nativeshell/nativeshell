import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import 'api_model.dart';
import 'api_constants.dart';
import 'screen.dart';

final _screenManagerChannel = MethodChannel(Channels.screenManager);

class ScreenManager {
  ScreenManager._() {
    _screenManagerChannel.setMethodCallHandler(_onMethodCall);
  }

  Future<void> init() async {
    await _update();
  }

  Future<Screen?> getMainScreen() async {
    final mainScreenId = await _screenManagerChannel
        .invokeMethod(Methods.screenManagerGetMainScreen);
    return screens
        .cast<Screen?>()
        .firstWhere((screen) => screen?.id == mainScreenId, orElse: () => null);
  }

  Future<Offset> logicalToSystem(Offset logical) async {
    return OffsetExt.deserialize(await _screenManagerChannel.invokeMethod(
        Methods.screenManagerLogicalToSystem, logical.serialize()));
  }

  Future<Offset> systemToLogical(Offset logical) async {
    return OffsetExt.deserialize(await _screenManagerChannel.invokeMethod(
        Methods.screenManagerSystemToLogical, logical.serialize()));
  }

  Future<dynamic> _onMethodCall(MethodCall call) async {
    if (call.method == Methods.screenManagerScreensChanged) {
      // TODO debounce
      await _update();
    }
  }

  static final instance = ScreenManager._();

  List<Screen> screens = [];

  Future<void> _update() async {
    final screens = (await _screenManagerChannel
            .invokeListMethod(Methods.screenManagerGetScreens))!
        .map((screen) => Screen.deserialize(screen))
        .toList()
      ..sort((a, b) {
        final aOffset = a.frame.topLeft;
        final bOffset = b.frame.topLeft;
        if (aOffset.dy == bOffset.dy) {
          return aOffset.dx.compareTo(bOffset.dx);
        } else {
          return aOffset.dy.compareTo(bOffset.dy);
        }
      });
    if (!listEquals(screens, this.screens)) {
      this.screens = screens;
      Screen.onScreensChanged.fire();
    }
  }
}
