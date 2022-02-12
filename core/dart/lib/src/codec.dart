import 'dart:convert';
import 'dart:ffi';
import 'dart:typed_data';

import 'finalizable_handle.dart';
import 'native_functions.dart';
import 'native_list.dart';
import 'write_buffer.dart';
import 'read_buffer.dart';

const int _valueNull = 255 - 0;
const int _valueTrue = 255 - 1;
const int _valueFalse = 255 - 2;
const int _valueInt64 = 255 - 3;
const int _valueFloat64 = 255 - 4;
const int _valueSmallString = 255 - 5; // stored inline

// Serialization
const int _valueString = 255 - 6;
const int _valueInt8List = 255 - 7;
const int _valueUint8List = 255 - 8;
const int _valueInt16List = 255 - 9;
const int _valueUint16List = 255 - 10;
const int _valueInt32List = 255 - 11;
const int _valueUint32List = 255 - 12;
const int _valueInt64List = 255 - 13;
const int _valueFloat32List = 255 - 14;
const int _valueFloat64List = 255 - 15;

// Deserialization
const int _valueAttachment = _valueString;
const int _valueFinalizableHandle = _valueAttachment - 1;

const int _valueList = 255 - 16;
const int _valueMap = 255 - 17;
const int _valueLast = _valueMap;

abstract class FinalizableHandleProvider {
  FinalizableHandle getFinalizableHandle(int id);
}

/// Similar to StandardMessageCodec, but uses NativeList for typed lists
class Serializer {
  Serializer(this._functions);

  final NativeFunctions _functions;

  /// Important: Serialized NativeList must be passed to Rust code and deserialized
  /// otherwise all typed lists will leak.
  NativeList<Uint8> serialize(Object? message) {
    final list = NativeList.create<Uint8>(_functions, 0);
    final buffer = WriteBuffer(list);
    final nativeLists = <NativeList>[list];
    try {
      _writeValue(buffer, message, nativeLists);
    } on Exception {
      for (final NativeList list in nativeLists) {
        list.free();
      }
      rethrow;
    }
    return list;
  }

  void _writeValue(
      WriteBuffer buffer, Object? value, List<NativeList> nativeLists) {
    if (value == null) {
      buffer.putUint8(_valueNull);
    } else if (value is bool) {
      buffer.putUint8(value ? _valueTrue : _valueFalse);
    } else if (value is double) {
      buffer.putUint8(_valueFloat64);
      buffer.putFloat64(value);
    } else if (value is int) {
      if (value >= 0 && value < _valueLast) {
        buffer.putUint8(value);
      } else {
        buffer.putUint8(_valueInt64);
        buffer.putInt64(value);
      }
    } else if (value is String) {
      final encoded = utf8.encoder.convert(value);
      if (encoded.length < 50) {
        buffer.putUint8(_valueSmallString);
        _writeSize(buffer, encoded.length);
        buffer.putUint8List(encoded);
      } else {
        buffer.putUint8(_valueString);
        final bytes = NativeList.create<Uint8>(_functions, encoded.length);
        bytes.asTypedList().setAll(0, encoded);
        _writeNativeList(buffer, bytes);
      }
    } else if (value is Int8List) {
      buffer.putUint8(_valueInt8List);
      final v = NativeList.create<Int8>(_functions, value.length);
      v.asTypedList().setAll(0, value);
      _writeNativeList(buffer, v);
      nativeLists.add(v);
    } else if (value is Uint8List) {
      buffer.putUint8(_valueUint8List);
      final v = NativeList.create<Uint8>(_functions, value.length);
      v.asTypedList().setAll(0, value);
      _writeNativeList(buffer, v);
      nativeLists.add(v);
    } else if (value is Int16List) {
      buffer.putUint8(_valueInt16List);
      final v = NativeList.create<Int16>(_functions, value.length);
      v.asTypedList().setAll(0, value);
      _writeNativeList(buffer, v);
      nativeLists.add(v);
    } else if (value is Uint16List) {
      buffer.putUint8(_valueUint16List);
      final v = NativeList.create<Uint16>(_functions, value.length);
      v.asTypedList().setAll(0, value);
      _writeNativeList(buffer, v);
      nativeLists.add(v);
    } else if (value is Int32List) {
      buffer.putUint8(_valueInt32List);
      final v = NativeList.create<Int32>(_functions, value.length);
      v.asTypedList().setAll(0, value);
      _writeNativeList(buffer, v);
      nativeLists.add(v);
    } else if (value is Uint32List) {
      buffer.putUint8(_valueUint32List);
      final v = NativeList.create<Uint32>(_functions, value.length);
      v.asTypedList().setAll(0, value);
      _writeNativeList(buffer, v);
      nativeLists.add(v);
    } else if (value is Int64List) {
      buffer.putUint8(_valueInt64List);
      final v = NativeList.create<Int64>(_functions, value.length);
      v.asTypedList().setAll(0, value);
      _writeNativeList(buffer, v);
      nativeLists.add(v);
    } else if (value is Float32List) {
      buffer.putUint8(_valueFloat32List);
      final v = NativeList.create<Float>(_functions, value.length);
      v.asTypedList().setAll(0, value);
      _writeNativeList(buffer, v);
      nativeLists.add(v);
    } else if (value is Float64List) {
      buffer.putUint8(_valueFloat64List);
      final v = NativeList.create<Double>(_functions, value.length);
      v.asTypedList().setAll(0, value);
      _writeNativeList(buffer, v);
      nativeLists.add(v);
    } else if (value is List) {
      buffer.putUint8(_valueList);
      _writeSize(buffer, value.length);
      for (final Object? item in value) {
        _writeValue(buffer, item, nativeLists);
      }
    } else if (value is Map) {
      buffer.putUint8(_valueMap);
      _writeSize(buffer, value.length);
      value.forEach((Object? key, Object? value) {
        _writeValue(buffer, key, nativeLists);
        _writeValue(buffer, value, nativeLists);
      });
    } else {
      throw ArgumentError.value(value);
    }
  }

  void _writeNativeList(WriteBuffer buffer, NativeList list) {
    buffer.putInt64(list.data.address);
    _writeSize(buffer, list.length);
  }

  /// Writes a non-negative 32-bit integer [value] to [buffer]
  /// using an expanding 1-5 byte encoding that optimizes for small values.
  ///
  /// This method is intended for use by subclasses overriding
  /// [_writeValue].
  void _writeSize(WriteBuffer buffer, int value) {
    assert(0 <= value && value <= 0xffffffff);
    if (value < 254) {
      buffer.putUint8(value);
    } else if (value <= 0xffff) {
      buffer.putUint8(254);
      buffer.putUint16(value);
    } else {
      buffer.putUint8(255);
      buffer.putUint32(value);
    }
  }
}

class Deserializer {
  Object? deserialize(ByteData data, List attachments,
      FinalizableHandleProvider finalizableHandleProvider) {
    final buffer = ReadBuffer(data);
    return _readValue(buffer, attachments, finalizableHandleProvider);
  }

  Object? _readValue(ReadBuffer buffer, List attachments,
      FinalizableHandleProvider finalizableHandleProvider) {
    if (!buffer.hasRemaining) throw const FormatException('Message corrupted');
    final int type = buffer.getUint8();
    if (type < _valueLast) {
      return type; //small integer
    }
    switch (type) {
      case _valueNull:
        return null;
      case _valueTrue:
        return true;
      case _valueFalse:
        return false;
      case _valueInt64:
        return buffer.getInt64();
      case _valueFloat64:
        return buffer.getFloat64();
      case _valueSmallString:
        final length = _readSize(buffer);
        return utf8.decoder.convert(buffer.getUint8List(length));
      case _valueAttachment:
        final index = _readSize(buffer);
        return attachments[index];
      case _valueFinalizableHandle:
        final id = _readSize(buffer);
        return finalizableHandleProvider.getFinalizableHandle(id);
      case _valueList:
        final int length = _readSize(buffer);
        final List<Object?> result = List<Object?>.filled(length, null);
        for (int i = 0; i < length; i++) {
          result[i] =
              _readValue(buffer, attachments, finalizableHandleProvider);
        }
        return result;
      case _valueMap:
        final int length = _readSize(buffer);
        final Map<Object?, Object?> result = <Object?, Object?>{};
        // Allow deserializing with JSON if keys are all Strings
        bool allStrings = true;
        for (int i = 0; i < length; i++) {
          final key =
              _readValue(buffer, attachments, finalizableHandleProvider);
          result[key] =
              _readValue(buffer, attachments, finalizableHandleProvider);
          if (key is! String) {
            allStrings = false;
          }
        }
        return allStrings ? result.cast<String, Object?>() : result;
      default:
        throw const FormatException('Message corrupted');
    }
  }

  ///
  /// This method is intended for use by subclasses overriding
  /// [readValueOfType].
  int _readSize(ReadBuffer buffer) {
    final int value = buffer.getUint8();
    switch (value) {
      case 254:
        return buffer.getUint16();
      case 255:
        return buffer.getUint32();
      default:
        return value;
    }
  }
}
