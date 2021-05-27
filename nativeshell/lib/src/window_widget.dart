import 'dart:async';
import 'dart:math';

import 'package:flutter/rendering.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';

import 'api_model.dart';
import 'window.dart';
import 'window_manager.dart';

abstract class WindowBuilder {
  Widget build(BuildContext context);

  Future<void> initializeWindow(
      LocalWindow window, Size intrinsicContentSize) async {
    await window.setGeometry(Geometry(
      contentSize: intrinsicContentSize,
    ));
    await window.show();
  }

  Future<void> updateWindowConstraints(
      LocalWindow window, Size intrinsicContentSize) async {}

  bool get autoSizeWindow => false;

  Future<void> updateWindowSize(LocalWindow window, Size contentSize) async {
    await window.setGeometry(Geometry(contentSize: contentSize));
  }
}

typedef WindowBuilderProvider = WindowBuilder Function(dynamic initData);

class WindowWidget extends StatefulWidget {
  WindowWidget({
    required this.builder,
    Key? key,
  }) : super(key: key);

  final WindowBuilderProvider builder;

  @override
  State<StatefulWidget> createState() {
    return _WindowWidgetState();
  }
}

//
//
//

enum _Status { notInitialized, initializing, initialized }

class _WindowWidgetState extends State<WindowWidget> implements WindowContext {
  WindowBuilder? _builder;

  @override
  Widget build(BuildContext context) {
    _maybeInitialize();
    if (status == _Status.initialized) {
      final window = WindowManager.instance.currentWindow;
      _builder ??= widget.builder(window.initData);

      return Listener(
        onPointerDown: _onWindowTap,
        child: Container(
          color: Color(0x00000000),
          child: _WindowContextWidget(
            context: this,
            child: _WindowLayout(
              builtWindow: _builder!,
              updatingConstraints: updatingConstraints,
              updatingConstraintsDone: updatingConstraintsDone,
              child: _WindowLayoutInner(
                builtWindow: _builder!,
                child: Builder(
                  builder: (context) {
                    return _builder!.build(context);
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

  @override
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

  @override
  void registerTapCallback(ValueChanged<PointerDownEvent> cb) {
    _tapCallbacks.add(cb);
  }

  @override
  void unregisterTapCallback(ValueChanged<PointerDownEvent> cb) {
    _tapCallbacks.remove(cb);
  }

  void _onWindowTap(PointerDownEvent e) {
    for (final cb in List<ValueChanged<PointerDownEvent>>.from(_tapCallbacks)) {
      if (_tapCallbacks.contains(cb)) {
        cb(e);
      }
    }
  }

  @override
  LocalWindow get window => WindowManager.instance.currentWindow;
  final _tapCallbacks = <ValueChanged<PointerDownEvent>>[];
}

abstract class WindowContext {
  LocalWindow get window;

  void registerTapCallback(ValueChanged<PointerDownEvent> e);
  void unregisterTapCallback(ValueChanged<PointerDownEvent> e);
  void requestUpdateConstraints();

  static WindowContext of(BuildContext context) {
    final res = context
        .dependOnInheritedWidgetOfExactType<_WindowContextWidget>()
        ?.context;
    return res!;
  }

  static WindowContext? maybeoOf(BuildContext context) {
    final res = context
        .dependOnInheritedWidgetOfExactType<_WindowContextWidget>()
        ?.context;
    return res;
  }
}

// Used by Window.of(context)
class _WindowContextWidget extends InheritedWidget {
  final WindowContext context;

  _WindowContextWidget({
    required Widget child,
    required this.context,
  }) : super(child: child);

  @override
  bool updateShouldNotify(covariant InheritedWidget oldWidget) {
    return false;
  }
}

class _WindowLayoutInner extends SingleChildRenderObjectWidget {
  final WindowBuilder builtWindow;

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

  WindowBuilder builtWindow;

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
      await builtWindow.updateWindowSize(WindowManager.instance.currentWindow,
          _sanitizeAndSnapToPixelBoundary(size));
      _geometryInProgress = false;
      if (_geometryPending) {
        _geometryPending = false;
        _updateGeometry();
      }
    }
  }
}

class _WindowLayout extends SingleChildRenderObjectWidget {
  final WindowBuilder builtWindow;
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

  WindowBuilder builtWindow;
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
      final win = WindowManager.instance.currentWindow;
      builtWindow.updateWindowConstraints(win, intrinsicSize);

      final maxSize = Size(max(intrinsicSize.width, size.width),
          max(intrinsicSize.height, size.height));

      if (maxSize.width > size.width || maxSize.height > size.height) {
        builtWindow.updateWindowSize(win, maxSize);
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
          await builtWindow.initializeWindow(win, size);
          await builtWindow.updateWindowConstraints(win, size);
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
