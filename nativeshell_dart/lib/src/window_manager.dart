import 'package:flutter/widgets.dart';

import 'key_interceptor.dart';
import 'api_constants.dart';
import 'drag_drop.dart';
import 'event.dart';
import 'window.dart';
import 'window_method_channel.dart';

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

  Future<Window> createWindow(dynamic initData) async {
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
    return res;
  }

  void windowClosed(Window window) {
    _windows.remove(window.handle);
  }

  final _windows = <WindowHandle, Window>{};
  WindowHandle? _currentWindow;

  LocalWindow get currentWindow => _windows[_currentWindow]! as LocalWindow;

  Window? getWindow(WindowHandle handle) => _windows[handle];

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
      return window._dropTarget.onMethodCall(call);
    } else {
      return null;
    }
  }

  final windowAddedEvent = Event<Window>();
}

class _LocalWindow extends LocalWindow {
  _LocalWindow(WindowHandle handle,
      {WindowHandle? parentWindow, dynamic initData})
      : super(handle, parentWindow: parentWindow, initData: initData);

  final _dropTarget = DropTarget();
}
