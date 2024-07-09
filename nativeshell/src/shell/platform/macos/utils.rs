use crate::shell::{api_model::ImageData, Point, Rect, Size};
use cocoa::{
    appkit::{NSImage, NSScreen, NSView},
    base::{id, nil},
    foundation::{NSArray, NSPoint, NSRect, NSSize, NSString},
};
use core_graphics::{
    base::{kCGBitmapByteOrderDefault, kCGImageAlphaLast, kCGRenderingIntentDefault},
    color_space::CGColorSpace,
    data_provider::CGDataProvider,
    image::CGImage,
};
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    rc::{autoreleasepool, StrongPtr},
    runtime::{objc_getClass, Class, Object},
    sel, sel_impl,
};
use std::{ffi::CString, mem::ManuallyDrop, os::raw::c_char, slice, sync::Arc};

impl<'a> From<&'a Size> for NSSize {
    fn from(size: &'a Size) -> Self {
        NSSize::new(size.width, size.height)
    }
}

impl<'a> From<&'a Point> for NSPoint {
    fn from(position: &'a Point) -> Self {
        NSPoint::new(position.x, position.y)
    }
}

impl From<Size> for NSSize {
    fn from(size: Size) -> Self {
        NSSize::new(size.width, size.height)
    }
}

impl From<Point> for NSPoint {
    fn from(position: Point) -> Self {
        NSPoint::new(position.x, position.y)
    }
}

impl<'a> From<&'a Rect> for NSRect {
    fn from(position: &'a Rect) -> Self {
        NSRect::new(position.origin().into(), position.size().into())
    }
}

impl From<Rect> for NSRect {
    fn from(position: Rect) -> Self {
        NSRect::new(position.origin().into(), position.size().into())
    }
}

impl From<NSSize> for Size {
    fn from(size: NSSize) -> Self {
        Size {
            width: size.width,
            height: size.height,
        }
    }
}

impl From<NSPoint> for Point {
    fn from(point: NSPoint) -> Self {
        Point {
            x: point.x,
            y: point.y,
        }
    }
}

impl From<NSRect> for Rect {
    fn from(rect: NSRect) -> Self {
        Self::xywh(
            rect.origin.x,
            rect.origin.y,
            rect.size.width,
            rect.size.height,
        )
    }
}

pub unsafe fn from_nsstring(ns_string: id) -> String {
    let bytes: *const c_char = msg_send![ns_string, UTF8String];
    let bytes = bytes as *const u8;

    let len = ns_string.len();

    let bytes = slice::from_raw_parts(bytes, len);
    std::str::from_utf8(bytes).unwrap().into()
}

pub fn to_nsstring(string: &str) -> StrongPtr {
    unsafe {
        let ptr = NSString::alloc(nil).init_str(string);
        StrongPtr::new(ptr)
    }
}

// pub fn from_nsdata(data: id) -> Vec<u8> {
//     unsafe {
//         let bytes: *const u8 = msg_send![data, bytes];
//         let length: usize = msg_send![data, length];
//         let data: &[u8] = std::slice::from_raw_parts(bytes, length);
//         data.into()
//     }
// }

pub fn global_screen_frame() -> Rect {
    let mut res = Rect::default();
    autoreleasepool(|| unsafe {
        let screens = NSScreen::screens(nil);
        for i in 0..NSArray::count(screens) {
            let screen = NSArray::objectAtIndex(screens, i);
            let screen_frame: Rect = NSScreen::frame(screen).into();
            res = Rect::union(&res, &screen_frame);
        }
    });
    res
}

pub fn to_nsdata(data: &[u8]) -> StrongPtr {
    unsafe {
        StrongPtr::retain(msg_send![class!(NSData), dataWithBytes:data.as_ptr() length:data.len()])
    }
}

pub unsafe fn superclass(this: &Object) -> &Class {
    let superclass: id = msg_send![this, superclass];
    &*(superclass as *const _)
}

pub unsafe fn array_with_objects(objects: &[StrongPtr]) -> id {
    let vec: Vec<id> = objects.iter().map(|f| *(f.clone()) as id).collect();
    NSArray::arrayWithObjects(nil, &vec)
}

pub fn ns_image_from(images: Vec<ImageData>) -> StrongPtr {
    unsafe {
        let res = StrongPtr::new(msg_send![NSImage::alloc(nil), init]);
        for image in images {
            let data = CGDataProvider::from_buffer(Arc::new(image.data));

            let rgb = CGColorSpace::create_device_rgb();

            let cgimage = CGImage::new(
                image.width as usize,
                image.height as usize,
                8,
                32,
                image.bytes_per_row as usize,
                &rgb,
                kCGBitmapByteOrderDefault | kCGImageAlphaLast,
                &data,
                true,
                kCGRenderingIntentDefault,
            );

            let rep: id = msg_send![class!(NSBitmapImageRep), alloc];
            let rep = StrongPtr::new(msg_send![rep, initWithCGImage:&*cgimage]);
            NSImage::addRepresentation_(*res, *rep);
        }
        res
    }
}

struct MyClassDecl {
    _cls: *mut Class,
}

pub(super) fn class_decl_from_name(name: &str) -> ManuallyDrop<ClassDecl> {
    let name = CString::new(name).unwrap();
    let class = unsafe { objc_getClass(name.as_ptr() as *const _) as *mut _ };
    let res = MyClassDecl { _cls: class };
    // bit dirty, unfortunatelly ClassDecl doesn't let us create instance with custom
    // class, and it's now worth replicating the entire functionality here
    #[allow(clippy::missing_transmute_annotations)]
    ManuallyDrop::new(unsafe { std::mem::transmute(res) })
}

pub(super) fn class_from_string(name: &str) -> *const Class {
    let name = CString::new(name).unwrap();
    unsafe { objc_getClass(name.as_ptr() as *const _) }
}

pub(super) unsafe fn flip_position(view: id, position: &mut NSPoint) {
    let flipped: bool = msg_send![view, isFlipped];
    if !flipped {
        position.y = NSView::bounds(view).size.height - position.y;
    }
}

pub(super) unsafe fn flip_rect(view: id, rect: &mut NSRect) {
    let flipped: bool = msg_send![view, isFlipped];
    if !flipped {
        rect.origin.y = NSView::bounds(view).size.height - rect.size.height - rect.origin.y;
    }
}
