import 'dart:async';
import 'dart:math';

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

import 'api_model.dart';
import 'window.dart';
import 'window_manager.dart';

enum WindowSizingMode {
  // Window is always sized to match the content. This is useful for non-resizable
  // windows.
  //
  // You must need to make sure that the content can properly layout itself
  // with unbounded constraints. For example that means unconstrained Columns and Rows
  // need to have mainAxisSize set to MainAxisSize.min.
  sizeToContents,

  // Minimum window size will always match intrinsic content size. If window is
  // too small for intrinsic size, it will be resized.
  //
  // This mode requires content to be able to provide intrinsic content size.
  //
  // If you have widgets in your hierarchy that don't have intrinsic size you
  // can either wrap them in widgets that impose tight constraints on them, or
  // wrap them in IntrinsicSizedBox.
  atLeastIntrinsicSize,

  // No automatic sizing is done. You may need to override initializeWindow to
  // resize window to initial content size.
  manual,
}

// Class responsible for creating window contents and managing window properties.
abstract class WindowState {
  // Build the contents within the window
  Widget build(BuildContext context);

  // Specify the sizing mode for the window. See WindowSizing mode values.
  WindowSizingMode get windowSizingMode;

  // Returns the window associated with current hierarchy. You can also use
  // 'Window.of(context)' instead.
  LocalWindow get window => WindowManager.instance.currentWindow;

  // Called after window creation. By default resizes window to content size
  // (if known) and shows the window.
  // You can override this to change window title, configure frame,
  // buttons, or if you want the window to be initially hidden.
  Future<void> initializeWindow(Size contentSize) async {
    await window.setGeometry(Geometry(
      contentSize: contentSize,
    ));
    // Disable user resizing for auto-sized windows
    if (windowSizingMode == WindowSizingMode.sizeToContents) {
      await window.setStyle(WindowStyle(
        canResize: false,
        canFullScreen: false,
      ));
    }
    await window.show();
  }

  // Called when user requests closing the window. Default implementation calls
  // window.close() to close the window. Omitting the window.close() call will
  // prevent user from closing the window.
  Future<void> windowCloseRequested() async {
    await window.close();
  }

  // Updates window constraints; Called for windows sized with
  // WindowSizingMode.atLeastIntrinsicSize when intrinsic content size changes.
  Future<void> updateWindowConstraints(Size intrinsicContentSize) async {
    await window.setGeometry(Geometry(
      minContentSize: intrinsicContentSize,
    ));
  }

  // Called to update window size to new dimensions
  Future<void> updateWindowSize(Size contentSize) async {
    await window.setGeometry(Geometry(contentSize: contentSize));
  }

  // Convenience function to calculate initial geometry for centered windows
  Future<Geometry> centerInParent(Size contentSize) async {
    final parent = window.parentWindow;
    if (parent != null) {
      final parentGeometry = await parent.getGeometry();
      final parentOrigin =
          parentGeometry.contentOrigin ?? parentGeometry.frameOrigin;
      final parentSize =
          parentGeometry.contentSize ?? parentGeometry.contentSize;
      if (parentOrigin != null && parentSize != null) {
        final origin = Offset(
            parentOrigin.dx + parentSize.width / 2 - contentSize.width / 2,
            parentOrigin.dy + parentSize.height / 2 - contentSize.height / 2);
        return Geometry(
            contentOrigin: origin,
            // in case backend doesn't support contentOrigin, frameOrigin will be used
            frameOrigin: origin,
            contentSize: contentSize);
      }
    }

    return Geometry(contentSize: contentSize);
  }

  // Returns the WindowState of given type in the hierarchy. If not preset will fail with
  // assertion (or exception in release build.)
  static T of<T extends WindowState>(BuildContext context) {
    final res = context
        .dependOnInheritedWidgetOfExactType<_WindowStateWidget>()
        ?.windowState;
    assert(res is T, 'Window context of requested type not found in hierarchy');
    return res as T;
  }

  // Returns the WindowState of given type in the hierarchy or null if not present.
  static T? maybeOf<T extends WindowState>(BuildContext context) {
    final res = context
        .dependOnInheritedWidgetOfExactType<_WindowStateWidget>()
        ?.windowState;
    return res is T ? res : null;
  }

  void registerTapCallback(ValueChanged<PointerDownEvent> cb) {
    _tapCallbacks.add(cb);
  }

  void unregisterTapCallback(ValueChanged<PointerDownEvent> cb) {
    _tapCallbacks.remove(cb);
  }

  final _tapCallbacks = <ValueChanged<PointerDownEvent>>[];
}

typedef WindowStateFactory = WindowState Function(dynamic initData);

// Every window must have WindowWidget in hierarchy. WindowWidget is responsible
// for creating the WindowState from initData and internally for handling window
// contents layout and size (i.e. sizing window to match content size).
class WindowWidget extends StatefulWidget {
  WindowWidget({
    required this.onCreateState,
    Key? key,
  }) : super(key: key);

  // Factory responsible for creating state
  final WindowStateFactory onCreateState;

  @override
  State<StatefulWidget> createState() {
    return _WindowWidgetState();
  }
}

//
// WindowWidget internals
//

enum _Status { notInitialized, initializing, initialized }

bool _haveWindowLayoutProbe = false;

class _WindowWidgetState extends State<WindowWidget> {
  WindowState? _windowState;

  @override
  Widget build(BuildContext context) {
    _maybeInitialize();
    if (status == _Status.initialized) {
      final window = WindowManager.instance.currentWindow;
      final emptyBefore = _windowState == null;
      _windowState ??= widget.onCreateState(window.initData);
      if (emptyBefore) {
        WindowManager.instance.haveWindowState(_windowState!);
      }
      if (_windowState!.windowSizingMode == WindowSizingMode.manual) {
        // ignore: unnecessary_non_null_assertion
        WidgetsBinding.instance!.addPostFrameCallback((timeStamp) {
          _prepareAndShow(_windowState!, () => Size(0, 0));
        });
      } else {
        if (!_haveWindowLayoutProbe) {
          _checkWindowLayoutProbe();
        }
      }

      return Listener(
        onPointerDown: _onWindowTap,
        child: Container(
          color: Color(0x00000000),
          child: _WindowStateWidget(
            windowState: _windowState!,
            child: Builder(
              builder: (context) {
                return _windowState!.build(context);
              },
            ),
          ),
        ),
      );
    } else {
      return Container();
    }
  }

  void _maybeInitialize() async {
    if (status == _Status.notInitialized) {
      status = _Status.initializing;
      await WindowManager.initialize();
      status = _Status.initialized;
      setState(() {});
    }
  }

  Future<void> _checkWindowLayoutProbe() async {
    await Future.delayed(Duration(seconds: 2));
    assert(
        _haveWindowLayoutProbe,
        '\n*******************************************************\n\n'
        'BREAKING CHANGE:\n'
        'To use WindowSizingMode.sizeToContents or '
        'WindowSizingMode.atLeastIntrinsicSize you need to put '
        'the WindowLayoutProbe widget somewhere in widget hierarchy.\n'
        'It must be below WindowWidget, but higher than any '
        'widget that affects layout (i.e. Padding).\n'
        'For example:\n'
        '| WindowWidget\n'
        '|   MaterialApp\n'
        '|      WindowLayoutProbe\n'
        '|        <Actual Content>\n\n');
  }

  _Status status = _Status.notInitialized;
  dynamic initData;

  void _onWindowTap(PointerDownEvent e) {
    for (final cb in List<ValueChanged<PointerDownEvent>>.from(
        _windowState!._tapCallbacks)) {
      if (_windowState!._tapCallbacks.contains(cb)) {
        cb(e);
      }
    }
  }
}

class _WindowLayoutChecker extends InheritedWidget {
  _WindowLayoutChecker({Key? key, required Widget child})
      : super(key: key, child: child);

  @override
  bool updateShouldNotify(covariant InheritedWidget oldWidget) {
    return false;
  }
}

//
// This widget is responsible for sizing window to match content size.
// It must be placed in widget hierarchy in a way where its children can
// layout unconstrained (in case of WindowSizingMode.sizeToContents), or
// where the children can provide intrinsic content size
// (WindowSizingMode.atLeastIntrinsicSize). This can be for example home
// widget of MaterialApp.
//
class WindowLayoutProbe extends StatelessWidget {
  const WindowLayoutProbe({Key? key, required this.child}) : super(key: key);

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final state = WindowState.maybeOf(context);
    assert(
        state != null, 'WindowLayoutProbe must be placed below WindowWidget');
    final prev =
        context.dependOnInheritedWidgetOfExactType<_WindowLayoutChecker>();
    assert(
        prev == null,
        'Multiple WindowLayoutProbe widgets found in hierarchy. '
        'Please make sure there is only one WindowLayoutProbe widget present.');
    _haveWindowLayoutProbe = true;

    return _WindowLayoutChecker(
        child: _WindowLayout(
            windowState: state!,
            child: _WindowLayoutInner(windowState: state, child: child)));
  }
}

// Used by Window.of(context)
class _WindowStateWidget extends InheritedWidget {
  final WindowState windowState;

  _WindowStateWidget({
    required Widget child,
    required this.windowState,
  }) : super(child: child);

  @override
  bool updateShouldNotify(covariant InheritedWidget oldWidget) {
    return false;
  }
}

class _WindowLayoutInner extends SingleChildRenderObjectWidget {
  final WindowState windowState;

  const _WindowLayoutInner({required Widget child, required this.windowState})
      : super(child: child);

  @override
  RenderObject createRenderObject(BuildContext context) {
    return _RenderWindowLayoutInner(windowState);
  }

  @override
  void updateRenderObject(
      BuildContext context, covariant _RenderWindowLayoutInner renderObject) {
    renderObject.windowState = windowState;
  }
}

class _RenderWindowLayoutInner extends RenderProxyBox {
  _RenderWindowLayoutInner(this.windowState);

  WindowState windowState;

  @override
  void performLayout() {
    if (windowState.windowSizingMode != WindowSizingMode.sizeToContents) {
      super.performLayout();
    } else {
      final constraints = this.constraints.loosen();
      child!.layout(constraints, parentUsesSize: true);
      assert(
          child!.size.width != constraints.maxWidth &&
              child!.size.height != constraints.maxHeight,
          "Child failed to constraint itself! If you're using Row or Column, "
          "don't forget to set mainAxisSize to MainAxisSize.min");
      size = _sanitizeAndSnapToPixelBoundary(child!.size);
      if (size != child!.size) {
        // This can happen for fractional scaling when child didn't land exactly
        // on physical fixel boundaries. Hopefully in future Flutter will do better
        // job with fractional scaling. For now force child to fill available space.
        child!.layout(
            BoxConstraints(minWidth: size.width, minHeight: size.height),
            parentUsesSize: true);
      }
      _updateGeometry();
    }
  }

  bool _geometryPending = false;
  bool _geometryInProgress = false;

  void _updateGeometry() async {
    if (_geometryInProgress) {
      _geometryPending = true;
    } else {
      _geometryInProgress = true;
      await windowState.updateWindowSize(_sanitizeAndSnapToPixelBoundary(size));
      _geometryInProgress = false;
      if (_geometryPending) {
        _geometryPending = false;
        _updateGeometry();
      }
    }
  }
}

class _WindowLayout extends SingleChildRenderObjectWidget {
  final WindowState windowState;

  const _WindowLayout({
    Key? key,
    required Widget child,
    required this.windowState,
  }) : super(key: key, child: child);

  @override
  RenderObject createRenderObject(BuildContext context) {
    return _RenderWindowLayout(
      windowState,
    );
  }

  @override
  void updateRenderObject(
      BuildContext context, covariant _RenderWindowLayout renderObject) {
    renderObject.windowState = windowState;
  }
}

class _RenderWindowLayout extends RenderProxyBox {
  _RenderWindowLayout(this.windowState);

  WindowState windowState;

  Size? _lastConstraints;

  @override
  void performLayout() {
    if (windowState.windowSizingMode == WindowSizingMode.sizeToContents) {
      final constraints =
          BoxConstraints.loose(Size(double.infinity, double.infinity));
      child!.layout(constraints, parentUsesSize: true);
      size = Size(this.constraints.maxWidth, this.constraints.maxHeight);
    } else if (windowState.windowSizingMode ==
        WindowSizingMode.atLeastIntrinsicSize) {
      var w = child!.getMaxIntrinsicWidth(double.infinity);
      var h = child!.getMinIntrinsicHeight(w);

      final intrinsicSize = _sanitizeAndSnapToPixelBoundary(Size(w, h));

      if (_lastConstraints != intrinsicSize) {
        windowState.updateWindowConstraints(intrinsicSize);
        _lastConstraints = intrinsicSize;
      }

      final size = this.constraints.biggest;

      final maxSize = Size(max(intrinsicSize.width, size.width),
          max(intrinsicSize.height, size.height));

      if (maxSize.width > size.width || maxSize.height > size.height) {
        windowState.updateWindowSize(_sanitizeAndSnapToPixelBoundary(maxSize));
      }
      final constraints = BoxConstraints.tight(maxSize);
      child!.layout(constraints, parentUsesSize: true);
      this.size = size;
    } else {
      super.performLayout();
    }

    _prepareAndShow(windowState, () {
      final Size size;
      if (windowState.windowSizingMode ==
          WindowSizingMode.atLeastIntrinsicSize) {
        var w = child!.getMaxIntrinsicWidth(double.infinity);
        var h = child!.getMinIntrinsicHeight(w);
        size = _sanitizeAndSnapToPixelBoundary(Size(w, h));
      } else if (windowState.windowSizingMode == WindowSizingMode.manual) {
        size = Size(0, 0);
      } else {
        size = _sanitizeAndSnapToPixelBoundary(child!.size);
      }
      return size;
    });
  }
}

void _prepareAndShow(
    WindowState windowState, Size Function() getInitialSize) async {
  if (_windowShown) {
    return;
  }
  _windowShown = true;
  final win = WindowManager.instance.currentWindow;
  final size = getInitialSize();
  await windowState.initializeWindow(size);
  if (windowState.windowSizingMode == WindowSizingMode.atLeastIntrinsicSize) {
    await windowState.updateWindowConstraints(size);
  }
  await win.readyToShow();
}

bool _windowShown = false;

Size _sanitizeAndSnapToPixelBoundary(Size size) {
  var w = size.width;
  var h = size.height;
  // sane default in case intrinsic size can't be determined
  if (w == 0) {
    w = 100;
  }
  if (h == 0) {
    h = 100;
  }

  // Error messages can get huge
  if (w > 10000) {
    w = 800;
    h = 400;
  }
  size = Size(w, h);

  // ignore: unnecessary_non_null_assertion
  final ratio = WidgetsBinding.instance!.window.devicePixelRatio;
  size = size * ratio;
  size = Size(size.width.ceilToDouble(), size.height.ceilToDouble());
  size /= ratio;
  return size;
}
