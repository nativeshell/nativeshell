import 'package:flutter/services.dart';
import 'package:nativeshell/nativeshell.dart';

final _channel = MethodChannel('file_open_dialog_channel');

class FileOpenRequest {
  FileOpenRequest({
    required this.parentWindow,
  });

  final WindowHandle parentWindow;

  Map serialize() => {
        'parentWindow': parentWindow.value,
      };
}

Future<String?> showFileOpenDialog(FileOpenRequest request) async {
  return await _channel.invokeMethod('showFileOpenDialog', request.serialize());
}
