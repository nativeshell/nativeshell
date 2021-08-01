import 'dart:io';
import 'package:flutter/services.dart';
import 'accelerator.dart';

const ctrl = Accelerator(control: true);
const cmd = Accelerator(meta: true);
const alt = Accelerator(alt: true);
final cmdOrCtrl = _cmdOrCtrl();
final shift = Accelerator(shift: true);
final noModifier = Accelerator();

final f1 = _accelerator(LogicalKeyboardKey.f1);
final f2 = _accelerator(LogicalKeyboardKey.f2);
final f3 = _accelerator(LogicalKeyboardKey.f3);
final f4 = _accelerator(LogicalKeyboardKey.f4);
final f5 = _accelerator(LogicalKeyboardKey.f5);
final f6 = _accelerator(LogicalKeyboardKey.f6);
final f7 = _accelerator(LogicalKeyboardKey.f7);
final f8 = _accelerator(LogicalKeyboardKey.f8);
final f9 = _accelerator(LogicalKeyboardKey.f9);
final f10 = _accelerator(LogicalKeyboardKey.f10);
final f11 = _accelerator(LogicalKeyboardKey.f11);
final f12 = _accelerator(LogicalKeyboardKey.f12);
final home = _accelerator(LogicalKeyboardKey.home);
final end = _accelerator(LogicalKeyboardKey.end);
final insert = _accelerator(LogicalKeyboardKey.insert);
final delete = _accelerator(LogicalKeyboardKey.delete);
final backspace = _accelerator(LogicalKeyboardKey.backspace);
final pageUp = _accelerator(LogicalKeyboardKey.pageUp);
final pageDown = _accelerator(LogicalKeyboardKey.pageDown);
final space = _accelerator(LogicalKeyboardKey.space);
final tab = _accelerator(LogicalKeyboardKey.tab);
final enter = _accelerator(LogicalKeyboardKey.enter);
final arrowUp = _accelerator(LogicalKeyboardKey.arrowUp);
final arrowDown = _accelerator(LogicalKeyboardKey.arrowDown);
final arrowLeft = _accelerator(LogicalKeyboardKey.arrowLeft);
final arrowRight = _accelerator(LogicalKeyboardKey.arrowRight);

Accelerator _cmdOrCtrl() {
  return Accelerator(meta: Platform.isMacOS, control: !Platform.isMacOS);
}

Accelerator _accelerator(LogicalKeyboardKey key) => Accelerator(key: key);
