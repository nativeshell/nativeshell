class FinalizableHandle {
  FinalizableHandle(this.id);

  @override
  bool operator ==(Object other) =>
      identical(this, other) || (other is FinalizableHandle && other.id == id);

  @override
  int get hashCode => id.hashCode;

  final int id;
}
