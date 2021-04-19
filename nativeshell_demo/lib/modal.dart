import 'package:flutter/material.dart';
import 'package:flutter/widgets.dart';
import 'package:nativeshell/nativeshell.dart';

import 'util.dart';

class ModalWindowBuilder extends WindowBuilder {
  @override
  Widget build(BuildContext context) {
    return Container(
      padding: EdgeInsets.all(20),
      color: Colors.blueGrey.shade900,
      child: Column(
        // This is necessasry when using autoSizeWindow, as there are no
        // incoming constraints from the window itself
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
              'This is a Modal Dialog. It is sized to fit.\n\nPick the result:'),
          Container(
            height: 10,
            width: 0,
          ),
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextButton(
                onPressed: () {
                  // Result can be anything serializable with StandardMethodCodec
                  Window.of(context).closeWithResult(true);
                },
                child: Text('Yes'),
              ),
              Container(
                width: 10,
                height: 0,
              ),
              TextButton(
                onPressed: () {
                  Window.of(context).closeWithResult(false);
                },
                child: Text('No'),
              ),
            ],
          ),
          ExtraOptions(),
        ],
      ),
    );
  }

  static ModalWindowBuilder? fromInitData(dynamic initData) {
    if (initData is Map && initData['class'] == 'modalWindow') {
      return ModalWindowBuilder();
    }
    return null;
  }

  static dynamic toInitData() => {
        'class': 'modalWindow',
      };

  @override
  bool get autoSizeWindow => true;

  @override
  Future<void> initializeWindow(
      LocalWindow window, Size intrinsicContentSize) async {
    await window.setStyle(WindowStyle(canResize: false));
    await super.initializeWindow(window, intrinsicContentSize);
  }
}

class ExtraOptions extends StatefulWidget {
  @override
  State<StatefulWidget> createState() {
    return ExtraOptionsState();
  }
}

class ExtraOptionsState extends State<ExtraOptions> {
  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        TextButton(
            onPressed: () {
              setState(() {
                extraOptionsVisible = !extraOptionsVisible;
              });
            },
            child: !extraOptionsVisible
                ? Text('Show more options...')
                : Text('Hide more options')),
        AnimatedVisibility(
            visible: extraOptionsVisible,
            alignment: Alignment.center,
            duration: Duration(milliseconds: 200),
            direction: Axis.vertical,
            child: Padding(
              padding: const EdgeInsets.only(top: 8.0),
              child: TextButton(
                style: TextButton.styleFrom(
                  backgroundColor: Colors.blueAccent.shade700,
                  primary: Colors.white,
                ),
                onPressed: () {
                  Window.of(context).closeWithResult('maybe');
                },
                child: Text('Maybe'),
              ),
            )),
      ],
    );
  }

  bool extraOptionsVisible = false;
}
