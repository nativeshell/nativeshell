class Channels {
  static final dispatcher = 'nativeshell/window-dispatcher';

  // window sub channels
  static final windowManager = '.window.window-manager';
  static final dropTarget = '.window.drop-target';
  static final dragSource = '.window.drag-source';

  static final menuManager = 'nativeshell/menu-manager';
}

class Events {
  static final windowInitialize = 'event:Window.initialize';
  static final windowVisibilityChanged = 'event:Window.visibilityChanged';
  static final windowCloseRequest = 'event:Window.closeRequest';
  static final windowClose = 'event:Window.close';
}

class Methods {
  // Window
  static final windowCreate = 'Window.create';
  static final windowInit = 'Window.init';
  static final windowShow = 'Window.show';
  static final windowShowModal = 'Window.showModal';
  static final windowReadyToShow = 'Window.readyToShow';
  static final windowHide = 'Window.hide';
  static final windowClose = 'Window.close';
  static final windowCloseWithResult = 'Window.closeWithResult';

  static final windowSetGeometry = 'Window.setGeometry';
  static final windowGetGeometry = 'Window.getGeometry';
  static final windowSupportedGeometry = 'Window.supportedGeometry';

  static final windowSetStyle = 'Window.setStyle';
  static final windowPerformWindowDrag = 'Window.performWindowDrag';

  static final windowShowPopupMenu = 'Window.showPopupMenu';
  static final windowHidePopupMenu = 'Window.hidePopupMenu';
  static final windowShowSystemMenu = 'Window.showSystemMenu';
  static final windowSetWindowMenu = 'Window.setWindowMenu';

  // Drop Target
  static final dropTargetDraggingUpdated = 'DropTarget.draggingUpdated';
  static final dropTargetDraggingExited = 'DropTarget.draggingExited';
  static final dropTargetPerformDrop = 'DropTarget.performDrop';

  // Drop Source
  static final dragSourceBeginDragSession = 'DragSource.beginDragSession';
  static final dragSourceDragSessionEnded = 'DragSource.dragSessionEnded';

  // Menu
  static final menuCreateOrUpdate = 'Menu.createOrUpdate';
  static final menuDestroy = 'Menu.destroy';
  static final menuOnAction = 'Menu.onAction';
  static final menuSetAppMenu = 'Menu.setAppMenu';

  // Menubar
  static final menubarMoveToPreviousMenu = 'Menubar.moveToPreviousMenu';
  static final menubarMoveToNextMenu = 'Menubar.moveToNextMenu';
}

class Keys {
  static final dragDataFiles = 'drag-data:internal:files';
  static final dragDataURLs = 'drag-data:internal:urls';
}
