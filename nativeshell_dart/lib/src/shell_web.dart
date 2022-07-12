class Shell {
  static final instance = Shell._();

  Shell._();

  /// Reveals file on given path in graphical shell. Note that on Linix the file will
  /// not be preselected as no shell supports that.
  void revealPath(String path) async {
    throw UnsupportedError('revealPath is not supported on web');
  }
}
