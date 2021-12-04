import 'package:flutter/material.dart';
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
    final screens = await _screenManagerChannel
        .invokeListMethod(Methods.screenManagerGetScreens);
    this.screens =
        screens!.map((screen) => Screen.deserialize(screen)).toList();
    Screen.onScreensChanged.fire();
  }
}
