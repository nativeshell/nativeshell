import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

class AnimatedVisibility extends ImplicitlyAnimatedWidget {
  AnimatedVisibility({
    Key? key,
    required this.visible,
    required Duration duration,
    required this.child,
    required this.direction,
    this.alignment = Alignment.topLeft,
  }) : super(
          key: key,
          duration: duration,
          curve: Curves.linear,
        );

  @override
  ImplicitlyAnimatedWidgetState createState() => _AnimatedVisibilityState();

  final bool visible;
  final Widget child;
  final Axis direction;
  final Alignment alignment;
}

class _AnimatedVisibilityState
    extends AnimatedWidgetBaseState<AnimatedVisibility> {
  Tween<dynamic>? _factor;

  @override
  void initState() {
    super.initState();
  }

  @override
  void forEachTween(TweenVisitor<dynamic> visitor) {
    var factor = widget.visible ? 1.0 : 0.0;

    _factor = visitor(
        _factor, factor, (dynamic value) => Tween<double>(begin: value));
  }

  @override
  Widget build(BuildContext context) {
    double factor = _factor!.evaluate(animation);

    if (factor == 0) {
      return Container(
        width: widget.direction == Axis.horizontal ? 0.0 : null,
        height: widget.direction == Axis.vertical ? 0.0 : null,
      );
    }

    Widget child = BetterAlign(
      alignment: widget.alignment,
      widthFactor: widget.direction == Axis.horizontal ? factor : null,
      heightFactor: widget.direction == Axis.vertical ? factor : null,
      child: widget.child,
    );
    if (factor < 1.0) {
      child = Opacity(
        opacity: factor,
        child: ClipRect(child: child),
      );
    }
    return child;
  }
}

class BetterAlign extends Align {
  const BetterAlign({
    Key? key,
    Alignment alignment = Alignment.center,
    double? widthFactor,
    double? heightFactor,
    Widget? child,
  }) : super(
            key: key,
            alignment: alignment,
            widthFactor: widthFactor,
            heightFactor: heightFactor,
            child: child);

  @override
  _RenderAlign createRenderObject(BuildContext context) {
    return _RenderAlign(
      alignment: alignment,
      widthFactor: widthFactor,
      heightFactor: heightFactor,
      textDirection: Directionality.of(context),
    );
  }
}

class _RenderAlign extends RenderPositionedBox {
  _RenderAlign({
    RenderBox? child,
    double? widthFactor,
    double? heightFactor,
    AlignmentGeometry alignment = Alignment.center,
    TextDirection? textDirection,
  }) : super(
          child: child,
          widthFactor: widthFactor,
          heightFactor: heightFactor,
          alignment: alignment,
          textDirection: textDirection,
        );

  @override
  double computeMinIntrinsicHeight(double width) => heightFactor == 0.0
      ? 0.0
      : (heightFactor ?? 1.0) * super.computeMinIntrinsicHeight(width);

  @override
  double computeMaxIntrinsicHeight(double width) => heightFactor == 0.0
      ? 0.0
      : (heightFactor ?? 1.0) * super.computeMaxIntrinsicHeight(width);

  @override
  double computeMinIntrinsicWidth(double height) => widthFactor == 0.0
      ? 0.0
      : (widthFactor ?? 1.0) * super.computeMinIntrinsicWidth(height);

  @override
  double computeMaxIntrinsicWidth(double height) => widthFactor == 0.0
      ? 0.0
      : (widthFactor ?? 1.0) * super.computeMaxIntrinsicWidth(height);

  @override
  void performLayout() {
    final shrinkWrapWidth =
        widthFactor != null || constraints.maxWidth == double.infinity;
    final shrinkWrapHeight =
        heightFactor != null || constraints.maxHeight == double.infinity;

    if (child != null) {
      child!.layout(
          constraints.copyWith(
            // only loosen constraints when shrink wrapping
            minWidth: shrinkWrapWidth ? 0.0 : null,
            minHeight: shrinkWrapHeight ? 0.0 : null,
          ),
          parentUsesSize: true);
      size = constraints.constrain(Size(
          shrinkWrapWidth
              ? child!.size.width * (widthFactor ?? 1.0)
              : double.infinity,
          shrinkWrapHeight
              ? child!.size.height * (heightFactor ?? 1.0)
              : double.infinity));
      alignChild();
    } else {
      size = constraints.constrain(Size(shrinkWrapWidth ? 0.0 : double.infinity,
          shrinkWrapHeight ? 0.0 : double.infinity));
    }
  }
}
