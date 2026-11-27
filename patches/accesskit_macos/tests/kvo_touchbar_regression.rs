// Copyright 2026 The AccessKit Authors & SundayDeck Contributors.
// Licensed under the Apache License, Version 2.0 or MIT license.
//
// Regression test for AccessKit #823 / PR #835:
// On macOS Touch Bar hardware (e.g. MacBookPro17,1), AppKit's `_NSTouchBarFinderObservation`
// registers KVO on `nextResponder`, wrapping `AccessKitSubclassOfGPUIView` in
// `NSKVONotifying_AccessKitSubclassOfGPUIView`. Dropping `SubclassingAdapter` must NOT
// overwrite the KVO runtime class, or `removeObserver:forKeyPath:` aborts with SIGTRAP.

use std::{cell::Cell, ffi::c_void, rc::Rc};

use accesskit::{ActionHandler, ActionRequest, ActivationHandler, TreeUpdate};
use accesskit_macos::SubclassingAdapter;
use objc2::{
    ClassType, msg_send, msg_send_id,
    rc::Id,
    runtime::{AnyClass, AnyObject},
};
use objc2_app_kit::NSView;
use objc2_foundation::{NSArray, NSObject, NSPoint, NSString};

#[derive(Default)]
struct HandlerEvents {
    dropped: Cell<bool>,
}

struct NoTree(Rc<HandlerEvents>);

impl ActivationHandler for NoTree {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        None
    }
}

impl Drop for NoTree {
    fn drop(&mut self) {
        self.0.dropped.set(true);
    }
}

struct NoActions;

impl ActionHandler for NoActions {
    fn do_action(&mut self, _request: ActionRequest) {}
}

fn new_view() -> Id<NSView> {
    unsafe { msg_send_id![NSView::class(), new] }
}

fn runtime_class_name(view: &NSView) -> String {
    let object: &AnyObject = view;
    object.class().name().to_owned()
}

fn query_accessibility_after_drop(view: &NSView) {
    let _: *mut NSArray<NSObject> = unsafe { msg_send![view, accessibilityChildren] };
    let _: *mut NSObject = unsafe { msg_send![view, accessibilityFocusedUIElement] };
    let _: *mut NSObject = unsafe { msg_send![view, accessibilityHitTest: NSPoint::new(0., 0.)] };
    let superclass: &AnyClass = unsafe { msg_send![view, superclass] };
    assert_eq!(superclass.name(), "NSResponder");
}

fn main() {
    let view = new_view();
    let events = Rc::new(HandlerEvents::default());
    let adapter = unsafe {
        SubclassingAdapter::new(
            &*view as *const NSView as *mut c_void,
            NoTree(Rc::clone(&events)),
            NoActions,
        )
    };
    assert_eq!(runtime_class_name(&view), "AccessKitSubclassOfNSView");

    // Simulate AppKit's `_NSTouchBarFinderObservation` observing `nextResponder`.
    let observer = NSObject::new();
    let key_path = NSString::from_str("nextResponder");
    let no_context: *mut c_void = std::ptr::null_mut();
    let _: () = unsafe {
        msg_send![
            &*view,
            addObserver: &*observer,
            forKeyPath: &*key_path,
            options: 0usize,
            context: no_context
        ]
    };
    assert_eq!(
        runtime_class_name(&view),
        "NSKVONotifying_AccessKitSubclassOfNSView"
    );

    // Dropping the adapter while KVO is active must preserve the KVO wrapper class.
    drop(adapter);
    assert!(events.dropped.get());
    assert_eq!(
        runtime_class_name(&view),
        "NSKVONotifying_AccessKitSubclassOfNSView"
    );
    query_accessibility_after_drop(&view);

    // AppKit later removes its observer during `NSDisplayCycleFlush`; must not raise NSRangeException.
    let _: () = unsafe {
        msg_send![
            &*view,
            removeObserver: &*observer,
            forKeyPath: &*key_path
        ]
    };
    assert_eq!(runtime_class_name(&view), "AccessKitSubclassOfNSView");
    query_accessibility_after_drop(&view);

    // Without KVO active, dropping restores the original view class cleanly.
    let plain = new_view();
    let plain_adapter = unsafe {
        SubclassingAdapter::new(
            &*plain as *const NSView as *mut c_void,
            NoTree(Rc::new(HandlerEvents::default())),
            NoActions,
        )
    };
    drop(plain_adapter);
    assert_eq!(runtime_class_name(&plain), "NSView");

    println!("OK: kvo_touchbar_regression passed (AccessKit #823 / PR #835 verified on macOS).");
}
