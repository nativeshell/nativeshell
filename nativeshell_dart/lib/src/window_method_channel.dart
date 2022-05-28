import 'dart:async';
import 'dart:collection';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import 'api_constants.dart';
import 'window.dart';

class WindowMethodCall {
  const WindowMethodCall(this.targetWindowHandle, this.method,
      [this.arguments]);

  final WindowHandle targetWindowHandle;
  final String method;
  final dynamic arguments;

  @override
  String toString() =>
      '${objectRuntimeType(this, 'MethodCall')}($method, $arguments)';
}

class WindowMessage {
  WindowMessage(this.sourceWindowHandle, this.message, [this.arguments]);

  final WindowHandle sourceWindowHandle;
  final String message;
  final dynamic arguments;

  @override
  String toString() =>
      '${objectRuntimeType(this, 'MethodCall')}($message, $arguments)';
}

typedef MethodHandler = Future<dynamic> Function(WindowMethodCall call);
typedef MessageHandler = FutureOr<void> Function(WindowMessage message);

class WindowMethodDispatcher {
  WindowMethodDispatcher() : _codec = const StandardMessageCodec() {
    _binaryMessenger.setMessageHandler(Channels.dispatcher, _handleMessage);
  }

  final MessageCodec _codec;
  BinaryMessenger get _binaryMessenger =>
      ServicesBinding.instance.defaultBinaryMessenger;

  Future<T> invokeMethod<T>({
    required String channel,
    required String method,
    dynamic arguments,
    required WindowHandle targetWindowHandle,
  }) async {
    final envelope = {
      'targetWindowHandle': targetWindowHandle.value,
      'channel': channel,
      'method': method,
      'arguments': arguments,
    };
    final encoded = _codec.encodeMessage(envelope);
    final res = await _binaryMessenger.send(Channels.dispatcher, encoded);
    final decoded = _codec.decodeMessage(res);

    if (decoded is! Map) {
      throw PlatformException(
          code: 'format', message: 'Invalid response format');
    }
    final code = decoded['code'] as String?;
    final message = decoded['message'] as String?;
    if (code != null) {
      throw PlatformException(code: code, message: message);
    }
    return decoded['result'];
  }

  void registerMethodHandler(String channelName, MethodHandler? handler) {
    if (handler != null) {
      _methodHandlers[channelName] = handler;
    } else {
      _methodHandlers.remove(channelName);
    }
  }

  int registerMessageHandler(String channelName, MessageHandler handler) {
    var id = ++_nextMessageHandler;
    final map = _messageHandlers.putIfAbsent(channelName, () => HashMap());
    map[id] = handler;
    return id;
  }

  void unregisterMessageHandler(int handler) {
    for (final map in _messageHandlers.entries) {
      map.value.remove(handler);
    }
  }

  int _nextMessageHandler = 1;

  final _methodHandlers = <String, MethodHandler>{};
  final _messageHandlers = <String, Map<int, MessageHandler>>{};

  Future<ByteData?> _handleMessage(ByteData? message) async {
    final decoded = _codec.decodeMessage(message);
    try {
      final map = decoded as Map;
      final sourceWindowHandle = map['sourceWindowHandle'] as int?;
      final targetWindowHandle = map['targetWindowHandle'] as int?;
      final channel = map['channel'] as String;
      final method = map['method'] as String?;
      final message = map['message'] as String?;
      final arguments = map['arguments'];

      if (method != null) {
        final call = WindowMethodCall(
            WindowHandle(targetWindowHandle!), method, arguments);

        final handler = _methodHandlers[channel];
        if (handler == null) {
          return _encodeError('missing-handler', 'Missing handler for channel');
        } else {
          final res = await handler(call);
          return _codec.encodeMessage({
            'result': res,
          });
        }
      } else if (message != null) {
        final call = WindowMessage(
            WindowHandle(sourceWindowHandle!), message, arguments);
        final handlers = _messageHandlers[channel];
        if (handlers != null) {
          for (final h in handlers.values) {
            try {
              h(call);
            } catch (e) {
              print(e);
              // TODO log
            }
          }
        }
        return null;
      }
      return _encodeError('error', 'Malformed message');
    } on PlatformException catch (e) {
      print(e);
      return _encodeError(e.code, e.message);
    } catch (e) {
      print(e);
      return _encodeError('error', e.toString());
    }
  }

  ByteData _encodeError(String code, String? message) {
    return _codec.encodeMessage({
      'code': code,
      'message': message,
    })!;
  }

  static WindowMethodDispatcher get instance => _dispatcher;
}

final _dispatcher = WindowMethodDispatcher();

class WindowMethodChannel {
  final String name;

  const WindowMethodChannel(this.name);

  Future<T?> invokeMethod<T>(WindowHandle targetWindowHandle, String method,
      [dynamic arguments]) {
    return _dispatcher.invokeMethod(
        channel: name,
        method: method,
        arguments: arguments,
        targetWindowHandle: targetWindowHandle);
  }

  void setMethodCallHandler(MethodHandler? handler) {
    _dispatcher.registerMethodHandler(name, handler);
  }
}
