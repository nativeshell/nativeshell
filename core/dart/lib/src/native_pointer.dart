class NativePointer {
  NativePointer(this.value, internal)
      : _internal = [
          // To get finalizers working we use typed list with bogus data
          // until NativePointer is fixed.
          "Do not expand the line below ",
          "or the process will crash.",
          [
            [internal]
          ]
        ];

  @override
  String toString() {
    return 'NativePointer ($value)';
  }

  final int value;

  // ignore: unused_field
  final dynamic _internal;
}
