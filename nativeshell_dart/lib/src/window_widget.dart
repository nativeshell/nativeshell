import 'dart:async';
import 'dart:math';

import 'package:flutter/rendering.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';

import 'api_model.dart';
import 'window.dart';
import 'window_manager.dart';

// Class responsible for creating window contents and managing window properties.
abstract class WindowState {
  // Build the contents within the window
  Widget build(BuildContext context);

  // Returns the window associated with current hierarchy. You can also use
  // 'Window.of(context)' instead.
  LocalWindow get window => WindowManager.instance.currentWindow;

  // Called after window creation. By default resizes window to intrinsic
  // content size and shows the window.
  // You can override this to change window title, configure frame,
  // buttons, or if you want the window to be initially hidden.
  Future<void> initializeWindow(Size intrinsicContentSize) async {
    await window.setGeometry(Geometry(
      contentSize: intrinsicContentSize,
    ));
    // Disable user resizing for auto-sized windows
    if (autoSizeWindow) {
      await window.setStyle(WindowStyle(
        canResize: false,
        canFullScreen: false,
      ));
    }
    await window.show();
  }

  // Updates window constraints; Called for manually sized windows after creation
  // or after requestUpdateConstraints() was called
  Future<void> updateWindowConstraints(Size intrinsicContentSize) async {
    await window.setGeometry(Geometry(
      minContentSize: intrinsicContentSize,
    ));
  }

  // Windows is always sized to fit the content. This is useful for non-resizable
  // windows. You must need to make sure that the content can properly layout itself
  // with unbounded constraints. For example that means unconstrained Columns and Rows
  // need to have mainAxisSize set to MainAxisSize.min.
  bool get autoSizeWindow => false;

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

  // Useful for resizable window. This will schedule a special layout pass that will let
  // the content overflow windows dimensions. If it does overflow, window will be resized
  // to new dimensions and minimum content size will be udpated accordingly.
  // You can call this in setState() if you know that content size will change after
  // rebuild.
  void requestUpdateConstraints() {
    assert(_requestUpdateConstraints != null,
        'requestUpdateConstraints() may not be called in WindowState constructor!');
    _requestUpdateConstraints!();
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
  VoidCallback? _requestUpdateConstraints;
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
      _windowContext!._requestUpdateConstraints = requestUpdateConstraints;

      return Listener(
        onPointerDown: _onWindowTap,
        child: Container(
          color: Color(0x00000000),
          child: _WindowStateWidget(
            context: _windowContext!,
            child: _WindowLayout(
              builtWindow: _windowContext!,
              updatingConstraints: updatingConstraints,
              updatingConstraintsDone: updatingConstraintsDone,
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
  bool updatingConstraints = false;
  int constraintsUpdateCookie = 0;

  void requestUpdateConstraints() {
    ++constraintsUpdateCookie;
    setState(() {
      updatingConstraints = true;
    });
  }

  void updatingConstraintsDone() {
    final cookie = constraintsUpdateCookie;
    SchedulerBinding.instance!.addPostFrameCallback((Duration _) {
      if (constraintsUpdateCookie == cookie) {
        // not changed in the meanwhile
        setState(() {
          updatingConstraints = false;
        });
      }
    });
  }

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
    if (!builtWindow.autoSizeWindow) {
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
  final bool updatingConstraints;
  final VoidCallback updatingConstraintsDone;

  const _WindowLayout({
    Key? key,
    required Widget child,
    required this.builtWindow,
    required this.updatingConstraints,
    required this.updatingConstraintsDone,
  }) : super(key: key, child: child);

  @override
  RenderObject createRenderObject(BuildContext context) {
    return _RenderWindowLayout(
        builtWindow, updatingConstraints, updatingConstraintsDone);
  }

  @override
  void updateRenderObject(
      BuildContext context, covariant _RenderWindowLayout renderObject) {
    renderObject.builtWindow = builtWindow;
    renderObject.updatingConstraints = updatingConstraints;
    renderObject.updatingConstraintsDone = updatingConstraintsDone;
    if (updatingConstraints) {
      renderObject.markNeedsLayout();
    }
  }
}

class _RenderWindowLayout extends RenderProxyBox {
  _RenderWindowLayout(
      this.builtWindow, this.updatingConstraints, this.updatingConstraintsDone);

  WindowState builtWindow;
  bool updatingConstraints;
  VoidCallback updatingConstraintsDone;

  @override
  void performLayout() {
    if (builtWindow.autoSizeWindow) {
      final constraints =
          BoxConstraints.loose(Size(double.infinity, double.infinity));
      child!.layout(constraints, parentUsesSize: true);
      size = Size(this.constraints.maxWidth, this.constraints.maxHeight);
    } else if (updatingConstraints) {
      var w = child!.getMaxIntrinsicWidth(double.infinity);
      var h = child!.getMinIntrinsicHeight(w);
      final intrinsicSize = _sanitizeAndSnapToPixelBoundary(Size(w, h));
      builtWindow.updateWindowConstraints(intrinsicSize);

      final maxSize = Size(max(intrinsicSize.width, size.width),
          max(intrinsicSize.height, size.height));

      if (maxSize.width > size.width || maxSize.height > size.height) {
        builtWindow.updateWindowSize(maxSize);
      } else {
        updatingConstraintsDone();
      }

      final constraints = BoxConstraints.loose(maxSize);
      child!.layout(constraints, parentUsesSize: true);
      size = Size(this.constraints.maxWidth, this.constraints.maxHeight);
    } else {
      super.performLayout();
    }

    if (!hasLayout) {
      hasLayout = true;

      final win = WindowManager.instance.currentWindow;
      SchedulerBinding.instance!.scheduleFrameCallback((timeStamp) {
        SchedulerBinding.instance!.addPostFrameCallback((timeStamp) async {
          var w = child!.getMaxIntrinsicWidth(double.infinity);
          var h = child!.getMinIntrinsicHeight(w);

          final size = _sanitizeAndSnapToPixelBoundary(Size(w, h));
          await builtWindow.initializeWindow(size);
          if (!builtWindow.autoSizeWindow) {
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
