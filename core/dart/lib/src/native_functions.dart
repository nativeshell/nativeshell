import 'dart:ffi';

import 'package:ffi/ffi.dart';

typedef IsolateId = int;

/// Provides access to NativeShell Core native functions.
class NativeFunctions {
  NativeFunctions._({
    required this.token,
    required this.registerIsolate,
    required this.postMessage,
    required this.vecAllocateInt8,
    required this.vecAllocateUint8,
    required this.vecAllocateInt16,
    required this.vecAllocateUint16,
    required this.vecAllocateInt32,
    required this.vecAllocateUint32,
    required this.vecAllocateInt64,
    required this.vecAllocateFloat,
    required this.vecAllocateDouble,
    required this.vecFreeInt8,
    required this.vecFreeUint8,
    required this.vecFreeInt16,
    required this.vecFreeUint16,
    required this.vecFreeInt32,
    required this.vecFreeUint32,
    required this.vecFreeInt64,
    required this.vecFreeFloat,
    required this.vecFreeDouble,
    required this.vecResizeUint8,
  });

  /// Unique token. Can be used to identify functions from different modules.
  final int token;

  final RegisterIsolate registerIsolate;
  final PostMessage postMessage;

  final VecAllocate<Int8> vecAllocateInt8;
  final VecAllocate<Uint8> vecAllocateUint8;
  final VecAllocate<Int16> vecAllocateInt16;
  final VecAllocate<Uint16> vecAllocateUint16;
  final VecAllocate<Int32> vecAllocateInt32;
  final VecAllocate<Uint32> vecAllocateUint32;
  final VecAllocate<Int64> vecAllocateInt64;
  final VecAllocate<Float> vecAllocateFloat;
  final VecAllocate<Double> vecAllocateDouble;

  final VecFree<Int8> vecFreeInt8;
  final VecFree<Uint8> vecFreeUint8;
  final VecFree<Int16> vecFreeInt16;
  final VecFree<Uint16> vecFreeUint16;
  final VecFree<Int32> vecFreeInt32;
  final VecFree<Uint32> vecFreeUint32;
  final VecFree<Int64> vecFreeInt64;
  final VecFree<Float> vecFreeFloat;
  final VecFree<Double> vecFreeDouble;

  final VecResize<Uint8> vecResizeUint8;

  /// Returns default NativeShell functions. This should only be used in
  /// application code, never for plugins. Each plugin should have own function,
  /// which forwards to the NativeShell call. Otherwise the functions may be
  /// from wrong module and not have access to module state.
  static NativeFunctions getDefault() {
    return NativeFunctions.get(
        DynamicLibrary.process(), "nativeshell_get_ffi_context");
  }

  /// Returns NativeShell functions for given module and symbol name.
  /// Throws [NativeFunctionsException] in case something goes wrong.
  static NativeFunctions get(DynamicLibrary dylib, String symbolName) {
    final init = dylib
        .lookup<NativeFunction<_InitFunctionF>>(symbolName)
        .asFunction<_InitFunction>();

    late Pointer<_GetFunctions> context;
    try {
      final size = sizeOf<_GetFunctions>();
      context = malloc.allocate<_GetFunctions>(size);

      context.ref.size = size;
      context.ref.ffiData = NativeApi.initializeApiDLData;

      final res = NativeFunctionsError.values[init(context)];
      if (res != NativeFunctionsError._noError) {
        throw NativeFunctionsException(res);
      }

      final token = context.ref.registerIsolate.address;

      return NativeFunctions._(
        token: token,
        registerIsolate:
            context.ref.registerIsolate.asFunction<RegisterIsolate>(),
        postMessage: context.ref.postMessage.asFunction<PostMessage>(),
        vecAllocateInt8:
            context.ref.vecAllocateInt8.asFunction<VecAllocate<Int8>>(),
        vecAllocateUint8:
            context.ref.vecAllocateUint8.asFunction<VecAllocate<Uint8>>(),
        vecAllocateInt16:
            context.ref.vecAllocateInt16.asFunction<VecAllocate<Int16>>(),
        vecAllocateUint16:
            context.ref.vecAllocateUint16.asFunction<VecAllocate<Uint16>>(),
        vecAllocateInt32:
            context.ref.vecAllocateInt32.asFunction<VecAllocate<Int32>>(),
        vecAllocateUint32:
            context.ref.vecAllocateUint32.asFunction<VecAllocate<Uint32>>(),
        vecAllocateInt64:
            context.ref.vecAllocateInt64.asFunction<VecAllocate<Int64>>(),
        vecAllocateFloat:
            context.ref.vecAllocateFloat.asFunction<VecAllocate<Float>>(),
        vecAllocateDouble:
            context.ref.vecAllocateDouble.asFunction<VecAllocate<Double>>(),
        vecFreeInt8: context.ref.vecFreeInt8.asFunction<VecFree<Int8>>(),
        vecFreeUint8: context.ref.vecFreeUint8.asFunction<VecFree<Uint8>>(),
        vecFreeInt16: context.ref.vecFreeInt16.asFunction<VecFree<Int16>>(),
        vecFreeUint16: context.ref.vecFreeUint16.asFunction<VecFree<Uint16>>(),
        vecFreeInt32: context.ref.vecFreeInt32.asFunction<VecFree<Int32>>(),
        vecFreeUint32: context.ref.vecFreeUint32.asFunction<VecFree<Uint32>>(),
        vecFreeInt64: context.ref.vecFreeInt64.asFunction<VecFree<Int64>>(),
        vecFreeFloat: context.ref.vecFreeFloat.asFunction<VecFree<Float>>(),
        vecFreeDouble: context.ref.vecFreeDouble.asFunction<VecFree<Double>>(),
        vecResizeUint8:
            context.ref.vecResizeUint8.asFunction<VecResize<Uint8>>(),
      );
    } finally {
      malloc.free(context);
    }
  }

  // Access to NativeVector functions
  VecAllocate<T> vecAllocate<T extends NativeType>() {
    final t = <T>[];
    if (t is List<Int8>) {
      return vecAllocateInt8 as VecAllocate<T>;
    } else if (t is List<Uint8>) {
      return vecAllocateUint8 as VecAllocate<T>;
    } else if (t is List<Int16>) {
      return vecAllocateInt16 as VecAllocate<T>;
    } else if (t is List<Uint16>) {
      return vecAllocateUint16 as VecAllocate<T>;
    } else if (t is List<Int32>) {
      return vecAllocateInt32 as VecAllocate<T>;
    } else if (t is List<Uint32>) {
      return vecAllocateUint32 as VecAllocate<T>;
    } else if (t is List<Int64>) {
      return vecAllocateInt64 as VecAllocate<T>;
    } else if (t is List<Float>) {
      return vecAllocateFloat as VecAllocate<T>;
    } else if (t is List<Double>) {
      return vecAllocateDouble as VecAllocate<T>;
    } else {
      throw UnsupportedError("Unsupported NativeList type");
    }
  }

  VecResize<T> vecResize<T extends NativeType>() {
    final t = <T>[];
    if (t is List<Uint8>) {
      return vecResizeUint8 as VecResize<T>;
    } else {
      throw UnsupportedError("Unsupported NativeList type");
    }
  }

  VecFree<T> vecFree<T extends NativeType>() {
    final t = <T>[];
    if (t is List<Int8>) {
      return vecFreeInt8 as VecFree<T>;
    } else if (t is List<Uint8>) {
      return vecFreeUint8 as VecFree<T>;
    } else if (t is List<Int16>) {
      return vecFreeInt16 as VecFree<T>;
    } else if (t is List<Uint16>) {
      return vecFreeUint16 as VecFree<T>;
    } else if (t is List<Int32>) {
      return vecFreeInt32 as VecFree<T>;
    } else if (t is List<Uint32>) {
      return vecFreeUint32 as VecFree<T>;
    } else if (t is List<Int64>) {
      return vecFreeInt64 as VecFree<T>;
    } else if (t is List<Float>) {
      return vecFreeFloat as VecFree<T>;
    } else if (t is List<Double>) {
      return vecFreeDouble as VecFree<T>;
    } else {
      throw UnsupportedError("Unsupported NativeList type");
    }
  }
}

enum NativeFunctionsError {
  _noError,
  invalidStructSize,
}

extension Message on NativeFunctionsError {
  String get message {
    switch (this) {
      case NativeFunctionsError._noError:
        return "No Error";
      case NativeFunctionsError.invalidStructSize:
        return "NativeShell init structure size differs between rust and dart.";
    }
  }
}

class NativeFunctionsException implements Exception {
  NativeFunctionsException(this.error);

  final NativeFunctionsError error;

  @override
  String toString() => error.message;
}

//
// Internal
//

typedef _InitFunctionF = Int64 Function(Pointer<_GetFunctions>);
typedef _InitFunction = int Function(Pointer<_GetFunctions>);

class _GetFunctions extends Struct {
  // in
  @Int64()
  external int size;
  external Pointer<Void> ffiData;

  // out
  external Pointer<NativeFunction<_RegisterIsolate>> registerIsolate;
  external Pointer<NativeFunction<_PostMessage>> postMessage;
  external Pointer<NativeFunction<_VecAllocate<Int8>>> vecAllocateInt8;
  external Pointer<NativeFunction<_VecAllocate<Uint8>>> vecAllocateUint8;
  external Pointer<NativeFunction<_VecAllocate<Int16>>> vecAllocateInt16;
  external Pointer<NativeFunction<_VecAllocate<Uint16>>> vecAllocateUint16;
  external Pointer<NativeFunction<_VecAllocate<Int32>>> vecAllocateInt32;
  external Pointer<NativeFunction<_VecAllocate<Uint32>>> vecAllocateUint32;
  external Pointer<NativeFunction<_VecAllocate<Int64>>> vecAllocateInt64;
  external Pointer<NativeFunction<_VecAllocate<Float>>> vecAllocateFloat;
  external Pointer<NativeFunction<_VecAllocate<Double>>> vecAllocateDouble;
  external Pointer<NativeFunction<_VecFree<Int8>>> vecFreeInt8;
  external Pointer<NativeFunction<_VecFree<Uint8>>> vecFreeUint8;
  external Pointer<NativeFunction<_VecFree<Int16>>> vecFreeInt16;
  external Pointer<NativeFunction<_VecFree<Uint16>>> vecFreeUint16;
  external Pointer<NativeFunction<_VecFree<Int32>>> vecFreeInt32;
  external Pointer<NativeFunction<_VecFree<Uint32>>> vecFreeUint32;
  external Pointer<NativeFunction<_VecFree<Int64>>> vecFreeInt64;
  external Pointer<NativeFunction<_VecFree<Float>>> vecFreeFloat;
  external Pointer<NativeFunction<_VecFree<Double>>> vecFreeDouble;
  external Pointer<NativeFunction<_VecResize<Uint8>>> vecResizeUint8;
}

typedef _RegisterIsolate = Int64 Function(Int64);
typedef RegisterIsolate = IsolateId Function(int dartPort);

typedef _PostMessage = Void Function(Int64, Pointer<Uint8>, Int64);
typedef PostMessage = void Function(IsolateId, Pointer<Uint8>, int len);

typedef _VecAllocate<T extends NativeType> = Pointer<T> Function(Uint64 size);
typedef VecAllocate<T extends NativeType> = Pointer<T> Function(int size);

typedef _VecResize<T extends NativeType> = Pointer<T> Function(
    Pointer<T> oldData, Uint64 oldSize, Uint64 newSize);
typedef VecResize<T extends NativeType> = Pointer<T> Function(
    Pointer<T> oldData, int oldsize, int newSize);

typedef _VecFree<T extends NativeType> = Void Function(
    Pointer<T> data, Uint64 size);
typedef VecFree<T extends NativeType> = void Function(
    Pointer<T> data, int size);
