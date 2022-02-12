/// Proxy object that is tied to a Rust `FinalizableHandle`. When this Dart
/// instance gets garbage collected rust side will be notified of it.
class FinalizableHandle {
  FinalizableHandle(this.id);

  final int id;

  @override
  String toString() => 'FinalizableHandle ($id, #${identityHashCode(this)})';
}
