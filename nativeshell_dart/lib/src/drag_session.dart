import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

import 'api_constants.dart';
import 'api_model.dart';
import 'drag_drop.dart';
import 'util.dart';
import 'window.dart';
import 'window_method_channel.dart';

final _dragSourceChannel = WindowMethodChannel(Channels.dragSource);

class DragException implements Exception {
  final String message;
  DragException(this.message);
}

class DragSession {
  static DragSession? currentSession() {
    return _DragSessionManager.instance.activeSession;
  }

  static Future<DragSession> beginWithContext({
    required BuildContext context,
    required DragData data,
    required Set<DragEffect> allowedEffects,
  }) async {
    final renderObject_ = context.findRenderObject();
    final renderObject = renderObject_ is RenderRepaintBoundary
        ? renderObject_
        : context.findAncestorRenderObjectOfType<RenderRepaintBoundary>();

    if (renderObject == null) {
      throw DragException("Couldn't find any repaint boundary ancestor");
    }

    final pr = MediaQuery.of(context).devicePixelRatio;
    final snapshot = await renderObject.toImage(pixelRatio: pr);
    final rect = MatrixUtils.transformRect(renderObject.getTransformTo(null),
        Rect.fromLTWH(0, 0, renderObject.size.width, renderObject.size.height));
    return DragSession.beginWithImage(
        window: Window.of(context),
        image: snapshot,
        rect: rect,
        data: data,
        allowedEffects: allowedEffects);
  }

  static Future<DragSession> beginWithImage({
    required LocalWindow window,
    required ui.Image image,
    required Rect rect,
    required DragData data,
    required Set<DragEffect> allowedEffects,
  }) async {
    final bytes = await image.toByteData(format: ui.ImageByteFormat.rawRgba);

    await _dragSourceChannel
        .invokeMethod(window.handle, Methods.dragSourceBeginDragSession, {
      'image': {
        'width': image.width,
        'height': image.height,
        'bytesPerRow': image.width * 4,
        'data': bytes!.buffer.asUint8List()
      },
      'rect': rect.serialize(),
      'data': data.serialize(),
      'allowedEffects':
          allowedEffects.map<String>((e) => enumToString(e)).toList(),
    });

    final res = DragSession();
    _DragSessionManager.instance.registerSession(res);
    return res;
  }

  Future<DragEffect> waitForResult() async {
    if (_result != null) {
      return _result!;
    } else {
      return _completer.future;
    }
  }

  void _setResult(DragEffect result) {
    _result = result;
    _completer.complete(_result);
  }

  DragEffect? _result;
  final _completer = Completer<DragEffect>();
}

class _DragSessionManager {
  static final instance = _DragSessionManager();

  _DragSessionManager() {
    _dragSourceChannel.setMethodCallHandler(_onMethodCall);
  }

  Future<dynamic> _onMethodCall(WindowMethodCall call) async {
    if (call.method == Methods.dragSourceDragSessionEnded) {
      final result =
          enumFromString(DragEffect.values, call.arguments, DragEffect.None);
      assert(_activeSessions.isNotEmpty,
          'Received drag session notification without active drag session.');
      final session = _activeSessions.removeAt(0);
      session._setResult(result);
    }
  }

  DragSession? get activeSession =>
      _activeSessions.isEmpty ? null : _activeSessions.last;

  void registerSession(DragSession session) {
    _activeSessions.add(session);
  }

  // It is possible to have more than one active session; MacOS drag session finished
  // notification can be delayed so we might have nother session already in progress;
  // Last value is current session
  final _activeSessions = <DragSession>[];
}
