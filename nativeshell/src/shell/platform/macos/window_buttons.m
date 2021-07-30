#import <Cocoa/Cocoa.h>

@interface IMContentView : NSView {
  CGPoint lastHitTestScreen;
  CGPoint lastHitTestWindow;
}

@end

@interface IMWindowButtons : NSView

- (void)setEnabled:(BOOL)enabled;
- (void)setOrigin:(NSPoint)origin;

@end

@implementation IMContentView

- (BOOL)isFlipped {
  return YES;
}

// These three methods are necessary to disable default window server titlebar
// dragging when titlebar is hidden.

- (BOOL)acceptsFirstResponder {
  return YES;
}

- (BOOL)becomeFirstResponder {
  for (NSView *view in self.subviews) {
    if (![view isKindOfClass:[IMWindowButtons class]]) {
      return [self.window makeFirstResponder:view];
    }
  }
  return NO;
}

- (BOOL)isOpaque {
  return YES;
}

- (CGPoint)lastHitTestScreen {
  return lastHitTestScreen;
}

- (CGPoint)lastHitTestWindow {
  return lastHitTestWindow;
}

- (NSView *)hitTest:(NSPoint)point {
  NSView *res = [super hitTest:point];

  // hit test window in flipped coordinates (0,0 - top left)
  lastHitTestWindow = [self convertPoint:point toView:nil];

  CGPoint windowFlipped = lastHitTestWindow;
  windowFlipped.y = self.window.frame.size.height - windowFlipped.y;
  CGPoint screen = [self.window convertPointToScreen:windowFlipped];

  // Flip screen to 0,0 - top left
  NSRect screen_frame = self.window.screen.frame;
  screen.y = screen_frame.size.height - screen.y + screen_frame.origin.y;
  lastHitTestScreen = screen;

  return res;
}

- (void)mouseDown:(NSEvent *)event {
  [super mouseDown:event];
}

@end

@interface IMWindowButtons () {
  NSButton *closeButton;
  NSButton *minimizeButton;
  NSButton *zoomButton;
  NSTrackingArea *trackingArea;

  NSView *originalParent;
  NSWindow *originalWindow;
  NSPoint origin;

  BOOL mouseInside;
  BOOL enabled;
}
@end

@implementation IMWindowButtons

- (instancetype)initWithCoder:(NSCoder *)coder {
  if (self = [super initWithCoder:coder]) {
    [self initialize];
  }
  return self;
}

- (instancetype)init {
  if (self = [super init]) {
    [self initialize];
  }
  return self;
}

- (void)initialize {
  closeButton = [NSWindow standardWindowButton:NSWindowCloseButton
                                  forStyleMask:NSWindowStyleMaskTitled];
  [self addSubview:closeButton];

  minimizeButton = [NSWindow standardWindowButton:NSWindowMiniaturizeButton
                                     forStyleMask:NSWindowStyleMaskTitled];
  [self addSubview:minimizeButton];
  NSRect frame = minimizeButton.frame;
  frame.origin.x += 20;
  minimizeButton.frame = frame;

  zoomButton = [NSWindow standardWindowButton:NSWindowZoomButton
                                 forStyleMask:NSWindowStyleMaskTitled];
  [self addSubview:zoomButton];
  frame = zoomButton.frame;
  frame.origin.x += 40;
  zoomButton.frame = frame;

  trackingArea = [[NSTrackingArea alloc]
      initWithRect:NSZeroRect
           options:NSTrackingMouseEnteredAndExited | NSTrackingActiveAlways |
                   NSTrackingInVisibleRect
             owner:self
          userInfo:nil];
  [self addTrackingArea:trackingArea];

  origin = NSMakePoint(6, 6);

  [[NSNotificationCenter defaultCenter]
      addObserver:self
         selector:@selector(update:)
             name:NSWindowDidBecomeKeyNotification
           object:nil];
  [[NSNotificationCenter defaultCenter]
      addObserver:self
         selector:@selector(update:)
             name:NSWindowDidResignKeyNotification
           object:nil];

  [[NSNotificationCenter defaultCenter]
      addObserver:self
         selector:@selector(willEnterFullScreen:)
             name:NSWindowWillEnterFullScreenNotification
           object:nil];
  [[NSNotificationCenter defaultCenter]
      addObserver:self
         selector:@selector(willExitFullScreen:)
             name:NSWindowWillExitFullScreenNotification
           object:nil];
}

- (BOOL)isFlipped {
  return YES;
}

- (void)dealloc {
  [[NSNotificationCenter defaultCenter] removeObserver:self];
}

- (void)update:(id)notification {
  [self updateButtons];
}

- (BOOL)_mouseInGroup:(NSButton *)button {
  return mouseInside;
}

- (void)updateFrame {
  NSRect frame = self.frame;
  frame.origin = origin;
  frame.size = NSMakeSize(54, 16);
  self.frame = frame;
}

- (void)viewDidMoveToWindow {
  [super viewDidMoveToWindow];
  [self updateFrame];

  if (self.superview != nil) {
    originalParent = self.superview;
    originalWindow = self.window;
    if (!self->enabled) {
      [self doDisableButtons];
    }
  }
}

- (void)setEnabled:(BOOL)_enabled {
  if (self->enabled != _enabled) {
    self->enabled = _enabled;
    if (_enabled) {
      [self doEnableButtons];
    } else {
      [self doDisableButtons];
    }
  }
}

- (void)setOrigin:(NSPoint)_origin {
  origin = _origin;
  [self updateFrame];
  [self updateButtons];
}

- (void)doEnableButtons {
  [originalWindow standardWindowButton:NSWindowCloseButton].hidden = YES;
  [originalWindow standardWindowButton:NSWindowMiniaturizeButton].hidden = YES;
  [originalWindow standardWindowButton:NSWindowZoomButton].hidden = YES;
  [originalParent addSubview:self];
  [self updateButtons];
}

- (void)doDisableButtons {
  [self removeFromSuperview];
  mouseInside = NO;
  [originalWindow standardWindowButton:NSWindowCloseButton].hidden = NO;
  [originalWindow standardWindowButton:NSWindowMiniaturizeButton].hidden = NO;
  [originalWindow standardWindowButton:NSWindowZoomButton].hidden = NO;
}

- (void)willEnterFullScreen:(NSNotification *)n {
  if (n.object == originalWindow) {
    [self doDisableButtons];
  }
}

- (void)willExitFullScreen:(NSNotification *)n {
  if (n.object == originalWindow) {
    mouseInside = NO;
    if (enabled) {
      [self doEnableButtons];
    }
  }
}

- (void)updateButtons {
  [closeButton setNeedsDisplay:YES];
  closeButton.enabled =
      (self.window.styleMask & NSWindowStyleMaskClosable) != 0;

  [minimizeButton setNeedsDisplay:YES];
  minimizeButton.enabled =
      (self.window.styleMask & NSWindowStyleMaskMiniaturizable) != 0;

  [zoomButton setNeedsDisplay:YES];
  zoomButton.enabled =
      (self.window.styleMask & NSWindowStyleMaskResizable) != 0;
}

- (void)mouseEntered:(NSEvent *)event {
  mouseInside = YES;
  [self updateButtons];
}

- (void)mouseExited:(NSEvent *)event {
  mouseInside = NO;
  [self updateButtons];
}

@end

// Make sure rust actually links with this library
void im_link_objc_dummy_method() {}
