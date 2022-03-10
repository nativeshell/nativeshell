import 'dart:io';

class Shell {
  static final instance = Shell._();

  /// Reveals file on given path in graphical shell. Note that on Linix the file will
  /// not be preselected as no shell supports that.
  void revealPath(String path) async {
    FileSystemEntity file = File(path);
    if (Platform.isWindows) {
      await Process.run('explorer.exe', [
        '/select,${file.absolute.path}',
      ]);
    } else if (Platform.isMacOS) {
      await Process.run('/usr/bin/osascript', [
        '-e',
        'tell application "Finder" to reveal POSIX file "${file.absolute.path}"'
      ]);
      await Process.run('/usr/bin/osascript', [
        '-e',
        'tell application "Finder" to activate',
      ]);
    } else if (Platform.isLinux) {
      if (file.statSync().type != FileSystemEntityType.directory) {
        file = file.parent;
      }
      await Process.run('xdg-open', [file.absolute.path]);
    } else {
      throw UnimplementedError(
          'Target platform does not support `revealPath` function.');
    }
  }

  Shell._();
}
