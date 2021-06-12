import 'dart:async';
import 'dart:math';

import 'package:flutter/rendering.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';

import 'api_model.dart';
import 'window.dart';
import 'window_manager.dart';

enum WindowSizingMode {
  // Windows is always sized to match the content. This is useful for non-resizable
  // windows. You must need to make sure that the content can properly layout itself
  // with unbounded constraints. For example that means unconstrained Columns and Rows
  // need to have mainAxisSize set to MainAxisSize.min.
  sizeToContents,

  // Minimum window size will always match intrinsic content size. If window is
  // too small for intrinsic size, it will be resized.
  //
  // This mode requires content to be able to provide intrinsic content size.
  atLeastIntrinsicSize,

  // No automatic sizing is done. You may need to override initializeWindow to
  // provide initial content size.
  manual,
}

// Class responsible for creating window contents and managing window properties.
abstract class WindowState {
  // Build the contents within the window
  Widget build(BuildContext context);

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

  // Updates window constraints; Called for manually sized windows when intrinsic
  // content size changes.
  Future<void> updateWindowConstraints(Size intrinsicContentSize) async {
    await window.setGeometry(Geometry(
      minContentSize: intrinsicContentSize,
    ));
  }

  WindowSizingMode get windowSizingMode =>
      WindowSizingMode.atLeastIntrinsicSize;

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
        ?.context;
    assert(res is T, 'Window context of requested type not found in hierarchy');
    return res as T;
  }

  // Returns the WindowState of given type in the hierarchy or null if not present.
  static T? maybeOf<T extends WindowState>(BuildContext context) {
    final res = context
        .dependOnInheritedWidgetOfExactType<_WindowStateWidget>()
        ?.context;
    res is T ? res : null;
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

class _WindowWidgetState extends State<WindowWidget> {
  WindowState? _windowContext;

  @override
  Widget build(BuildContext context) {
    _maybeInitialize();
    if (status == _Status.initialized) {
      final window = WindowManager.instance.currentWindow;
      _windowContext ??= widget.onCreateState(window.initData);

      return Listener(
        onPointerDown: _onWindowTap,
        child: Container(
          color: Color(0x00000000),
          child: _WindowStateWidget(
            context: _windowContext!,
            child: _WindowLayout(
              builtWindow: _windowContext!,
              child: _WindowLayoutInner(
                builtWindow: _windowContext!,
                child: Builder(
                  builder: (context) {
                    return _windowContext!.build(context);
                  },
                ),
              ),
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

  _Status status = _Status.notInitialized;
  dynamic initData;

  void _onWindowTap(PointerDownEvent e) {
    for (final cb in List<ValueChanged<PointerDownEvent>>.from(
        _windowContext!._tapCallbacks)) {
      if (_windowContext!._tapCallbacks.contains(cb)) {
        cb(e);
      }
    }
  }
}

// Used by Window.of(context)
class _WindowStateWidget extends InheritedWidget {
  final WindowState context;

  _WindowStateWidget({
    required Widget child,
    required this.context,
  }) : super(child: child);

  @override
  bool updateShouldNotify(covariant InheritedWidget oldWidget) {
    return false;
  }
}

class _WindowLayoutInner extends SingleChildRenderObjectWidget {
  final WindowState builtWindow;

  const _WindowLayoutInner({required Widget child, required this.builtWindow})
      : super(child: child);

  @override
  RenderObject createRenderObject(BuildContext context) {
    return _RenderWindowLayoutInner(builtWindow);
  }

  @override
  void updateRenderObject(
      BuildContext context, covariant _RenderWindowLayoutInner renderObject) {
    renderObject.builtWindow = builtWindow;
  }
}

class _RenderWindowLayoutInner extends RenderProxyBox {
  _RenderWindowLayoutInner(this.builtWindow);

  WindowState builtWindow;

  @override
  void performLayout() {
    if (builtWindow.windowSizingMode != WindowSizingMode.sizeToContents) {
      super.performLayout();
    } else {
      final constraints = this.constraints.loosen();
      child!.layout(constraints, parentUsesSize: true);
      assert(
          child!.size.width != constraints.maxWidth &&
              child!.size.height != constraints.maxHeight,
          "Child failed to constraint itself! If you're using Row or Column, "
          "don't forget to set mainAxisSize to MainAxisSize.min");
      size = child!.size;
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
      await builtWindow.updateWindowSize(_sanitizeAndSnapToPixelBoundary(size));
      _geometryInProgress = false;
      if (_geometryPending) {
        _geometryPending = false;
        _updateGeometry();
      }
    }
  }
}

class _WindowLayout extends SingleChildRenderObjectWidget {
  final WindowState builtWindow;

  const _WindowLayout({
    Key? key,
    required Widget child,
    required this.builtWindow,
  }) : super(key: key, child: child);

  @override
  RenderObject createRenderObject(BuildContext context) {
    return _RenderWindowLayout(
      builtWindow,
    );
  }

  @override
  void updateRenderObject(
      BuildContext context, covariant _RenderWindowLayout renderObject) {
    renderObject.builtWindow = builtWindow;
  }
}

class _RenderWindowLayout extends RenderProxyBox {
  _RenderWindowLayout(this.builtWindow);

  WindowState builtWindow;

  Size? _lastConstraints;

  @override
  void performLayout() {
    if (builtWindow.windowSizingMode == WindowSizingMode.sizeToContents) {
      final constraints =
          BoxConstraints.loose(Size(double.infinity, double.infinity));
      child!.layout(constraints, parentUsesSize: true);
      size = Size(this.constraints.maxWidth, this.constraints.maxHeight);
    } else if (builtWindow.windowSizingMode ==
        WindowSizingMode.atLeastIntrinsicSize) {
      var w = child!.getMaxIntrinsicWidth(double.infinity);
      var h = child!.getMinIntrinsicHeight(w);

      final intrinsicSize = _sanitizeAndSnapToPixelBoundary(Size(w, h));

      if (_lastConstraints != intrinsicSize) {
        builtWindow.updateWindowConstraints(intrinsicSize);
        _lastConstraints = intrinsicSize;
      }

      final size = this.constraints.biggest;

      final maxSize = Size(max(intrinsicSize.width, size.width),
          max(intrinsicSize.height, size.height));

      if (maxSize.width > size.width || maxSize.height > size.height) {
        builtWindow.updateWindowSize(maxSize);
      }
      final constraints = BoxConstraints.tight(maxSize);
      child!.layout(constraints, parentUsesSize: true);
      this.size = size;
    } else {
      super.performLayout();
    }

    if (!hasLayout) {
      hasLayout = true;
      // Can't really use WidgetsBinding.waitUntilFirstFrameRasterized here
      // since that seem to be fired before the layer tree is even sent to
      // rasterizer, which is way too early
      final win = WindowManager.instance.currentWindow;
      SchedulerBinding.instance!.scheduleFrameCallback((timeStamp) {
        SchedulerBinding.instance!.addPostFrameCallback((timeStamp) async {
          final size = _sanitizeAndSnapToPixelBoundary(this.size);
          await builtWindow.initializeWindow(size);
          if (builtWindow.windowSizingMode != WindowSizingMode.sizeToContents) {
            await builtWindow.updateWindowConstraints(size);
          }
          await win.readyToShow();
        });
      });
    }
  }

  bool hasLayout = false;
}

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

  final ratio = WidgetsBinding.instance!.window.devicePixelRatio;
  size = size * ratio;
  size = Size(size.width.ceilToDouble(), size.height.ceilToDouble());
  size /= ratio;
  return size;
}
