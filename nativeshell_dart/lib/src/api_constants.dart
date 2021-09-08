class Channels {
  static final dispatcher = 'nativeshell/window-dispatcher';

  // window sub channels
  static final windowManager = '.window.window-manager';
  static final dropTarget = '.window.drop-target';
  static final dragSource = '.window.drag-source';

  static final menuManager = 'nativeshell/menu-manager';
  static final keyboardMapManager = 'nativeshell/keyboard-map-manager';
  static final hotKeyManager = 'nativeshell/hot-key-manager';
}

class Events {
  static final windowInitialize = 'event:Window.initialize';
  static final windowVisibilityChanged = 'event:Window.visibilityChanged';
  static final windowCloseRequest = 'event:Window.closeRequest';
  static final windowClose = 'event:Window.close';
}

const currentApiVersion = 1;

class Methods {
  // WindowManager
  static final windowManagerGetApiVersion = 'WindowManager.getApiVersion';
  static final windowManagerCreateWindow = 'WindowManager.createWindow';
  static final windowManagerInitWindow = 'WindowManager.initWindow';

  // Window
  static final windowShow = 'Window.show';
  static final windowShowModal = 'Window.showModal';
  static final windowReadyToShow = 'Window.readyToShow';
  static final windowHide = 'Window.hide';
  static final windowActivate = 'Window.activate';
  static final windowClose = 'Window.close';
  static final windowCloseWithResult = 'Window.closeWithResult';

  static final windowSetGeometry = 'Window.setGeometry';
  static final windowGetGeometry = 'Window.getGeometry';
  static final windowSupportedGeometry = 'Window.supportedGeometry';

  static final windowSetStyle = 'Window.setStyle';
  static final windowSetTitle = 'Window.setTitle';
  static final windowPerformWindowDrag = 'Window.performWindowDrag';

  static final windowShowPopupMenu = 'Window.showPopupMenu';
  static final windowHidePopupMenu = 'Window.hidePopupMenu';
  static final windowShowSystemMenu = 'Window.showSystemMenu';
  static final windowSetWindowMenu = 'Window.setWindowMenu';

  static final windowSavePositionToString = 'Window.savePositionToString';
  static final windowRestorePositionFromString =
      'Window.restorePositionFromString';

  // Drag Driver
  static final dragDriverDraggingUpdated = 'DragDriver.draggingUpdated';
  static final dragDriverDraggingExited = 'DragDriver.draggingExited';
  static final dragDriverPerformDrop = 'DragDriver.performDrop';

  // Drop Source
  static final dragSourceBeginDragSession = 'DragSource.beginDragSession';
  static final dragSourceDragSessionEnded = 'DragSource.dragSessionEnded';

  // Menu
  static final menuCreateOrUpdate = 'Menu.createOrUpdate';
  static final menuDestroy = 'Menu.destroy';
  static final menuOnAction = 'Menu.onAction';
  static final menuOnOpen = 'Menu.onOpen';
  static final menuSetAppMenu = 'Menu.setAppMenu';

  // Menubar
  static final menubarMoveToPreviousMenu = 'Menubar.moveToPreviousMenu';
  static final menubarMoveToNextMenu = 'Menubar.moveToNextMenu';

  // KeyboardMap
  static final keyboardMapGet = 'KeyboardMap.get';
  static final keyboardMapOnChanged = 'KeyboardMap.onChanged';

  // HotKey
  static final hotKeyCreate = 'HotKey.create';
  static final hotKeyDestroy = 'HotKey.destroy';
  static final hotKeyOnPressed = 'HotKey.onPressed';
}

class Keys {
  static final dragDataFiles = 'drag-data:internal:files';
  static final dragDataURLs = 'drag-data:internal:urls';
}
