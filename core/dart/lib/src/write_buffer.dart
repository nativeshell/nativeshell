import 'dart:ffi';
import 'dart:math';
import 'dart:typed_data';

import 'native_list.dart';

/// Write-only buffer for incrementally building a [ByteData] instance.
///
/// A WriteBuffer instance can be used only once. Attempts to reuse will result
/// in [StateError]s being thrown.
///
/// The byte order used is [Endian.host] throughout.
class WriteBuffer {
  /// Creates an interface for incrementally building a [ByteData] instance.
  WriteBuffer(NativeList<Uint8> list)
      : _buffer = _NativeBuffer(list),
        _isDone = false,
        _eightBytes = ByteData(8) {
    _eightBytesAsList = _eightBytes.buffer.asUint8List();
  }

  int count = 0;

  final _NativeBuffer _buffer;
  bool _isDone;
  final ByteData _eightBytes;
  late Uint8List _eightBytesAsList;
  static final Uint8List _zeroBuffer =
      Uint8List.fromList(<int>[0, 0, 0, 0, 0, 0, 0, 0]);

  /// Write a Uint8 into the buffer.
  void putUint8(int byte) {
    assert(!_isDone);
    _buffer.add(byte);
  }

  /// Write a Uint16 into the buffer.
  void putUint16(int value, {Endian? endian}) {
    assert(!_isDone);
    _eightBytes.setUint16(0, value, endian ?? Endian.host);
    _buffer.addAll(_eightBytesAsList, 0, 2);
  }

  /// Write a Uint32 into the buffer.
  void putUint32(int value, {Endian? endian}) {
    assert(!_isDone);
    _eightBytes.setUint32(0, value, endian ?? Endian.host);
    _buffer.addAll(_eightBytesAsList, 0, 4);
  }

  /// Write an Int32 into the buffer.
  void putInt32(int value, {Endian? endian}) {
    assert(!_isDone);
    _eightBytes.setInt32(0, value, endian ?? Endian.host);
    _buffer.addAll(_eightBytesAsList, 0, 4);
  }

  /// Write an Int64 into the buffer.
  void putInt64(int value, {Endian? endian}) {
    ++count;
    assert(!_isDone);
    _eightBytes.setInt64(0, value, endian ?? Endian.host);
    _buffer.addAll(_eightBytesAsList, 0, 8);
  }

  /// Write an Float64 into the buffer.
  void putFloat64(double value, {Endian? endian}) {
    assert(!_isDone);
    _alignTo(8);
    _eightBytes.setFloat64(0, value, endian ?? Endian.host);
    _buffer.addAll(_eightBytesAsList);
  }

  /// Write all the values from a [Uint8List] into the buffer.
  void putUint8List(Uint8List list) {
    assert(!_isDone);
    _buffer.addAll(list);
  }

  void _alignTo(int alignment) {
    assert(!_isDone);
    final int mod = _buffer.length % alignment;
    if (mod != 0) {
      _buffer.addAll(_zeroBuffer, 0, alignment - mod);
    }
  }

  /// Finalize and return the written [ByteData].
  void done() {
    if (_isDone) {
      throw StateError(
          'done() must not be called more than once on the same $runtimeType.');
    }
    _isDone = true;
    _buffer.list.resize(_buffer.length);
  }
}

class _NativeBuffer {
  _NativeBuffer(this.list);

  void add(int value) {
    _growIfNecessary(1);
    list.asTypedList()[length++] = value;
  }

  void addAll(Iterable<int> values, [int start = 0, int? end]) {
    int valuesLen = end != null ? end - start : values.length - start;
    _growIfNecessary(valuesLen);
    list.asTypedList().setRange(length, length + valuesLen, values, start);
    length += valuesLen;
  }

  void _growIfNecessary(int requiredSize) {
    final requiredLen = length + requiredSize;
    if (requiredLen > list.length) {
      list.resize(max(list.length * 2, length + requiredSize));
    }
  }

  int length = 0;
  final NativeList<Uint8> list;
}
