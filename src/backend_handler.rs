use crate::State;
use anodium_backend::{BackendHandler, BackendState};

impl BackendHandler for State {
    fn backend_state(&mut self) -> &mut BackendState {
        &mut self.backend
    }

    fn send_frames(&mut self) {
        self.space
            .send_frames(self.start_time.elapsed().as_millis() as u32);
    }

    fn start_compositor(&mut self) {
        let socket_name = self
            .display
            .borrow_mut()
            .add_socket_auto()
            .unwrap()
            .into_string()
            .unwrap();

        ::std::env::set_var("WAYLAND_DISPLAY", &socket_name);
        dbg!(&socket_name);

        #[cfg(feature = "xwayland")]
        {
            self.xwayland.start().ok();
        }
    }

    fn close_compositor(&mut self) {
        self.loop_signal.stop();
    }
}
