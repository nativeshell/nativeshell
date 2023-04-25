class Channels {
  static final dispatcher = 'nativeshell/window-dispatcher';

  // window sub channels
  static final windowManager = '.window.window-manager';
  static final dropTarget = '.window.drop-target';
  static final dragSource = '.window.drag-source';

  static final menuManager = 'nativeshell/menu-manager';
  static final keyboardMapManager = 'nativeshell/keyboard-map-manager';
  static final hotKeyManager = 'nativeshell/hot-key-manager';
  static final screenManager = 'nativeshell/screen-manager';
  static final statusItemManager = 'nativeshell/status-item-manager';
}

class Events {
  static final windowInitialize = 'event:Window.initialize';
  static final windowVisibilityChanged = 'event:Window.visibilityChanged';
  static final WindowStateFlagsChanged = 'event:Window.stateFlagsChanged';
  static final WindowGeometryChanged = 'event:Window.geometryChanged';
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
  static final windowDeactivate = 'Window.deactivate';
  static final windowClose = 'Window.close';
  static final windowCloseWithResult = 'Window.closeWithResult';

  static final windowSetGeometry = 'Window.setGeometry';
  static final windowGetGeometry = 'Window.getGeometry';
  static final windowSupportedGeometry = 'Window.supportedGeometry';
  static final windowGetScreenId = 'Window.getScreenId';

  static final windowSetStyle = 'Window.setStyle';
  static final windowSetTitle = 'Window.setTitle';
  static final windowSetMinimized = 'Window.setMinimized';
  static final windowSetMaximized = 'Window.setMaximized';
  static final windowSetFullScreen = 'Window.setFullScreen';
  static final windowSetCollectionBehavior = 'Window.setCollectionBehavior';
  static final windowGetWindowStateFlags = 'Window.getWindowStateFlags';
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

  // ScreenManager
  static final screenManagerScreensChanged = 'ScreenManager.screensChanged';
  static final screenManagerGetScreens = 'ScreenManager.getScreens';
  static final screenManagerGetMainScreen = 'ScreenManager.getMainScreen';
  static final screenManagerLogicalToSystem = 'ScreenManager.logicalToSystem';
  static final screenManagerSystemToLogical = 'ScreenManager.systemToLogical';

  // StatusItem
  static final statusItemInit = 'StatusItem.init';
  static final statusItemCreate = 'StatusItem.create';
  static final statusItemDestroy = 'StatusItem.destroy';
  static final statusItemSetImage = 'StatusItem.setImage';
  static final statusItemSetHint = 'StatusItem.setHint';
  static final statusItemShowMenu = 'StatusItem.showMenu';
  static final statusItemSetHighlighted = 'StatusItem.setHighlighted';
  static final statusItemGetGeometry = 'StatusItem.getGeometry';
  static final statusItemGetScreenId = 'StatusItem.getScreenId';
  static final statusItemOnAction = 'StatusItem.onAction';
}

class Keys {
  static final dragDataFiles = 'drag-data:internal:files';
  static final dragDataURLs = 'drag-data:internal:urls';
}
