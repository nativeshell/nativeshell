import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';

import 'api_constants.dart';
import 'api_model.dart';
import 'util.dart';

enum DragEffect {
  None,
  Copy,
  Link,
  Move,
}

typedef DragDataEncode<T> = dynamic Function(T value);
typedef DragDataDecode<T> = T? Function(dynamic value);

dynamic _defaultEncode<T>(T t) => t;
T _defaultDecode<T>(dynamic t) => t;

class DragDataKey<T> {
  DragDataKey(
    String name, {
    DragDataEncode<T>? encode,
    DragDataDecode<T>? decode,
  })  : _name = name,
        _encode = encode ?? _defaultEncode,
        _decode = decode ?? _defaultDecode;

  _DragDataInitProperty call(T value) =>
      _DragDataInitProperty(key: _name, value: _encode(value));

  final String _name;
  final DragDataEncode<T> _encode;
  final DragDataDecode<T> _decode;
}

class _DragDataInitProperty {
  final String key;
  final dynamic value;

  _DragDataInitProperty({
    required this.key,
    this.value,
  });
}

dynamic _encodeURLs(List<Uri> urls) {
  return urls.map((e) => e.toString()).toList();
}

List<Uri> _decodeURLs(dynamic urls) {
  final list = urls as List;
  return list.map((e) => Uri.parse(e as String)).toList();
}

List<String> _decodeFiles(dynamic files) {
  final list = files as List;
  return list.cast<String>();
}

dynamic _encodeFiles(List<String> files) {
  return files;
}

class DragData {
  // Predefined keys
  static final files = DragDataKey<List<String>>(Keys.dragDataFiles,
      encode: _encodeFiles, decode: _decodeFiles);

  // While this is defined as List, only one URI is supported on Windows
  static final uris = DragDataKey<List<Uri>>(Keys.dragDataURLs,
      encode: _encodeURLs, decode: _decodeURLs);

  // Usage
  //
  // final data = DragData([
  //   DragData.files(['file-path-1', 'file-path-'2])
  // ])
  //
  // final files = data.get(DragData.files);
  //
  DragData(List<_DragDataInitProperty> properties)
      : _properties =
            Map.fromEntries(properties.map((e) => MapEntry(e.key, e.value)));

  bool contains(DragDataKey key) {
    return _properties.containsKey(key);
  }

  Future<T?> get<T>(DragDataKey<T> key) async {
    // Access to values is async for future proofing;
    // Some platforms may only allow accessing data asynchronously
    final res = _properties[key._name];
    if (res != null) {
      return key._decode(res);
    } else {
      return null;
    }
  }

  dynamic serialize() => {'properties': _properties};

  static DragData deserialize(dynamic value) {
    final map = value as Map;
    final properties = map['properties'] as Map;
    return DragData._withProperties(properties.cast<String, dynamic>());
  }

  DragData._withProperties(Map<String, dynamic> properties)
      : _properties = properties;

  final Map<String, dynamic> _properties;
}

class DropEvent {
  DropEvent({
    required this.info,
  });

  DropEvent transformed(Matrix4 matrix) {
    return DropEvent(
      info:
          info.withLocation(MatrixUtils.transformPoint(matrix, info.location)),
    );
  }

  @override
  String toString() {
    return 'DragEvent: ${info.toString()}';
  }

  final DragInfo info;
}

typedef DropEventListener = FutureOr<DragEffect> Function(DropEvent);
typedef DropExitListener = void Function();
typedef PerformDropListener = void Function(DropEvent);

class RawDropRegion extends SingleChildRenderObjectWidget {
  final DropEventListener? onDropOver;
  final DropExitListener? onDropExit;
  final PerformDropListener? onPerformDrop;

  RawDropRegion({
    Key? key,
    Widget? child,
    this.onDropOver,
    this.onDropExit,
    this.onPerformDrop,
  }) : super(key: key, child: child);

  @override
  RenderObject createRenderObject(BuildContext context) {
    return RenderDropRegion(
        onDropOver: onDropOver,
        onDropExit: onDropExit,
        onPerformDrop: onPerformDrop);
  }

  @override
  void updateRenderObject(BuildContext context, RenderDropRegion renderObject) {
    renderObject
      ..onDropOver = onDropOver
      ..onDropExit = onDropExit
      ..onPerformDrop = onPerformDrop;
  }
}

class RenderDropRegion extends RenderProxyBox {
  FutureOr<DragEffect> handleOnDrop(DropEvent event, HitTestEntry entry) async {
    final onDropOver = this.onDropOver;
    if (onDropOver != null) {
      return onDropOver(
          event.transformed(entry.transform ?? Matrix4.identity()));
    } else {
      return DragEffect.None;
    }
  }

  @override
  bool hitTestSelf(ui.Offset position) {
    return true;
  }

  void handleOnDropExit(HitTestEntry entry) {
    final onDropExit = this.onDropExit;
    if (onDropExit != null) {
      onDropExit();
    }
  }

  void handlePerformDrop(DropEvent event, HitTestEntry entry) {
    final onPerformDrop = this.onPerformDrop;
    if (onPerformDrop != null) {
      onPerformDrop(event.transformed(entry.transform ?? Matrix4.identity()));
    }
  }

  RenderDropRegion(
      {this.onDropOver, this.onDropExit, this.onPerformDrop, RenderBox? child})
      : super(child);

  DropEventListener? onDropOver;
  DropExitListener? onDropExit;
  PerformDropListener? onPerformDrop;
}

class DragInfo {
  DragInfo({
    required this.location,
    required this.data,
    required this.allowedEffects,
  });

  final Offset location;
  final DragData data;
  final Set<DragEffect> allowedEffects;

  DragInfo withLocation(Offset location) => DragInfo(
        location: location,
        data: data,
        allowedEffects: allowedEffects,
      );

  static DragInfo deserialize(dynamic value) {
    final map = value as Map;
    return DragInfo(
        location: OffsetExt.deserialize(map['location']),
        data: DragData.deserialize(map['data']),
        allowedEffects: Set<DragEffect>.from((map['allowedEffects'] as List)
            .map(
                (e) => enumFromString(DragEffect.values, e, DragEffect.None))));
  }

  Map serialize() => {
        'location': location.serialize(),
        'data': data.serialize(),
        'allowedEffects': allowedEffects.map((e) => enumToString(e)).toList(),
      };

  @override
  String toString() => serialize().toString();
}

class DropRegion extends StatefulWidget {
  const DropRegion({
    Key? key,
    this.onDropEnter,
    this.onDropExit,
    this.onDropOver,
    this.onPerformDrop,
    required this.child,
  }) : super(key: key);

  final VoidCallback? onDropEnter;
  final VoidCallback? onDropExit;
  final DropEventListener? onDropOver;
  final PerformDropListener? onPerformDrop;
  final Widget child;

  @override
  State<StatefulWidget> createState() {
    return DropRegionState();
  }
}

class DropRegionState extends State<DropRegion> {
  var inside = false;

  @override
  Widget build(BuildContext context) {
    return RawDropRegion(
      onDropOver: _onDropOver,
      onDropExit: _onDropExit,
      onPerformDrop: _onPerformDrop,
      child: widget.child,
    );
  }

  Future<DragEffect> _onDropOver(DropEvent info) async {
    var effect = DragEffect.None;
    if (widget.onDropOver != null) {
      effect = await widget.onDropOver!(info);
    }
    if (effect != DragEffect.None && !inside) {
      inside = true;
      if (widget.onDropEnter != null) {
        widget.onDropEnter!();
      }
    } else if (effect == DragEffect.None && inside) {
      inside = false;
      if (widget.onDropExit != null) {
        widget.onDropExit!();
      }
    }
    return effect;
  }

  void _onDropExit() {
    if (inside) {
      inside = false;
      if (widget.onDropExit != null) {
        widget.onDropExit!();
      }
    }
  }

  void _onPerformDrop(DropEvent info) {
    if (inside && widget.onPerformDrop != null) {
      widget.onPerformDrop!(info);
    }
    _onDropExit();
  }
}

class DragDriver {
  RenderDropRegion? _lastDropRegion;
  HitTestEntry? _lastDropRegionEntry;

  Future<DragEffect> draggingUpdated(DragInfo info) async {
    var res = DragEffect.None;
    final hitTest = HitTestResult();
    final event = DropEvent(info: info);
    RenderDropRegion? dropRegion;
    HitTestEntry? entry;
    GestureBinding.instance!.hitTest(hitTest, info.location);

    for (final item in hitTest.path) {
      final target = item.target;
      if (target is RenderDropRegion) {
        res = await target.handleOnDrop(event, item);
        if (res != DragEffect.None) {
          dropRegion = target;
          entry = item;
          break;
        }
      }
    }
    if (_lastDropRegion != dropRegion && _lastDropRegion != null) {
      _lastDropRegion!.handleOnDropExit(_lastDropRegionEntry!);
    }
    _lastDropRegion = dropRegion;
    _lastDropRegionEntry = entry;

    return res;
  }

  void draggingExited() {
    if (_lastDropRegion != null) {
      _lastDropRegion!.handleOnDropExit(_lastDropRegionEntry!);
      _lastDropRegion = null;
      _lastDropRegionEntry = null;
    }
  }

  void performDrop(DragInfo info) async {
    final res = await draggingUpdated(info);
    if (res != DragEffect.None) {
      assert(_lastDropRegion != null);
      final event = DropEvent(info: info);
      _lastDropRegion!.handlePerformDrop(event, _lastDropRegionEntry!);
      _lastDropRegion = null;
      _lastDropRegionEntry = null;
    }
  }
}
