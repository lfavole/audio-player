//! Functions that can create an invisible window **on Windows only***
//! and return its HWND.
use std::os::raw::c_void;
use std::sync::mpsc::SyncSender;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::platform::windows::EventLoopBuilderExtWindows;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{Window, WindowId};

use crate::song::EBox;

/// A `*mut c_void` that can be sent between threads.
#[expect(
    clippy::exhaustive_structs,
    reason = "this struct only contains the pointer"
)]
pub struct PointerWrapper(pub *mut c_void);

/// SAFETY: we *really* need this!
unsafe impl Send for PointerWrapper {}

/// An app with an invisible window.
struct App {
    /// The window associated with this app.
    pub window: Option<Window>,
    /// The sender that will send the HWND.
    pub tx: SyncSender<Result<PointerWrapper, EBox>>,
}

impl App {
    #[expect(clippy::unwrap_in_result, reason = "the window has been filled")]
    fn get_window_handle(&self) -> Result<PointerWrapper, EBox> {
        let handle = self
            .window
            .as_ref()
            .expect("the window should have been filled")
            .window_handle()?
            .as_raw();
        if let RawWindowHandle::Win32(handle) = handle {
            return Ok(PointerWrapper(handle.hwnd.get() as *mut c_void));
        }
        unreachable!();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.tx
            .send(
                match event_loop.create_window(Window::default_attributes().with_visible(false)) {
                    Ok(window) => {
                        self.window = Some(window);
                        self.get_window_handle()
                    }
                    Err(error) => Err(Box::new(error)),
                },
            )
            .expect("failed sending the window handle");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if event == WindowEvent::CloseRequested {
            event_loop.exit();
        }
    }
}

/// Create an invisible window and run it.
///
/// Inspired from <https://docs.rs/winit/#event-handling>.
///
/// # Errors
/// Fails if there is some problem with the operating system during the window creation.
pub fn run_window(tx: SyncSender<Result<PointerWrapper, EBox>>) -> Result<(), EBox> {
    let event_loop = EventLoop::builder().with_any_thread(true).build()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App { window: None, tx };
    event_loop.run_app(&mut app)?;
    Ok(())
}
