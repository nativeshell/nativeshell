import 'dart:io';
import 'package:flutter/services.dart';
import 'accelerator.dart';

const ctrl = Accelerator(control: true);
const cmd = Accelerator(meta: true);
const alt = Accelerator(alt: true);
final cmdOrCtrl = _cmdOrCtrl();
final shift = Accelerator(shift: true);
final noModifier = Accelerator();

final f1 = _accelerator(LogicalKeyboardKey.f1, 'F1');
final f2 = _accelerator(LogicalKeyboardKey.f2, 'F2');
final f3 = _accelerator(LogicalKeyboardKey.f3, 'F3');
final f4 = _accelerator(LogicalKeyboardKey.f4, 'F4');
final f5 = _accelerator(LogicalKeyboardKey.f5, 'F5');
final f6 = _accelerator(LogicalKeyboardKey.f6, 'F6');
final f7 = _accelerator(LogicalKeyboardKey.f7, 'F7');
final f8 = _accelerator(LogicalKeyboardKey.f8, 'F8');
final f9 = _accelerator(LogicalKeyboardKey.f9, 'F9');
final f10 = _accelerator(LogicalKeyboardKey.f10, 'F10');
final f11 = _accelerator(LogicalKeyboardKey.f11, 'F11');
final f12 = _accelerator(LogicalKeyboardKey.f12, 'F12');
final home = _accelerator(LogicalKeyboardKey.home, 'Home');
final end = _accelerator(LogicalKeyboardKey.end, 'End');
final insert = _accelerator(LogicalKeyboardKey.insert, 'Insert');
final delete = _accelerator(LogicalKeyboardKey.delete, 'Delete');
final backspace = _accelerator(LogicalKeyboardKey.backspace, 'Backspace');
final pageUp = _accelerator(LogicalKeyboardKey.pageUp, 'Page Up');
final pageDown = _accelerator(LogicalKeyboardKey.pageDown, 'Page Down');
final space = _accelerator(LogicalKeyboardKey.space, 'Space');
final tab = _accelerator(LogicalKeyboardKey.tab, 'Tab');
final enter = _accelerator(LogicalKeyboardKey.enter, 'Enter');
final upArrow = _accelerator(LogicalKeyboardKey.arrowUp, 'Up Arrow');
final downArrow = _accelerator(LogicalKeyboardKey.arrowDown, 'Down Arrow');
final leftArrow = _accelerator(LogicalKeyboardKey.arrowLeft, 'Left Arrow');
final rightArrow = _accelerator(LogicalKeyboardKey.arrowRight, 'Right Arrow');

Accelerator _cmdOrCtrl() {
  return Accelerator(meta: Platform.isMacOS, control: !Platform.isMacOS);
}

Accelerator _accelerator(LogicalKeyboardKey key, String description) =>
    Accelerator(key: AcceleratorKey(key, description));
