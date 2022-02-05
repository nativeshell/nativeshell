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
    _previousHandler =
        // ignore: unnecessary_non_null_assertion
        ServicesBinding.instance!.keyEventManager.keyMessageHandler;
    // ignore: unnecessary_non_null_assertion
    ServicesBinding.instance!.keyEventManager.keyMessageHandler = _onMessage;
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

  KeyMessageHandler? _previousHandler;

  bool _onMessage(KeyMessage message) {
    // rawEvent has changed from RawKeyEvent to RawKeyEvent?. We need to
    // support both.
    final rawEvent = (message.rawEvent as dynamic) as RawKeyEvent?;
    if (rawEvent != null) {
      for (final handler in List<KeyInterceptorHandler>.from(_handlersPre)) {
        if (handler(rawEvent)) {
          return true;
        }
      }
    }
    if (_previousHandler != null && _previousHandler!(message)) {
      return true;
    }
    if (rawEvent != null) {
      for (final handler in List<KeyInterceptorHandler>.from(_handlersPost)) {
        if (handler(rawEvent)) {
          return true;
        }
      }
    }
    return false;
  }
}
