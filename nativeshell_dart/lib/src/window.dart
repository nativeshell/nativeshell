import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:pedantic/pedantic.dart';

import 'api_model.dart';
import 'api_model_internal.dart';
import 'event.dart';
import 'menu.dart';
import 'screen.dart';
import 'util.dart';
import 'window_manager.dart';
import 'window_method_channel.dart';
import 'api_constants.dart';
import 'window_widget.dart';

class WindowHandle {
  const WindowHandle(this.value);

  final int value;

  @override
  bool operator ==(Object other) =>
      identical(this, other) || (other is WindowHandle && other.value == value);

  static const invalid = WindowHandle(-1);

  @override
  int get hashCode => value.hashCode;

  @override
  String toString() => 'WindowHandle($value)';
}

class Window {
  Window(this.handle);

  final WindowHandle handle;

  Future<void> show() async {
    if (_visible == true) {
      return;
    }
    _showCompleter ??= Completer();
    unawaited(_invokeMethod(Methods.windowShow));
    return _showCompleter!.future;
  }

  Future<dynamic> showModal() async {
    final res = await _invokeMethod(Methods.windowShowModal);
    return res;
  }

  Future<void> close() async {
    await _invokeMethod(Methods.windowClose);
  }

  Future<void> hide() async {
    _visible = null;
    await _invokeMethod(Methods.windowHide);
  }

  Future<bool> activate({bool activateApplication = true}) async {
    return await _invokeMethod(
        Methods.windowActivate,
        WindowActivationRequest(activateApplication: activateApplication)
            .serialize());
  }

  Future<bool> deactivate({bool deactivateApplication = false}) async {
    return await _invokeMethod(
        Methods.windowDeactivate,
        WindowDeactivationRequest(deactivateApplication: deactivateApplication)
            .serialize());
  }

  Future<GeometryFlags> setGeometry(Geometry request,
      [GeometryPreference preference =
          GeometryPreference.preferContent]) async {
    return GeometryFlags.deserialize(await _invokeMethod(
        Methods.windowSetGeometry, {
      'geometry': request.serialize(),
      'preference': enumToString(preference)
    }));
  }

  Future<Geometry> getGeometry() async {
    return Geometry.deserialize(await _invokeMethod(Methods.windowGetGeometry));
  }

  Future<GeometryFlags> supportedGeometry() async {
    return GeometryFlags.deserialize(
        await _invokeMethod(Methods.windowSupportedGeometry));
  }

  // Screen might be null temporarily - this can happen when connecting or
  // disconnecting displays.
  Future<Screen?> getScreen() async {
    final screenId = await _invokeMethod(Methods.windowGetScreenId) as int;
    final screens = Screen.getAllScreens().cast<Screen?>();
    return screens.firstWhere(
      (screen) => screen!.id == screenId,
      orElse: () => null,
    );
  }

  Future<void> setTitle(String title) {
    return _invokeMethod(Methods.windowSetTitle, title);
  }

  Future<void> setStyle(WindowStyle style) {
    return _invokeMethod(Methods.windowSetStyle, style.serialize());
  }

  Future<String> savePositionToString() async {
    return await _invokeMethod(Methods.windowSavePositionToString);
  }

  Future<void> restorePositionFromString(String position) async {
    return _invokeMethod(Methods.windowRestorePositionFromString, position);
  }

  Future<WindowStateFlags> getWindowStateFlags() async {
    return WindowStateFlags.deserialize(
        await _invokeMethod(Methods.windowGetWindowStateFlags));
  }

  /// If [minimized] is `true` and window is currently not minimized, minimizes
  /// the window.
  ///
  /// If [minimized] is `false` and window is currently minimized, unminimizes
  /// the window.
  Future<void> setMinimized(bool minimized) async {
    await _invokeMethod(Methods.windowSetMinimized, minimized);
  }

  /// If [maximized] is `true` and window is currently not maximized, maximizes
  /// the window.
  ///
  /// If [maximized] is `false` and window is currently maximized, restores the
  /// original size.
  Future<void> setMaximized(bool maximized) async {
    await _invokeMethod(Methods.windowSetMaximized, maximized);
  }

  /// If [fullScreen] is `true` and window is currently not full screen, toggles
  /// fullscreen mode. Only applicable on platforms that support full screen
  /// modes.
  ///
  /// If [fullScreen] is `false` and window is currently full screen, restores
  /// original window size.
  Future<void> setFullScreen(bool fullScreen) async {
    await _invokeMethod(Methods.windowSetFullScreen, fullScreen);
  }

  // MacOS specific;
  Future<void> setCollectionBehavior(WindowCollectionBehavior behavior) {
    return _invokeMethod(
        Methods.windowSetCollectionBehavior, behavior.serialize());
  }

  static LocalWindow of(BuildContext context) => WindowState.of(context).window;

  static LocalWindow? maybeOf(BuildContext context) =>
      WindowState.maybeOf(context)?.window;

  static Window? fromHandle(WindowHandle handle) {
    return WindowManager.instance.getWindow(handle);
  }

  static Future<Window> create(
    dynamic initData, {
    // Hint to window manager to not optimize for faster display of this window
    // at the expense of current window.
    bool invisibleWindowHint = false,
  }) {
    return WindowManager.instance
        .createWindow(initData, invisibleWindowHint: invisibleWindowHint);
  }

  final visibilityChangedEvent = Event<bool>();
  final closeRequestEvent = VoidEvent();
  final closeEvent = VoidEvent();
  final windowStateFlagsEvent = Event<WindowStateFlags>();

  void onMessage(String message, dynamic arguments) {
    if (message == Events.windowInitialize) {
      _initialized = true;
      _initializedCompleter.complete();
    } else if (message == Events.windowVisibilityChanged) {
      _visible = arguments as bool;
      visibilityChangedEvent.fire(_visible!);
      if (_visible! && _showCompleter != null) {
        _showCompleter!.complete();
        _showCompleter = null;
      }
    } else if (message == Events.windowClose) {
      WindowManager.instance.windowClosed(this);
      closeEvent.fire();
    } else if (message == Events.WindowStateFlagsChanged) {
      final stateFlags = WindowStateFlags.deserialize(arguments);
      windowStateFlagsEvent.fire(stateFlags);
    }
  }

  Future<dynamic> _invokeMethod(String method, [dynamic arguments]) {
    return WindowMethodDispatcher.instance.invokeMethod(
        channel: Channels.windowManager,
        method: method,
        arguments: arguments,
        targetWindowHandle: handle);
  }

  Future<void> waitUntilInitialized() async {
    if (_initialized) {
      return;
    } else {
      return _initializedCompleter.future;
    }
  }

  // Invoke custom method on target window. To handle the method see the
  // WindowMethodCallHandlerMixin class below.
  Future<dynamic> callMethod(String method, [dynamic argument]) {
    return _DefaultMethodCallHandler.instance.channel
        .invokeMethod(handle, method, argument);
  }

  final _initializedCompleter = Completer<void>();
  Completer<void>? _showCompleter;
  bool? _visible;
  bool _initialized = false;
}

// Window that belongs to current isolate
abstract class LocalWindow extends Window {
  LocalWindow(
    WindowHandle handle, {
    WindowHandle? parentWindow,
    this.initData,
  })  : _parentWindow = parentWindow,
        super(handle);

  @override
  void onMessage(String message, dynamic arguments) {
    if (message == Events.windowCloseRequest) {
      onCloseRequested();
    }
    super.onMessage(message, arguments);
  }

  @override
  Future<void> show() async {
    // Can't wait until readyToShow is called for window from current isolate;
    // that would cause deadlock
    return _invokeMethod(Methods.windowShow);
  }

  Future<void> readyToShow() async {
    await _invokeMethod(Methods.windowReadyToShow);
  }

  Future<void> onCloseRequested();

  Future<PopupMenuResponse> showPopupMenu(
    Menu menu,
    Offset globalPosition, {
    Rect? trackingRect,
    Rect? itemRect,
    bool preselectFirst = false,
  }) async {
    final handle = await menu.state.materialize();
    final res = await showPopupMenuWithHandle(handle, globalPosition,
        trackingRect: trackingRect,
        itemRect: itemRect,
        preselectFirst: preselectFirst);
    await menu.state.unmaterialize();
    return res;
  }

  Future<PopupMenuResponse> showPopupMenuWithHandle(
    MenuHandle handle,
    Offset globalPosition, {
    Rect? trackingRect,
    Rect? itemRect,
    bool preselectFirst = false,
  }) async {
    final value = await _invokeMethod(
        Methods.windowShowPopupMenu,
        PopupMenuRequest(
                handle: handle,
                position: globalPosition,
                trackingRect: trackingRect,
                itemRect: itemRect,
                preselectFirst: preselectFirst)
            .serialize());
    return PopupMenuResponse.deserialize(value);
  }

  Future<void> hidePopupMenu(MenuHandle handle) async {
    await _invokeMethod(Methods.windowHidePopupMenu,
        HidePopupMenuRequest(handle: handle).serialize());
  }

  Future<void> showSystemMenu() async {
    await _invokeMethod(Methods.windowShowSystemMenu);
  }

  Menu? _currentWindowMenu;

  Menu? get currentWindowMenu => _currentWindowMenu;

  Future<Menu?> setWindowMenu(Menu? menu) async {
    final res = _currentWindowMenu;
    if (_currentWindowMenu != null) {
      await _currentWindowMenu!.state.unmaterialize();
    }
    _currentWindowMenu = menu;
    final handle = await menu?.state.materialize();
    await _invokeMethod(Methods.windowSetWindowMenu, {
      'handle': handle?.value,
    });
    return res;
  }

  Future<void> performDrag() async {
    await _invokeMethod(Methods.windowPerformWindowDrag);
  }

  Future<void> closeWithResult(dynamic result) async {
    await _invokeMethod(Methods.windowCloseWithResult, result);
  }

  Window? get parentWindow =>
      WindowManager.instance.getWindow(_parentWindow ?? WindowHandle.invalid);

  final dynamic initData;
  final WindowHandle? _parentWindow;
}

//
// Handling custom methods on windows
//

typedef MethodCallHandler = FutureOr<dynamic> Function(dynamic argument);

mixin WindowMethodCallHandlerMixin<T extends StatefulWidget> on State<T>
    implements _WindowMethodCallHandler {
  @override
  MethodCallHandler? onMethodCall(String method);

  @override
  void initState() {
    super.initState();
    _DefaultMethodCallHandler.instance.handlers.add(this);
  }

  @override
  void dispose() {
    super.dispose();
    _DefaultMethodCallHandler.instance.handlers.remove(this);
  }
}

abstract class _WindowMethodCallHandler {
  MethodCallHandler? onMethodCall(String method);
}

//
//
//

class _DefaultMethodCallHandler {
  final channel = WindowMethodChannel('window.default-method-handler');
  static final instance = _DefaultMethodCallHandler();

  final handlers = <_WindowMethodCallHandler>[];

  _DefaultMethodCallHandler() {
    channel.setMethodCallHandler(onMethodCall);
  }

  Future<dynamic> onMethodCall(WindowMethodCall call) async {
    for (final h in handlers) {
      final handler = h.onMethodCall(call.method);
      if (handler != null) {
        return handler(call.arguments);
      }
    }
    throw PlatformException(
        code: 'no-handler',
        message: 'No handler was registered to handle "${call.method}"');
  }
}
