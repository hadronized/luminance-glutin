extern crate gl;
extern crate glutin;
extern crate luminance;

use glutin::{Api, Event, EventsLoop, GlProfile, GlRequest, MouseCursor, MouseScrollDelta, Window, WindowEvent, WindowBuilder, get_primary_monitor};
pub use glutin::{CreationError, ElementState, MouseButton, VirtualKeyCode};

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

/// Dimension of the window to create.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowDim {
  Windowed(u32, u32),
  Fullscreen,
  FullscreenRestricted(u32, u32)
}

/// Device object.
///
/// Upon window and context creation, this type is used to add interaction and context handling.
pub struct Device {
  /// Width of the window.
  w: u32,
  /// Height of the window.
  h: u32,
  /// Keyboard receiver.
  pub kbd: Keyboard,
  /// Mouse receiver.
  pub mouse: Mouse,
  /// Cursor receiver.
  pub cursor: MouseMove,
  /// Scroll receiver.
  pub scroll: Scroll,
  /// Window.
  window: Window,
  /// Event thread join handle. Unused and keep around until death.
  #[allow(dead_code)]
  event_thread: JoinHandle<()>,
  #[allow(dead_code)]
  events_loop: EventsLoop
}

impl Device {
  pub fn width(&self) -> u32 {
    self.w
  }

  pub fn height(&self) -> u32 {
    self.h
  }

  pub fn draw<F>(&mut self, f: F) where F: FnOnce() {
    f();
    self.window.swap_buffers();
  }
}

/// Different window options.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowOpt {
  hide_cursor: bool
}

impl Default for WindowOpt {
  fn default() -> Self {
    WindowOpt {
      hide_cursor: false
    }
  }
}

impl WindowOpt {
  /// Hide or unhide the cursor.
  #[inline]
  pub fn hide_cursor(self, hide: bool) -> Self {
    WindowOpt { hide_cursor: hide, ..self }
  }

  #[inline]
  pub fn is_cursor_hidden(&self) -> bool {
    self.hide_cursor
  }
}

/// Create a new window and bootstrap a luminance environment that lives as long as the `Device`
/// lives.
pub fn open_window(dim: WindowDim, title: &str, win_opt: WindowOpt) -> Result<Device, DeviceError> {
  // OpenGL hints
  let gl_version = GlRequest::Specific(Api::OpenGl, (3, 3));
  let gl_profile = GlProfile::Core;

  let events_loop = EventsLoop::new();
  let events_loop_ = &events_loop; // TODO: nope, won’t make it, because we need .clone() instead

  let decorate_window_builder = move |builder: WindowBuilder| {
    builder
      .with_title(title)
      .with_gl(gl_version)
      .with_gl_profile(gl_profile)
      .build_strict(events_loop_)
      .map_err(DeviceError::CreationError)
  };

  // open a window in windowed or fullscreen mode
  let (window, w, h) = match dim {
    WindowDim::Windowed(w, h) => {
      let window = decorate_window_builder(WindowBuilder::new().with_dimensions(w, h))?;
      (window, w, h)
    },
    WindowDim::Fullscreen => {
      let primary_monitor = get_primary_monitor();
      let (w, h) = primary_monitor.get_dimensions();
      let window = decorate_window_builder(WindowBuilder::new().with_fullscreen(primary_monitor))?;
      (window, w, h)
    },
    WindowDim::FullscreenRestricted(w, h) => {
      let primary_monitor = get_primary_monitor();
      let window = decorate_window_builder(WindowBuilder::new()
                                           .with_dimensions(w, h)
                                           .with_fullscreen(primary_monitor))?;
      (window, w, h)
    }
  };

  unsafe { window.make_current() };

  if win_opt.hide_cursor {
    window.set_cursor(MouseCursor::NoneCursor);
  }

  // init OpenGL
  gl::load_with(|s| window.get_proc_address(s) as *const c_void);

  // create channels to stream keyboard and mouse events
  let (kbd_snd, kbd_rcv) = channel();
  let (mouse_snd, mouse_rcv) = channel();
  let (cursor_snd, cursor_rcv) = channel();
  let (scroll_snd, scroll_rcv) = channel();

  let event_thread = spawn(move || {
    events_loop.run_forever(|Event::WindowEvent { event, .. }| {
      match event {
        WindowEvent::KeyboardInput(st, _, Some(key), _) => {
          let _ = kbd_snd.send((key, st));
        },
        WindowEvent::MouseInput(st, button) => {
          let _ = mouse_snd.send((button, st));
        },
        WindowEvent::MouseMoved(x, y) => {
          let _ = cursor_snd.send([x as f32, y as f32]);
        },
        WindowEvent::MouseWheel(MouseScrollDelta::LineDelta(x, y), _) | WindowEvent::MouseWheel(MouseScrollDelta::PixelDelta(x, y), _) => {
          let _ = scroll_snd.send([x, y]);
        },
        _ => ()
      }
    });
  });

  Ok(Device {
    w: w,
    h: h,
    kbd: kbd_rcv,
    mouse: mouse_rcv,
    cursor: cursor_rcv,
    scroll: scroll_rcv,
    window: window,
    event_thread: event_thread,
    events_loop
  })
}
