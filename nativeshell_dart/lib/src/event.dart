import 'dart:ui';

typedef Listener<T> = void Function(T t);

class Event<T> {
  void addListener(Listener<T> listener) {
    _listeners.add(listener);
  }

  void removeListener(Listener<T> listener) {
    _listeners.remove(listener);
  }

  void fire(T arg) {
    final copy = List<Listener<T>>.from(_listeners);
    for (final l in copy) {
      if (_listeners.contains(l)) {
        l(arg);
      }
    }
  }

  void dispose() {
    _listeners.clear();
  }

  final _listeners = List<Listener<T>>.empty(growable: true);
}

class VoidEvent {
  void addListener(VoidCallback listener) {
    _listeners.add(listener);
  }

  void removeListener(VoidCallback listener) {
    _listeners.remove(listener);
  }

  void fire() {
    final copy = List<VoidCallback>.from(_listeners);
    for (final l in copy) {
      if (_listeners.contains(l)) {
        l();
      }
    }
  }

  void dispose() {
    _listeners.clear();
  }

  final _listeners = List<VoidCallback>.empty(growable: true);
}
