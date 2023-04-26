import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';
import 'dart:io';

import 'api_constants.dart';
import 'drag_drop.dart';
import 'event.dart';
import 'key_interceptor.dart';
import 'keyboard_map_internal.dart';
import 'screen_internal.dart';
import 'status_item_internal.dart';
import 'util.dart';
import 'window_method_channel.dart';
import 'window_widget.dart';
import 'window.dart';

// Do not use directly. Access windows through Window.of(context) or through
// WindowState.window.
class WindowManager {
  WindowManager._();

  static final instance = WindowManager._();

  static Future<void> initialize() {
    return instance._init();
  }

  Future<void> _checkApiVersion(WindowMethodDispatcher dispatcher) async {
    final version = await dispatcher.invokeMethod(
        channel: Channels.windowManager,
        method: Methods.windowManagerGetApiVersion,
        targetWindowHandle: WindowHandle.invalid);
    if (version != currentApiVersion) {
      print('Warning: Mismatched API version!');
      print('  NativeShell Rust crate API version: $version');
      print('  NativeShell Dart package API version: $currentApiVersion.');
      print(
          '  Please update the ${version > currentApiVersion ? 'Dart package' : 'Rust crate'}.');
    }
  }

  Future<void> _init() async {
    WidgetsFlutterBinding.ensureInitialized();
    KeyInterceptor.instance;
    final dispatcher = WindowMethodDispatcher.instance;

    await _checkApiVersion(dispatcher);

    await KeyboardMapManager.instance.init();
    await ScreenManager.instance.init();
    await StatusItemManager.instance.init();

    final result = await dispatcher.invokeMethod(
        channel: Channels.windowManager,
        method: Methods.windowManagerInitWindow,
        targetWindowHandle: WindowHandle.invalid);

    _currentWindow = WindowHandle(result['currentWindow'] as int);
    final allWindows = result['allWindows'] as List;
    final initData = result['initData'];
    final parentWindow = result['parentWindow'] as int?;

    for (final win in allWindows) {
      final handle = WindowHandle(win);
      _windows[handle] = handle == _currentWindow
          ? _LocalWindow(handle,
              initData: initData,
              parentWindow:
                  parentWindow != null ? WindowHandle(parentWindow) : null)
          : Window(handle);
    }

    dispatcher.registerMessageHandler(Channels.windowManager, _onMessage);
    dispatcher.registerMethodHandler(Channels.dropTarget, _onDropTargetCall);
  }

  Future<Window> createWindow(
    dynamic initData, {
    required bool invisibleWindowHint,
  }) async {
    if (!invisibleWindowHint) {
      _maybePause();
    }
    final dispatcher = WindowMethodDispatcher.instance;
    final result = await dispatcher.invokeMethod(
        channel: Channels.windowManager,
        method: Methods.windowManagerCreateWindow,
        targetWindowHandle: WindowHandle.invalid,
        arguments: {
          'parent': currentWindow.handle.value,
          'initData': initData,
        });
    final handle = WindowHandle(result['windowHandle'] as int);
    final res = _windows.putIfAbsent(handle, () => Window(handle));
    await res.waitUntilInitialized();
    if (!invisibleWindowHint) {
      _maybeResume();
    }
    return res;
  }

  void windowClosed(Window window) {
    _windows.remove(window.handle)?.dispose();
  }

  final _windows = <WindowHandle, Window>{};
  WindowHandle? _currentWindow;

  LocalWindow get currentWindow => _windows[_currentWindow]! as LocalWindow;

  Window? getWindow(WindowHandle handle) => _windows[handle];

  void haveWindowState(WindowState state) {
    (currentWindow as _LocalWindow)._currentState = state;
  }

  void _onMessage(WindowMessage message) {
    var window = _windows[message.sourceWindowHandle];
    if (window == null) {
      window = Window(message.sourceWindowHandle);
      _windows[window.handle] = window;
      windowAddedEvent.fire(window);
    }
    window.onMessage(message.message, message.arguments);
  }

  Future<dynamic> _onDropTargetCall(WindowMethodCall call) async {
    final window = _windows[call.targetWindowHandle];
    if (window is _LocalWindow) {
      return window._dragDriver.onMethodCall(call);
    } else {
      return null;
    }
  }

  final windowAddedEvent = Event<Window>();
}

class _WindowDragDriver extends DragDriver {
  Future<dynamic> onMethodCall(WindowMethodCall call) async {
    if (call.method == Methods.dragDriverDraggingUpdated) {
      final info = DragInfo.deserialize(call.arguments);
      final res = await draggingUpdated(info);
      return {
        'effect': enumToString(res),
      };
    } else if (call.method == Methods.dragDriverDraggingExited) {
      return draggingExited();
    } else if (call.method == Methods.dragDriverPerformDrop) {
      final info = DragInfo.deserialize(call.arguments);
      return performDrop(info);
    }
  }
}

class _LocalWindow extends LocalWindow {
  _LocalWindow(
    WindowHandle handle, {
    WindowHandle? parentWindow,
    dynamic initData,
  }) : super(handle, parentWindow: parentWindow, initData: initData) {
    visibilityChangedEvent.addListener(_visibilityChanged);
  }

  WindowState? _currentState;

  bool _visible = false;

  void _updatePause() {
    if (!_visible && !_paused) {
      _pushPause();
      _paused = true;
    } else if (_visible && _paused) {
      _popPause();
      _paused = false;
    }
  }

  void _visibilityChanged(bool visible) {
    _visible = visible;
    _updatePause();
  }

  bool _paused = false;

  final _dragDriver = _WindowDragDriver();

  @override
  Future<void> onCloseRequested() async {
    if (_currentState != null) {
      await _currentState!.windowCloseRequested();
    } else {
      await close();
    }
  }

  @override
  Future<void> readyToShow() {
    SchedulerBinding.instance.addPostFrameCallback((_) {
      _updatePause();
    });

    return super.readyToShow();
  }

  @override
  void onMessage(String message, dynamic arguments) {
    super.onMessage(message, arguments);
  }
}

int _pauseCount = 0;

void _pushPause() {
  ++_pauseCount;
  if (_pauseCount > 0) {
    WidgetsBinding.instance
        .handleAppLifecycleStateChanged(AppLifecycleState.paused);
  }
}

void _popPause() {
  assert(_pauseCount > 0);
  --_pauseCount;
  if (_pauseCount == 0) {
    WidgetsBinding.instance
        .handleAppLifecycleStateChanged(AppLifecycleState.resumed);
  }
}

// On Windows Angle has a big global lock which can get congested and causes
// large delays when creating new window. As a workaround, we briefly pause
// current isolate rasterization when creating window.
bool _needPauseWhenCreatingWindow() {
  return Platform.isWindows;
}

void _maybePause() {
  if (_needPauseWhenCreatingWindow()) {
    _pushPause();
  }
}

void _maybeResume() async {
  if (_needPauseWhenCreatingWindow()) {
    await Future.delayed(Duration(milliseconds: 100));
    _popPause();
  }
}
