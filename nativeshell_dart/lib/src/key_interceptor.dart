import 'dart:async';
import 'dart:convert';

import 'package:flutter/cupertino.dart';
import 'package:flutter/services.dart';
import 'package:pedantic/pedantic.dart';

class RawKeyEventEx {
  RawKeyEventEx(
      {required RawKeyEvent event,
      required this.keyWithoutModifiers,
      this.keyWithoutModifiers2})
      : event = event,
        controlPressed = event.isControlPressed,
        altPressed = event.isAltPressed,
        metaPressed = event.isMetaPressed,
        shiftPressed = event.isShiftPressed;

  // Original key event
  final RawKeyEvent event;

  // Key event with "original" key without modifiers
  final LogicalKeyboardKey keyWithoutModifiers;

  // Alternate key without modifiers; This would be with shift applied, but only
  // if shift is pressed; This is used to handle accelerators such as shift + } on
  // US keyboard; Note that this will also match shift + ]; There is no way to
  // distinguish these two, so we match either
  final LogicalKeyboardKey? keyWithoutModifiers2;

  final bool controlPressed;
  final bool altPressed;
  final bool metaPressed;
  final bool shiftPressed;
}

typedef KeyInterceptorHandler = bool Function(RawKeyEventEx event);

enum InterceptorStage {
  pre, // interceptor executed before flutter keyboard handler (for all events)
  post, // interceptor executed after flutter keyboard handler (for unhandled events only)
}

class KeyInterceptor {
  KeyInterceptor._() {
    WidgetsFlutterBinding.ensureInitialized();
    _channel.setMessageHandler(_onMessage);
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

  static ByteData _dataForHandled(bool handled) {
    final res = <String, dynamic>{'handled': handled};
    return StringCodec().encodeMessage(json.encode(res))!;
  }

  Future<ByteData> _onMessage(ByteData? message) async {
    final keyMessage = json.decode(StringCodec().decodeMessage(message) ?? '');

    final event = _keyEventFromMessage(keyMessage);

    for (final handler in List<KeyInterceptorHandler>.from(_handlersPre)) {
      if (handler(event)) {
        return Future.value(_dataForHandled(true));
      }
    }

    final completer = Completer<ByteData>();
    unawaited(WidgetsBinding.instance?.defaultBinaryMessenger
        .handlePlatformMessage('flutter/keyevent', message, (data) {
      // macos with FN pressed seems to return null?
      data ??= _dataForHandled(false);

      completer.complete(data);
    }));
    final data = await completer.future;
    final response = json.decode(StringCodec().decodeMessage(data) ?? '');
    if (response['handled'] == false) {
      for (final handler in List<KeyInterceptorHandler>.from(_handlersPost)) {
        if (handler(event)) {
          return Future.value(_dataForHandled(true));
        }
      }
    }
    return data;
  }
}

const _channel = BasicMessageChannel('nativeshell/keyevent', BinaryCodec());

RawKeyEventEx _keyEventFromMessage(Map<String, dynamic> message) {
  final noModifiers = message['charactersIgnoringModifiersEx'] as String?;
  final noModifiersExceptShift =
      message['charactersIgnoringModifiersExceptShiftEx'] as String?;
  final event = RawKeyEvent.fromMessage(message);

  var noModifiersKey = event.logicalKey;
  var noModifiersExceptShiftKey;

  if (noModifiers != null) {
    noModifiersKey = _keyFromCharacters(noModifiers, event);
  }

  if (noModifiersExceptShift != null && event.isShiftPressed) {
    noModifiersExceptShiftKey =
        _keyFromCharacters(noModifiersExceptShift, event);
  }

  return RawKeyEventEx(
      event: event,
      keyWithoutModifiers: noModifiersKey,
      keyWithoutModifiers2: noModifiersExceptShiftKey);
}

LogicalKeyboardKey _keyFromCharacters(String characters, RawKeyEvent event) {
  final data = event.data;
  if (data is RawKeyEventDataMacOs) {
    final newEvent = RawKeyEventDataMacOs(
      characters: characters,
      charactersIgnoringModifiers: characters,
      keyCode: data.keyCode,
      modifiers: data.modifiers,
    );
    return newEvent.logicalKey;
  } else if (data is RawKeyEventDataWindows) {
    final newEvent = RawKeyEventDataWindows(
      characterCodePoint: characters.codeUnitAt(0),
      keyCode: data.keyCode,
      modifiers: data.modifiers,
      scanCode: data.scanCode,
    );
    return newEvent.logicalKey;
  } else if (data is RawKeyEventDataLinux) {
    final newEvent = RawKeyEventDataLinux(
      keyHelper: data.keyHelper,
      unicodeScalarValues: characters.codeUnitAt(0),
      keyCode: characters.codeUnitAt(0),
      scanCode: data.scanCode,
      modifiers: data.modifiers,
      isDown: data.isDown,
    );
    return newEvent.logicalKey;
  } else {
    return event.logicalKey;
  }
}
