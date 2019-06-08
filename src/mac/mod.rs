// Copyright 2019 The xi-editor Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! macOS implementation of window creation.
#![allow(non_snake_case)]

pub mod application;
pub mod dialog;
pub mod menu;
pub mod util;
pub mod win_main;

use cocoa::appkit::{
    NSApp, NSApplication, NSApplicationActivateIgnoringOtherApps, NSAutoresizingMaskOptions,
    NSBackingStoreBuffered, NSEvent, NSEventModifierFlags, NSRunningApplication, NSView,
    NSViewHeightSizable, NSViewWidthSizable, NSWindow, NSWindowStyleMask,
};
use cocoa::base::{id, nil, BOOL, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize, NSString};
pub use menu::Menu;
use objc::declare::ClassDecl;
use objc::rc::WeakPtr;
use objc::runtime::{Class, Object, Sel};
use std::any::Any;
use std::ffi::c_void;
use std::ffi::OsString;
use std::mem;
use std::sync::{Arc, Mutex, Weak};

use cairo::{Context, QuartzSurface};

use piet_common::{Piet, RenderContext};

use crate::keyboard::{KeyCode, KeyData, KeyEvent, KeyModifiers};
use crate::platform::dialog::{FileDialogOptions, FileDialogType};
use crate::util::make_nsstring;
use crate::window::{MouseButton, MouseEvent, MouseType, WinHandler};
use crate::Error;

use util::assert_main_thread;

#[derive(Clone, Default)]
pub struct WindowHandle {
    /// This is an NSView, as our concept of "window" is more the top-level container holding
    /// a view. Also, this is better for hosted applications such as VST.
    ///
    /// TODO: remove option (issue has been filed against objc, or we could manually impl default with nil)
    /// https://github.com/SSheldon/rust-objc/issues/77
    nsview: Option<WeakPtr>,
    idle_queue: Weak<Mutex<Vec<Box<dyn IdleCallback>>>>,
}

/// Builder abstraction for creating new windows.
pub struct WindowBuilder {
    handler: Option<Box<dyn WinHandler>>,
    title: String,
    enable_mouse_move_events: bool,
    menu: Option<Menu>,
}

#[derive(Clone)]
pub struct IdleHandle {
    nsview: WeakPtr,
    idle_queue: Weak<Mutex<Vec<Box<dyn IdleCallback>>>>,
}

// TODO: move this out of platform-dependent section.
trait IdleCallback: Send {
    fn call(self: Box<Self>, a: &dyn Any);
}

impl<F: FnOnce(&dyn Any) + Send> IdleCallback for F {
    fn call(self: Box<F>, a: &dyn Any) {
        (*self)(a)
    }
}
/// This is the state associated with our custom NSView.
struct ViewState {
    handler: Box<dyn WinHandler>,
    idle_queue: Arc<Mutex<Vec<Box<dyn IdleCallback>>>>,
}

impl WindowBuilder {
    pub fn new() -> WindowBuilder {
        WindowBuilder {
            handler: None,
            title: String::new(),
            enable_mouse_move_events: true,
            menu: Some(Menu::default()),
        }
    }

    pub fn set_handler(&mut self, handler: Box<dyn WinHandler>) {
        self.handler = Some(handler);
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    pub fn set_menu(&mut self, menu: Menu) {
        self.menu = Some(menu);
        // TODO
    }
    pub fn set_enable_mouse_move_events(&mut self, to: bool) {
        self.enable_mouse_move_events = to;
    }

    pub fn build(self) -> Result<WindowHandle, Error> {
        assert_main_thread();
        unsafe {
            let style_mask = NSWindowStyleMask::NSTitledWindowMask
                | NSWindowStyleMask::NSClosableWindowMask
                | NSWindowStyleMask::NSMiniaturizableWindowMask
                | NSWindowStyleMask::NSResizableWindowMask;
            let rect = NSRect::new(NSPoint::new(0., 0.), NSSize::new(500., 400.));

            let window = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
                rect,
                style_mask,
                NSBackingStoreBuffered,
                NO,
            );

            window.autorelease();
            window.cascadeTopLeftFromPoint_(NSPoint::new(20.0, 20.0));
            window.setTitle_(make_nsstring(&self.title));
            // TODO: this should probably be a tracking area instead
            window.setAcceptsMouseMovedEvents_(YES);

            let (view, idle_queue) = make_view(self.handler.expect("view"));
            let content_view = window.contentView();
            let frame = NSView::frame(content_view);
            view.initWithFrame_(frame);
            match self.menu {
                Some(menu) => NSApp().setMainMenu_(menu.menu),
                _ => (),
            }
            content_view.addSubview_(view);
            let handle = WindowHandle {
                nsview: Some(WeakPtr::new(view)),
                idle_queue,
            };
            let view_state: *mut c_void = *(*view).get_ivar("viewState");
            let view_state = &mut *(view_state as *mut ViewState);
            (*view_state).handler.connect(&crate::window::WindowHandle {
                inner: handle.clone(),
            });
            (*view_state)
                .handler
                .size(frame.size.width as u32, frame.size.height as u32);

            Ok(handle)
        }
    }
}

// Wrap pointer because lazy_static requires Sync.
struct ViewClass(*const Class);
unsafe impl Sync for ViewClass {}

lazy_static! {
    static ref VIEW_CLASS: ViewClass = unsafe {
        let mut decl = ClassDecl::new("DruidView", class!(NSView)).expect("View class defined");
        decl.add_ivar::<*mut c_void>("viewState");

        decl.add_method(
            sel!(isFlipped),
            isFlipped as extern "C" fn(&Object, Sel) -> BOOL,
        );
        extern "C" fn isFlipped(_this: &Object, _sel: Sel) -> BOOL {
            YES
        }
        decl.add_method(
            sel!(acceptsFirstResponder),
            acceptsFirstResponder as extern "C" fn(&Object, Sel) -> BOOL,
        );
        extern "C" fn acceptsFirstResponder(_this: &Object, _sel: Sel) -> BOOL {
            YES
        }
        decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
        extern "C" fn dealloc(this: &Object, _sel: Sel) {
            eprintln!("view is dealloc'ed");
            unsafe {
                let view_state: *mut c_void = *this.get_ivar("viewState");
                Box::from_raw(view_state as *mut ViewState);
            }
        }
        decl.add_method(
            sel!(setFrameSize:),
            set_frame_size as extern "C" fn(&mut Object, Sel, NSSize),
        );
        decl.add_method(
            sel!(mouseDown:),
            mouse_down as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(mouseUp:),
            mouse_up as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(mouseMoved:),
            mouse_move as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(mouseDragged:),
            mouse_move as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(scrollWheel:),
            scroll_wheel as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(keyDown:),
            key_down as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(drawRect:),
            draw_rect as extern "C" fn(&mut Object, Sel, NSRect),
        );
        decl.add_method(sel!(runIdle), run_idle as extern "C" fn(&mut Object, Sel));
        decl.add_method(sel!(redraw), redraw as extern "C" fn(&mut Object, Sel));
        ViewClass(decl.register())
    };
}

fn make_view(handler: Box<dyn WinHandler>) -> (id, Weak<Mutex<Vec<Box<dyn IdleCallback>>>>) {
    let idle_queue = Arc::new(Mutex::new(Vec::new()));
    let queue_handle = Arc::downgrade(&idle_queue);
    let state = ViewState {
        handler,
        idle_queue,
    };
    let state_ptr = Box::into_raw(Box::new(state));
    unsafe {
        let view: id = msg_send![VIEW_CLASS.0, new];
        (*view).set_ivar("viewState", state_ptr as *mut c_void);
        let options: NSAutoresizingMaskOptions = NSViewWidthSizable | NSViewHeightSizable;
        view.setAutoresizingMask_(options);
        (view.autorelease(), queue_handle)
    }
}

extern "C" fn set_frame_size(this: &mut Object, _: Sel, size: NSSize) {
    println!("size: {}x{}", size.width, size.height);
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        (*view_state)
            .handler
            .size(size.width as u32, size.height as u32);
        let superclass = msg_send![this, superclass];
        let () = msg_send![super(this, superclass), setFrameSize: size];
    }
}

fn mouse_event(nsevent: id, view: id, ty: MouseType) -> MouseEvent {
    unsafe {
        let point = nsevent.locationInWindow();
        let view_point = view.convertPoint_fromView_(point, nil);
        MouseEvent {
            x: view_point.x as i32,
            y: view_point.y as i32,
            mods: 0,                  // TODO
            which: MouseButton::Left, // TODO
            ty,
        }
    }
}

extern "C" fn mouse_down(this: &mut Object, _: Sel, nsevent: id) {
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        let event = mouse_event(nsevent, this as id, MouseType::Down);
        (*view_state).handler.mouse(&event);
    }
}

extern "C" fn mouse_up(this: &mut Object, _: Sel, nsevent: id) {
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        let event = mouse_event(nsevent, this as id, MouseType::Up);
        (*view_state).handler.mouse(&event);
    }
}

extern "C" fn mouse_move(this: &mut Object, _: Sel, nsevent: id) {
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        let event = mouse_event(nsevent, this as id, MouseType::Down);
        (*view_state)
            .handler
            .mouse_move(event.x, event.y, event.mods);
    }
}

extern "C" fn scroll_wheel(this: &mut Object, _: Sel, nsevent: id) {
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        let (dx, dy) = {
            let dx = nsevent.scrollingDeltaX() as i32;
            let dy = -nsevent.scrollingDeltaY() as i32;
            if nsevent.hasPreciseScrollingDeltas() == cocoa::base::YES {
                (dx, dy)
            } else {
                (dx * 32, dy * 32)
            }
        };
        let mods = 0; // TODO:
        if dx != 0 {
            (*view_state).handler.mouse_hwheel(dx, mods);
        }

        if dy != 0 {
            (*view_state).handler.mouse_wheel(dy, mods);
        }
    }
}

extern "C" fn key_down(this: &mut Object, _: Sel, nsevent: id) {
    let payload = make_key_event(nsevent);
    let event = if payload.key_code.is_printable() {
        KeyEvent::Character(payload)
    } else {
        KeyEvent::NonCharacter(payload)
    };

    let view_state = unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        &mut *(view_state as *mut ViewState)
    };
    (*view_state).handler.keydown(event);
}

extern "C" fn draw_rect(this: &mut Object, _: Sel, dirtyRect: NSRect) {
    unsafe {
        let context: id = msg_send![class![NSGraphicsContext], currentContext];
        // TODO: probably should use a better type than void pointer, but it's not obvious what's best.
        // cairo_sys::CGContextRef would be better documentation-wise, but it's a type alias.
        let cgcontext: *mut c_void = msg_send![context, CGContext];
        // TODO: use width and height from view size
        let frame = NSView::frame(this as *mut _);
        let width = frame.size.width as u32;
        let height = frame.size.height as u32;
        let cairo_surface =
            QuartzSurface::create_for_cg_context(cgcontext, width, height).expect("cairo surface");
        let mut cairo_ctx = Context::new(&cairo_surface);
        cairo_ctx.set_source_rgb(0.0, 0.5, 0.0);
        cairo_ctx.paint();
        let mut piet_ctx = Piet::new(&mut cairo_ctx);
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        let anim = (*view_state).handler.paint(&mut piet_ctx);
        if let Err(e) = piet_ctx.finish() {
            eprintln!("Error: {}", e);
        }
        // TODO: log errors

        if anim {
            // TODO: synchronize with screen refresh rate using CVDisplayLink instead.
            let () = msg_send!(this as *const _, performSelectorOnMainThread: sel!(redraw)
                withObject: nil waitUntilDone: NO);
        }

        let superclass = msg_send![this, superclass];
        let () = msg_send![super(this, superclass), drawRect: dirtyRect];
    }
}

extern "C" fn run_idle(this: &mut Object, _: Sel) {
    let view_state = unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        &mut *(view_state as *mut ViewState)
    };
    let queue: Vec<_> = mem::replace(
        &mut view_state.idle_queue.lock().expect("queue"),
        Vec::new(),
    );
    let handler_as_any = view_state.handler.as_any();
    for callback in queue {
        callback.call(handler_as_any);
    }
}

extern "C" fn redraw(this: &mut Object, _: Sel) {
    unsafe {
        let () = msg_send![this as *const _, setNeedsDisplay: YES];
    }
}

impl WindowHandle {
    pub fn show(&self) {
        unsafe {
            let current_app = NSRunningApplication::currentApplication(nil);
            current_app.activateWithOptions_(NSApplicationActivateIgnoringOtherApps);
            if let Some(ref nsview) = self.nsview {
                let window: id = msg_send![*nsview.load(), window];
                window.makeKeyAndOrderFront_(nil)
            }
        }
    }

    /// Close the window.
    pub fn close(&self) {
        if let Some(ref nsview) = self.nsview {
            unsafe {
                let window: id = msg_send![*nsview.load(), window];
                window.close();
            }
        }
    }

    // Request invalidation of the entire window contents.
    pub fn invalidate(&self) {
        if let Some(ref nsview) = self.nsview {
            unsafe {
                // We could share impl with redraw, but we'd need to deal with nil.
                let () = msg_send![*nsview.load(), setNeedsDisplay: YES];
            }
        }
    }

    /// Get a handle that can be used to schedule an idle task.
    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        // TODO: maybe try harder to return None if window has been dropped.
        self.nsview.as_ref().map(|nsview| IdleHandle {
            nsview: nsview.clone(),
            idle_queue: self.idle_queue.clone(),
        })
    }

    /// Get the dpi of the window.
    ///
    /// TODO: we want to migrate this from dpi (with 96 as nominal) to a scale
    /// factor (with 1 as nominal).
    pub fn get_dpi(&self) -> f32 {
        // TODO: get actual dpi
        96.0
    }

    // TODO: the following methods are cut'n'paste code. A good way to DRY
    // would be to have a platform-independent trait with these as methods with
    // default implementations.

    /// Convert a dimension in px units to physical pixels (rounding).
    pub fn px_to_pixels(&self, x: f32) -> i32 {
        (x * self.get_dpi() * (1.0 / 96.0)).round() as i32
    }

    /// Convert a point in px units to physical pixels (rounding).
    pub fn px_to_pixels_xy(&self, x: f32, y: f32) -> (i32, i32) {
        let scale = self.get_dpi() * (1.0 / 96.0);
        ((x * scale).round() as i32, (y * scale).round() as i32)
    }

    /// Convert a dimension in physical pixels to px units.
    pub fn pixels_to_px<T: Into<f64>>(&self, x: T) -> f32 {
        (x.into() as f32) * 96.0 / self.get_dpi()
    }

    /// Convert a point in physical pixels to px units.
    pub fn pixels_to_px_xy<T: Into<f64>>(&self, x: T, y: T) -> (f32, f32) {
        let scale = 96.0 / self.get_dpi();
        ((x.into() as f32) * scale, (y.into() as f32) * scale)
    }

    pub fn file_dialog(
        &self,
        _ty: FileDialogType,
        _options: FileDialogOptions,
    ) -> Result<OsString, Error> {
        unimplemented!()
    }
}

unsafe impl Send for IdleHandle {}

impl IdleHandle {
    /// Add an idle handler, which is called (once) when the message loop
    /// is empty. The idle handler will be run from the main UI thread, and
    /// won't be scheduled if the associated view has been dropped.
    ///
    /// Note: the name "idle" suggests that it will be scheduled with a lower
    /// priority than other UI events, but that's not necessarily the case.
    pub fn add_idle<F>(&self, callback: F)
    where
        F: FnOnce(&dyn Any) + Send + 'static,
    {
        if let Some(queue) = self.idle_queue.upgrade() {
            let mut queue = queue.lock().expect("queue lock");
            if queue.is_empty() {
                unsafe {
                    let nsview = self.nsview.load();
                    // Note: the nsview might be nil here if the window has been dropped, but that's ok.
                    let () = msg_send!(*nsview, performSelectorOnMainThread: sel!(runIdle)
                        withObject: nil waitUntilDone: NO);
                }
            }
            queue.push(Box::new(callback));
        }
    }
}

fn make_key_event(event: id) -> KeyData {
    unsafe {
        let chars = event.characters();
        let slice = std::slice::from_raw_parts(chars.UTF8String() as *const _, chars.len());
        let chars = std::str::from_utf8_unchecked(slice);
        let text = crate::keyboard::SmallStr::new(chars);

        let unmodified_chars = event.charactersIgnoringModifiers();
        let slice = std::slice::from_raw_parts(
            unmodified_chars.UTF8String() as *const _,
            unmodified_chars.len(),
        );
        let unmodified_chars = std::str::from_utf8_unchecked(slice);
        let unmodified_text = crate::keyboard::SmallStr::new(unmodified_chars);

        let virtual_key: std::os::raw::c_ushort = msg_send!(event, keyCode);
        let key_code = KeyCode::from_mac_vk_code(virtual_key);

        let is_repeat: bool = msg_send!(event, isARepeat);
        let modifiers = event.modifierFlags();
        let modifiers = make_modifiers(modifiers);

        KeyData {
            key_code,
            is_repeat,
            modifiers,
            text,
            unmodified_text,
        }
    }
}

fn make_modifiers(raw: NSEventModifierFlags) -> KeyModifiers {
    KeyModifiers {
        shift: raw.contains(NSEventModifierFlags::NSShiftKeyMask),
        alt: raw.contains(NSEventModifierFlags::NSAlternateKeyMask),
        ctrl: raw.contains(NSEventModifierFlags::NSControlKeyMask),
        meta: raw.contains(NSEventModifierFlags::NSCommandKeyMask),
    }
}
