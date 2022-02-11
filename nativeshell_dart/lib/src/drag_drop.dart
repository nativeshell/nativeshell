import 'dart:async';

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

  @override
  String toString() {
    return 'DragEvent: ${info.toString()}';
  }

  final DragInfo info;
}

typedef DropMonitorListener = void Function(DropEvent event,
    {required bool isInside});

// Widget that passively listens for drop events. Unlike (Raw)DropRegion,
// DropMonitor can not set drop effect and does not get performDrop
// notifications, however there can be multiple DropMonitor widgets nested
// and each of them will get the notification, whereas there can be only one
// DropRegion active.
// After first notification, DropMonitor will keep getting notifications until
// drop pointer leaves the window. Check the `isInside` argument in
// DropMonitorListener to see whether the pointer is inside monitor.
class DropMonitor extends SingleChildRenderObjectWidget {
  final DropMonitorListener? onDropOver;
  final DropExitListener? onDropExit;
  final HitTestBehavior behavior;

  DropMonitor({
    Key? key,
    Widget? child,
    this.onDropOver,
    this.onDropExit,
    this.behavior = HitTestBehavior.deferToChild,
  }) : super(key: key, child: child);

  @override
  RenderObject createRenderObject(BuildContext context) {
    return _RenderDropMonitor(
        onDropOver: onDropOver, onDropExit: onDropExit, behavior: behavior);
  }

  @override
  void updateRenderObject(
      BuildContext context, _RenderDropMonitor renderObject) {
    renderObject
      ..onDropOver = onDropOver
      ..onDropExit = onDropExit
      ..behavior = behavior;
  }
}

typedef DropEventListener = FutureOr<DragEffect> Function(DropEvent);
typedef DropExitListener = void Function();
typedef PerformDropListener = void Function(DropEvent);

class RawDropRegion extends SingleChildRenderObjectWidget {
  final DropEventListener? onDropOver;
  final DropExitListener? onDropExit;
  final PerformDropListener? onPerformDrop;
  final HitTestBehavior behavior;

  RawDropRegion({
    Key? key,
    Widget? child,
    this.onDropOver,
    this.onDropExit,
    this.onPerformDrop,
    this.behavior = HitTestBehavior.deferToChild,
  }) : super(key: key, child: child);

  @override
  RenderObject createRenderObject(BuildContext context) {
    return _RenderDropRegion(
        onDropOver: onDropOver,
        onDropExit: onDropExit,
        onPerformDrop: onPerformDrop,
        behavior: behavior);
  }

  @override
  void updateRenderObject(
      BuildContext context, _RenderDropRegion renderObject) {
    renderObject
      ..onDropOver = onDropOver
      ..onDropExit = onDropExit
      ..onPerformDrop = onPerformDrop
      ..behavior = behavior;
  }
}

class _RenderDropRegion extends RenderProxyBoxWithHitTestBehavior {
  FutureOr<DragEffect> handleOnDrop(DropEvent event) async {
    final onDropOver = this.onDropOver;
    if (onDropOver != null) {
      final transformed = DropEvent(
          info: event.info
              .withPosition(globalToLocal(event.info.globalPosition)));
      return onDropOver(transformed);
    } else {
      return DragEffect.None;
    }
  }

  void handleOnDropExit() {
    final onDropExit = this.onDropExit;
    if (onDropExit != null) {
      onDropExit();
    }
  }

  void handlePerformDrop(DropEvent event) {
    final onPerformDrop = this.onPerformDrop;
    if (onPerformDrop != null) {
      final transformed = DropEvent(
          info: event.info
              .withPosition(globalToLocal(event.info.globalPosition)));
      onPerformDrop(transformed);
    }
  }

  _RenderDropRegion({
    this.onDropOver,
    this.onDropExit,
    this.onPerformDrop,
    RenderBox? child,
    required HitTestBehavior behavior,
  }) : super(behavior: behavior, child: child);

  DropEventListener? onDropOver;
  DropExitListener? onDropExit;
  PerformDropListener? onPerformDrop;
}

class _RenderDropMonitor extends RenderProxyBoxWithHitTestBehavior {
  void handleOnDrop(DropEvent event, {required bool isInside}) {
    final onDropOver = this.onDropOver;
    if (onDropOver != null) {
      final transformed = DropEvent(
          info: event.info
              .withPosition(globalToLocal(event.info.globalPosition)));
      onDropOver(transformed, isInside: isInside);
    }
  }

  void handleOnDropExit() {
    final onDropExit = this.onDropExit;
    if (onDropExit != null) {
      onDropExit();
    }
  }

  _RenderDropMonitor({
    this.onDropOver,
    this.onDropExit,
    RenderBox? child,
    required HitTestBehavior behavior,
  }) : super(behavior: behavior, child: child);

  DropMonitorListener? onDropOver;
  DropExitListener? onDropExit;
}

class DragInfo {
  DragInfo({
    required this.position,
    required this.globalPosition,
    required this.data,
    required this.allowedEffects,
  });

  final Offset position;
  final Offset globalPosition;
  final DragData data;
  final Set<DragEffect> allowedEffects;

  DragInfo withPosition(Offset position) => DragInfo(
        position: position,
        globalPosition: globalPosition,
        data: data,
        allowedEffects: allowedEffects,
      );

  static DragInfo deserialize(dynamic value) {
    final map = value as Map;
    final position = OffsetExt.deserialize(map['location']);
    return DragInfo(
        position: position,
        globalPosition: position,
        data: DragData.deserialize(map['data']),
        allowedEffects: Set<DragEffect>.from((map['allowedEffects'] as List)
            .map(
                (e) => enumFromString(DragEffect.values, e, DragEffect.None))));
  }

  Map serialize() => {
        'location': position.serialize(),
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
    this.behavior = HitTestBehavior.deferToChild,
    required this.child,
  }) : super(key: key);

  final VoidCallback? onDropEnter;
  final VoidCallback? onDropExit;
  final DropEventListener? onDropOver;
  final PerformDropListener? onPerformDrop;
  final HitTestBehavior behavior;
  final Widget child;

  @override
  State<StatefulWidget> createState() {
    return DropRegionState();
  }
}

class DropListener extends StatelessWidget {
  const DropListener({
    Key? key,
    required this.child,
  }) : super(key: key);

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return MetaData();
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
      behavior: widget.behavior,
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
  _RenderDropRegion? _lastDropRegion;
  final _allDropMonitors = <_RenderDropMonitor>{};

  Future<DragEffect> draggingUpdated(DragInfo info) async {
    var res = DragEffect.None;
    final hitTest = HitTestResult();
    final event = DropEvent(info: info);
    _RenderDropRegion? dropRegion;
    final monitors = <_RenderDropMonitor>[];

    _allDropMonitors.removeWhere((element) => !element.attached);

    // ignore: unnecessary_non_null_assertion
    GestureBinding.instance!.hitTest(hitTest, info.position);

    for (final item in hitTest.path) {
      final target = item.target;
      if (target is _RenderDropRegion && dropRegion == null) {
        res = await target.handleOnDrop(event);
        if (res != DragEffect.None) {
          dropRegion = target;
        }
      }
      if (target is _RenderDropMonitor) {
        monitors.add(target);
      }
    }
    if (_lastDropRegion != dropRegion && _lastDropRegion != null) {
      _lastDropRegion!.handleOnDropExit();
    }
    _lastDropRegion = dropRegion;

    monitors.forEach((element) => element.handleOnDrop(event, isInside: true));

    _allDropMonitors.forEach((element) {
      if (!monitors.contains(element)) {
        element.handleOnDrop(event, isInside: false);
      }
    });
    _allDropMonitors.addAll(monitors);

    return res;
  }

  void draggingExited() {
    _allDropMonitors.removeWhere((element) => !element.attached);

    if (_lastDropRegion != null) {
      _lastDropRegion!.handleOnDropExit();
      _lastDropRegion = null;
    }
    _allDropMonitors.forEach((element) => element.handleOnDropExit());
    _allDropMonitors.clear();
  }

  void performDrop(DragInfo info) async {
    _allDropMonitors.removeWhere((element) => !element.attached);

    final res = await draggingUpdated(info);
    if (res != DragEffect.None) {
      assert(_lastDropRegion != null);
      final event = DropEvent(info: info);
      _lastDropRegion!.handlePerformDrop(event);
      _lastDropRegion = null;
    }
    _allDropMonitors.forEach((element) => element.handleOnDropExit());
    _allDropMonitors.clear();
  }
}
