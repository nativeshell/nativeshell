import 'dart:async';

class Mutex {
  // Serialize execution of citical sections; For uncontended mutex the execution
  // is guaranteed to begin immediately (in this runloop turn)
  Future<T> protect<T>(Future<T> Function() criticalSection) async {
    while (_locked) {
      await _waitUntilUnlocked();
    }
    assert(!_locked);
    _locked = true;
    T res;
    try {
      _locked = true;
      res = await criticalSection();
    } finally {
      _locked = false;
      _postUnlocked();
    }
    return res;
  }

  Future<void> _waitUntilUnlocked() {
    final c = Completer();
    _after.add(c);
    return c.future;
  }

  void _postUnlocked() {
    if (_after.isNotEmpty) {
      final next = _after.removeAt(0);
      next.complete();
    }
  }

  bool _locked = false;
  final _after = <Completer>[];
}
