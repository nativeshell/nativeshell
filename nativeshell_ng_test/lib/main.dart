import 'dart:async';
import 'dart:typed_data';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:nativeshell/nativeshell.dart';

import 'package:nativeshell_core/core.dart' as core;
import 'package:nativeshell_core/core.dart';

void main() async {
  // final c = Calculator();
  // print("${c.addOne(5)}");
  // register();
  // final f = core.NativeFunctions.getDefault();
  // final l = core.NativeList.create<Uint8>(f, 128);

  // final m = core.MessageChannelContext.getDefault();
  // m.sendMessage('Hello');
  // m.sendMessage({'a': 'Hello', 'b': Uint8List(10)});

  // l.asTypedList()[10] = 5;
  // l.resize(2);
  // l.free();

  // {
  //   final channel = core.MessageChannel("abcd");
  //   channel.setHandler((message) async {
  //     await Future.delayed(Duration(seconds: 1));
  //     return message;
  //   });
  //   final res_f = await channel.sendMessage(
  //     "Hello",
  //   );
  //   print("R: $res_f");
  // }

  await Future.delayed(Duration(seconds: 1));
  // exit(1);

  WidgetsFlutterBinding.ensureInitialized();
  // MessageChannelInternal.instance.initialize(isolateId: 10);

  // fn();

  final obj1 = {
    // "aa": "ABCD",
    "i": 30,
    "z": [
      42134,
      2134,
      2142134,
      213421,
      341234,
      123412341234,
      1234123412341234,
      123,
      234124,
      1,
      2,
      3,
      4,
      5,
      6,
      100,
      15,
      30,
      1
    ],
    "x": {
      "a": "A".padLeft(10, 'aaasdfasfasdf'),
      "b": 5,
      // "y": Uint8List(10000),
      // "z": Int32List(10),
      // "zz": Int64List(10),
      // "zz1": Float32List(10),
      // "zzz": Float64List(10),
    }
  };

  // final v = NativeList.create<Int32>(10);
  // v.asTypedList()[1] = 10;
  // v.free();

  // final list = NativeList.create<Uint8>(s1.lengthInBytes);
  // list.asTypedList().setAll(0, s1.buffer.asUint8List(0, s1.lengthInBytes));
  // tv(list.data, list.length, list.capacity);

  // print(">> ${s1.offsetInBytes} ${s1.lengthInBytes}");
  // exit(0);

  // final obj = List.filled(30, obj1);
  // final obj = [
  //   Uint8List(128),
  //   Uint8List(1024),
  //   // Uint8List(1024 * 1024),
  //   // Uint8List(1024 * 64),
  //   "abadfasdf".padLeft(10, 'abcdef')
  // ];

  final cnt = 20000;
  // final cnt = 10;

  final obj = List.filled(50, obj1);
  final sw1 = Stopwatch();
  sw1.start();
  final c1 = NativeMethodChannel("channel1");
  for (int i = 0; i < cnt; ++i) {
    // final res =
    await c1.invokeMethod("m1", obj);
    // print("RES: $res");
  }
  // final res = await c1.invokeMethod("m1", obj);
  // print("res $res");
  print("ELAPSED ${sw1.elapsed}.");
  // print("RES: $res2");

  sw1.reset();
  sw1.start();

  final c2 = MethodChannel("channel1");
  for (int i = 0; i < cnt; ++i) {
    await c2.invokeMethod("m1", obj);
  }
  // final res2 = await c2.invokeMethod("m1", obj);
  ;
  // print("res2: $res2");
  print("ELAPSED ${sw1.elapsed}");

  sw1.reset();
  sw1.start();
  // NativeMethodChannel("channel1");
  for (int i = 0; i < cnt; ++i) {
    await c1.invokeMethod("m1", obj);
  }
  print("ELAPSED ${sw1.elapsed}");

  // final p = NativeApi.initializeApiDLData;
  // final dylib = DynamicLibrary.process();
  // final fn = dylib
  //     .lookup<NativeFunction<init_func>>('hello_from_rust')
  //     .asFunction<init_f>();

  // print('CC ${NativeApi.postCObject}');

  // final port = RawReceivePort((data) {});
  //   print('Received ${fromFFI(data)}  ${data.runtimeType}');
  //   if (data is SendPort) {
  //     print('SendPort ${data.nativePort}');
  //     // data.send(['WTF IS THIS', 10]);
  //     // data.send([1, 2, 3].map((e) => e + 1).toList());
  //     data.send(toFFI({
  //       'name': 'John',
  //       'age': 30,
  //       'items': [4, 5, 6],
  //     }));
  //     Isolate.current
  //         .addOnExitListener(data, response: ['good bye', 13, false]);
  //     // data.send({'WTF IS THIS': 10});
  //   }
  //   received.add(data);
  // }, 'p1');

  // fn(NativeApi.initializeApiDLData, port.sendPort.nativePort);

  // print('Port: ${port.sendPort.nativePort}');
  // port.sendPort.send('Cool');

  // print('Hello2 $p $fn');
  // final MethodChannel c;
  print('WTF');
  // final i = MessageChannelInterface(10);
  final channel = MessageChannel("abcd");
  channel.setHandler((message) async {
    await Future.delayed(Duration(seconds: 1));
    return message;
  });
  // final res_f = channel.sendMessage(
  //   "Hello",
  // );
  // MessageChannelInternal.instance.initialize(isolateId: 10);
  // final res = await res_f;
  // print("RES $res");

  runApp(MyApp());
}

class MyApp extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: DefaultTextStyle(
        style: TextStyle(
          color: Colors.white,
          fontSize: 14,
        ),
        child: Container(
          color: Colors.black,
          child: WindowWidget(
            onCreateState: (initData) {
              WindowState? state;
              state ??= MainWindowState();
              return state;
            },
          ),
        ),
      ),
    );
  }
}

class MainWindowState extends WindowState {
  @override
  WindowSizingMode get windowSizingMode =>
      WindowSizingMode.atLeastIntrinsicSize;

  @override
  Widget build(BuildContext context) {
    return WindowLayoutProbe(
      child: Container(
        padding: EdgeInsets.all(20),
        child: Center(child: Text('Welcome to NativeShell!')),
      ),
    );
  }
}
