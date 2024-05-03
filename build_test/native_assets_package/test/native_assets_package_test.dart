import 'package:test/test.dart';

import 'package:native_assets_package/native_assets_package.dart';

void main() {
  test('invoke native function', () {
    // Tests are run in debug mode.
    expect(sum(24, 18), 1042);
  });

  test('invoke async native callback', () async {
    expect(await sumAsync(24, 18), 42);
  });
}
