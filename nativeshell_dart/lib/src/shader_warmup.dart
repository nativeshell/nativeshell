import 'package:flutter/rendering.dart';

void disableShaderWarmUp() {
  PaintingBinding.shaderWarmUp = _DummyWarmup();
}

class _DummyWarmup extends ShaderWarmUp {
  @override
  Future<void> execute() async {}

  @override
  Future<void> warmUpOnCanvas(Canvas canvas) async {}
}
