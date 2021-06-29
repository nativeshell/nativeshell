import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

typedef KeyInterceptorHandler = bool Function(RawKeyEvent event);

enum InterceptorStage {
  pre, // interceptor executed before flutter keyboard handler (for all events)
  post, // interceptor executed after flutter keyboard handler (for unhandled events only)
}

class KeyInterceptor {
  KeyInterceptor._() {
    WidgetsFlutterBinding.ensureInitialized();
    _previousHandler = RawKeyboard.instance.keyEventHandler;
    RawKeyboard.instance.keyEventHandler = _onRawKeyEvent;
  }

  void registerHandler(
    KeyInterceptorHandler handler, {
    required InterceptorStage stage,
  }) {
    if (stage == InterceptorStage.pre) {
      _handlersPre.add(handler);
    } else {
      _handlersPost.add(handler);
    }
  }

  void unregisterHandler(
    KeyInterceptorHandler handler, {
    required InterceptorStage stage,
  }) {
    if (stage == InterceptorStage.pre) {
      _handlersPre.remove(handler);
    } else {
      _handlersPost.remove(handler);
    }
  }

  static final KeyInterceptor instance = KeyInterceptor._();

  final _handlersPre = <KeyInterceptorHandler>[];
  final _handlersPost = <KeyInterceptorHandler>[];

  RawKeyEventHandler? _previousHandler;

  bool _onRawKeyEvent(RawKeyEvent event) {
    for (final handler in List<KeyInterceptorHandler>.from(_handlersPre)) {
      if (handler(event)) {
        return true;
      }
    }
    if (_previousHandler != null && _previousHandler!(event)) {
      return true;
    }
    for (final handler in List<KeyInterceptorHandler>.from(_handlersPost)) {
      if (handler(event)) {
        return true;
      }
    }
    return false;
  }
}
