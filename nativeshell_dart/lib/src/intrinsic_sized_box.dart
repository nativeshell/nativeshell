import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

// Simple Widget that can provide intrinsic size; Useful when child
// is unable to provide one
class IntrinsicSizedBox extends SingleChildRenderObjectWidget {
  const IntrinsicSizedBox({
    Key? key,
    this.intrinsicWidth,
    this.intrinsicHeight,
    Widget? child,
  }) : super(key: key, child: child);

  final double? intrinsicWidth;
  final double? intrinsicHeight;

  @override
  _RenderIntrinsicSizedBox createRenderObject(BuildContext context) {
    return _RenderIntrinsicSizedBox(
      intrinsicWidth: intrinsicWidth,
      intrinsicHeight: intrinsicHeight,
    );
  }

  @override
  void updateRenderObject(
      BuildContext context, covariant RenderObject renderObject) {
    final object = renderObject as _RenderIntrinsicSizedBox;
    object.intrinsicWidth = intrinsicWidth;
    object.intrinsicHeight = intrinsicHeight;
  }
}

class _RenderIntrinsicSizedBox extends RenderProxyBox {
  _RenderIntrinsicSizedBox({
    RenderBox? child,
    this.intrinsicWidth,
    this.intrinsicHeight,
  }) : super(child);

  double? intrinsicWidth;
  double? intrinsicHeight;

  @override
  double computeMaxIntrinsicWidth(double height) {
    return intrinsicWidth != null
        ? intrinsicWidth!
        : super.computeMaxIntrinsicWidth(height);
  }

  @override
  double computeMaxIntrinsicHeight(double width) {
    return intrinsicHeight != null
        ? intrinsicHeight!
        : super.computeMaxIntrinsicHeight(width);
  }

  @override
  double computeMinIntrinsicWidth(double height) {
    return intrinsicWidth != null
        ? intrinsicWidth!
        : super.computeMinIntrinsicWidth(height);
  }

  @override
  double computeMinIntrinsicHeight(double width) {
    return intrinsicHeight != null
        ? intrinsicHeight!
        : super.computeMinIntrinsicHeight(width);
  }
}
