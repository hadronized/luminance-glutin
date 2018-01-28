extern crate gl;
extern crate glutin;
extern crate luminance;
extern crate luminance_windowing;

use glutin::GlContext as GlContextTrait;
pub use glutin::{CreationError, ElementState, Event, MouseButton, VirtualKeyCode};
pub use luminance_windowing::{Device, WindowDim, WindowOpt};

use std::os::raw::c_void;
use std::sync::mpsc::{Receiver, channel};
use std::thread::{JoinHandle, spawn};

pub type Key = VirtualKeyCode;
pub type Action = ElementState;
pub type Keyboard = Receiver<(VirtualKeyCode, ElementState)>;
pub type Mouse = Receiver<(MouseButton, ElementState)>;
pub type MouseMove = Receiver<[f32; 2]>;
pub type Scroll = Receiver<[f32; 2]>;

/// Error that can be risen while creating a `Device` object.
#[derive(Debug)]
pub enum DeviceError {
  CreationError(CreationError)
}

/// Device object.
///
/// Upon window and context creation, this type is used to add interaction and context handling.
pub struct GlutinDevice {
  /// Event receiver.
  events_rx: Receiver<Event>,
  /// Window.
  window: glutin::GlWindow,
  /// Event thread join handle. Unused and keep around until death.
  #[allow(dead_code)]
  event_thread: JoinHandle<()>,
}

impl Device for GlutinDevice {
  type Event = Event;

  type Error = DeviceError;

  fn new(
    dim: WindowDim, 
    title: &str, 
    win_opt: WindowOpt
  ) -> Result<Self, Self::Error> {
    // OpenGL hints
    let gl_version = glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 3));
    let gl_profile = glutin::GlProfile::Core;

    let events_loop = glutin::EventsLoop::new();

    // create the OpenGL window by creating a window, a context and attaching it to the window
    let window =
      glutin::WindowBuilder::new()
        .with_title(title);

    let window =
      match dim {
        WindowDim::Windowed(w, h) => window.with_dimensions(w, h),
        WindowDim::Fullscreen => window.with_fullscreen(None),
        WindowDim::FullscreenRestricted(w, h) => window.with_dimensions(w, h).with_fullscreen(None)
      };


    let ctx = 
      glutin::ContextBuilder::new()
        .with_gl(gl_version)
        .with_gl_profile(gl_profile);

    let gl_window =
      glutin::GlWindow::new(window, ctx, &events_loop).map_err(DeviceError::CreationError)?;

    if win_opt.is_cursor_hidden() {
      gl_window.set_cursor(glutin::MouseCursor::NoneCursor);
    } else {
      gl_window.set_cursor(glutin::MouseCursor::Default);
    }

    unsafe { gl_window.make_current().unwrap() };
     gl::load_with(|s| gl_window.get_proc_address(s) as *const c_void);

    // place the event loop in a thread; every time an event is polled from glutin,
    // enqueue it in a channel so that we can get it back in the device
    let (events_sx, events_rx) = channel();
    let event_thread = spawn(move || {
      events_loop.run_forever(|event| {
        events_sx.send(event);

        if let Event::WindowEvent { event: glutin::WindowEvent::Closed, .. } = event {
          glutin::ControlFlow::Break
        } else {
          glutin::ControlFlow::Continue
        }
      });
    });

    let device =
      GlutinDevice {
        events_rx,
        window: gl_window,
        event_thread
      };

    Ok(device)
  }

  fn size(&self) -> [u32; 2] {
    let (w, h) = self.window.get_inner_size().unwrap_or((0, 0));

    [w, h]
  }

  fn events<'a>(&'a mut self) -> Box<Iterator<Item = Self::Event> + 'a> {
    Box::new(self.events_rx.try_iter())
  }

  fn draw<F>(&mut self, f: F) where F: FnOnce() {
    f();
    self.window.swap_buffers();
  }
}
