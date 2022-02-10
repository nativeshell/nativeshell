import 'dart:ffi';
import 'dart:typed_data';

import 'native_functions.dart';

extension NativeListInt8 on NativeList<Int8> {
  Int8List asTypedList() => data.asTypedList(length);
}

extension NativeListUint8 on NativeList<Uint8> {
  Uint8List asTypedList() => data.asTypedList(length);
  void resize(int newSize) {
    _resize(newSize);
  }
}

extension NativeListInt16 on NativeList<Int16> {
  Int16List asTypedList() => data.asTypedList(length);
}

extension NativeListUInt16 on NativeList<Uint16> {
  Uint16List asTypedList() => data.asTypedList(length);
}

extension NativeListInt32 on NativeList<Int32> {
  Int32List asTypedList() => data.asTypedList(length);
}

extension NativeListUInt32 on NativeList<Uint32> {
  Uint32List asTypedList() => data.asTypedList(length);
}

extension NativeListInt64 on NativeList<Int64> {
  Int64List asTypedList() => data.asTypedList(length);
}

extension NativeListFloat32 on NativeList<Float> {
  Float32List asTypedList() => data.asTypedList(length);
}

extension NativeListFloat64 on NativeList<Double> {
  Float64List asTypedList() => data.asTypedList(length);
}

/// List backed by Rust `Vec<T>`. Supported types are `Uint8`, `Int32`,
/// `Int64`, `Double`.
///
/// Must be freed with `free()` or the `data`, `length` and `capacity` fields
/// can be passed to Rust, where a `Vec<T>` can be recreated using
/// `Vec::from_raw_parts`.
class NativeList<T extends NativeType> {
  static NativeList<T> create<T extends NativeType>(
      NativeFunctions functions, int len) {
    final allocate = functions.vecAllocate<T>();
    return NativeList<T>._(
      functions,
      allocate(len),
      len,
    );
  }

  /// Resizes the underlying vector to new size preserving the data.
  /// Currently only supported on Vec<Uint8>
  void _resize(int newSize) {
    final resize = functions.vecResize<T>();
    _data = resize(
      _data,
      _length,
      newSize,
    );
    _length = newSize;
  }

  /// Frees the underlying vector. It is not safe to access the list data
  /// (`asTypedList`) after calling this method.
  void free() {
    final free = functions.vecFree<T>();
    free(_data, _length);
  }

  NativeFunctions functions;
  NativeList._(this.functions, this._data, this._length);

  Pointer<T> get data => _data;
  int get length => _length;

  Pointer<T> _data;
  int _length;
}
