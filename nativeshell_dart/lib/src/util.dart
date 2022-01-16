import 'dart:async';

import 'package:flutter/painting.dart';

String enumToString<T>(T enumItem) {
  return enumItem.toString().split('.')[1];
}

T enumFromString<T>(List<T> enumValues, String value, [T? defaultValue]) {
  // ignore: unnecessary_cast
  return enumValues.singleWhere(
      (enumItem) => enumToString(enumItem).toLowerCase() == value.toLowerCase(),
      orElse: () => defaultValue!);
}

Future<ImageInfo?> loadImage(
  AssetImage image,
  ImageConfiguration configuration,
) {
  final stream = image.resolve(configuration);
  final completer = Completer<ImageInfo?>();
  stream.addListener(ImageStreamListener((image, synchronousCall) {
    completer.complete(image);
  }, onError: (_, __) {
    completer.complete(null);
  }));
  return completer.future;
}

Future<List<ImageInfo>> loadAllImages(AssetImage image) async {
  final keys = <AssetBundleImageKey>{};
  keys.add(await image.obtainKey(ImageConfiguration(devicePixelRatio: 1.0)));
  keys.add(await image.obtainKey(ImageConfiguration(devicePixelRatio: 1.25)));
  keys.add(await image.obtainKey(ImageConfiguration(devicePixelRatio: 1.5)));
  keys.add(await image.obtainKey(ImageConfiguration(devicePixelRatio: 1.75)));
  keys.add(await image.obtainKey(ImageConfiguration(devicePixelRatio: 2.0)));
  keys.add(await image.obtainKey(ImageConfiguration(devicePixelRatio: 2.5)));
  keys.add(await image.obtainKey(ImageConfiguration(devicePixelRatio: 3.0)));

  final res = <ImageInfo>[];
  for (final k in keys) {
    final i =
        await loadImage(image, ImageConfiguration(devicePixelRatio: k.scale));
    if (i != null) {
      res.add(i);
    }
  }
  return res;
}
