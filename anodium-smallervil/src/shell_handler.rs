use smithay::{
    reexports::wayland_server::{
        protocol::{wl_pointer::ButtonState, wl_surface::WlSurface},
        DispatchData,
    },
    utils::{Logical, Point},
    wayland::{
        seat::{AxisFrame, PointerGrab, PointerGrabStartData, PointerInnerHandle},
        Serial,
    },
};

use {
    crate::State,
    anodium_framework::shell::{ShellEvent, ShellHandler},
    smithay::desktop,
};

impl ShellHandler for State {
    fn on_shell_event(&mut self, event: anodium_framework::shell::ShellEvent) {
        match event {
            ShellEvent::WindowCreated { window } => {
                self.space.map_window(&window, (0, 0), true);
                window.configure();
            }
            ShellEvent::WindowMove {
                toplevel,
                start_data,
                seat,
                serial,
            } => {
                let pointer = seat.get_pointer().unwrap();

                let window = self
                    .space
                    .window_for_surface(toplevel.get_surface().unwrap())
                    .cloned();

                if let Some(window) = window {
                    let initial_window_location = self.space.window_location(&window).unwrap();

                    let grab = MoveSurfaceGrab {
                        start_data,
                        window,
                        initial_window_location,
                    };
                    pointer.set_grab(grab, serial, 0);
                }
            }
            ShellEvent::SurfaceCommit { surface } => {
                self.space.commit(&surface);
            }
            ShellEvent::WindowResize {
                toplevel,
                start_data,
                seat,
                edges,
                serial,
            } => {
                let pointer = seat.get_pointer().unwrap();
                let wl_surface = toplevel.get_surface().unwrap();

                let window = self.space.window_for_surface(wl_surface);

                if let Some(window) = window {
                    let loc = self.space.window_location(window).unwrap();
                    let geometry = window.geometry();

                    let (initial_window_location, initial_window_size) = (loc, geometry.size);

                    SurfaceData::with_mut(wl_surface, |data| {
                        data.resize_state = ResizeState::Resizing(ResizeData {
                            edges,
                            initial_window_location,
                            initial_window_size,
                        });
                    });

                    let grab = ResizeSurfaceGrab {
                        start_data,
                        window: window.clone(),
                        edges,
                        initial_window_size,
                        last_window_size: initial_window_size,
                    };

                    pointer.set_grab(grab, serial, 0);
                }
            }

            ShellEvent::WindowGotResized {
                window,
                new_location,
            } => {
                self.space.map_window(&window, new_location, false);
            }
            _ => {}
        }
    }

    fn window_location(
        &self,
        window: &desktop::Window,
    ) -> smithay::utils::Point<i32, smithay::utils::Logical> {
        self.space.window_location(window).unwrap_or_default()
    }
}

pub struct MoveSurfaceGrab {
    pub start_data: PointerGrabStartData,

    pub window: desktop::Window,
    pub initial_window_location: Point<i32, Logical>,
}

impl PointerGrab for MoveSurfaceGrab {
    fn motion(
        &mut self,
        handle: &mut PointerInnerHandle<'_>,
        location: Point<f64, Logical>,
        _focus: Option<(WlSurface, Point<i32, Logical>)>,
        serial: Serial,
        time: u32,
        mut ddata: DispatchData,
    ) {
        handle.motion(location, None, serial, time);

        let state = ddata.get::<State>().unwrap();

        let delta = location - self.start_data.location;
        let new_location = self.initial_window_location.to_f64() + delta;

        state
            .space
            .map_window(&self.window, new_location.to_i32_round(), false);
    }

    fn button(
        &mut self,
        handle: &mut PointerInnerHandle<'_>,
        button: u32,
        state: ButtonState,
        serial: Serial,
        time: u32,
        _ddata: DispatchData,
    ) {
        handle.button(button, state, serial, time);
        if handle.current_pressed().is_empty() {
            // No more buttons are pressed, release the grab.
            handle.unset_grab(serial, time);
        }
    }

    fn axis(
        &mut self,
        handle: &mut PointerInnerHandle<'_>,
        details: AxisFrame,
        _ddata: DispatchData,
    ) {
        handle.axis(details)
    }

    fn start_data(&self) -> &PointerGrabStartData {
        &self.start_data
    }
}

use smithay::{
    desktop::Kind,
    reexports::{
        wayland_protocols::xdg_shell::server::xdg_toplevel, wayland_server::protocol::wl_surface,
    },
    utils::Size,
    wayland::{compositor::with_states, shell::xdg::SurfaceCachedState},
};

use anodium_framework::surface_data::{ResizeData, ResizeEdge, ResizeState, SurfaceData};

pub struct ResizeSurfaceGrab {
    pub start_data: PointerGrabStartData,
    pub window: desktop::Window,
    pub edges: ResizeEdge,
    pub initial_window_size: Size<i32, Logical>,
    pub last_window_size: Size<i32, Logical>,
}

impl PointerGrab for ResizeSurfaceGrab {
    fn motion(
        &mut self,
        _handle: &mut PointerInnerHandle<'_>,
        location: Point<f64, Logical>,
        _focus: Option<(wl_surface::WlSurface, Point<i32, Logical>)>,
        _serial: Serial,
        _time: u32,
        _ddata: DispatchData,
    ) {
        let (mut dx, mut dy) = (location - self.start_data.location).into();

        let mut new_window_width = self.initial_window_size.w;
        let mut new_window_height = self.initial_window_size.h;

        let left_right = ResizeEdge::LEFT | ResizeEdge::RIGHT;
        let top_bottom = ResizeEdge::TOP | ResizeEdge::BOTTOM;

        if self.edges.intersects(left_right) {
            if self.edges.intersects(ResizeEdge::LEFT) {
                dx = -dx;
            }

            new_window_width = (self.initial_window_size.w as f64 + dx) as i32;
        }

        if self.edges.intersects(top_bottom) {
            if self.edges.intersects(ResizeEdge::TOP) {
                dy = -dy;
            }

            new_window_height = (self.initial_window_size.h as f64 + dy) as i32;
        }

        let (min_size, max_size) =
            with_states(self.window.toplevel().get_surface().unwrap(), |states| {
                let data = states.cached_state.current::<SurfaceCachedState>();
                (data.min_size, data.max_size)
            })
            .expect("Can't resize surface");

        let min_width = min_size.w.max(1);
        let min_height = min_size.h.max(1);
        let max_width = if max_size.w == 0 {
            i32::max_value()
        } else {
            max_size.w
        };
        let max_height = if max_size.h == 0 {
            i32::max_value()
        } else {
            max_size.h
        };

        new_window_width = new_window_width.max(min_width).min(max_width);
        new_window_height = new_window_height.max(min_height).min(max_height);

        self.last_window_size = (new_window_width, new_window_height).into();

        match self.window.toplevel() {
            Kind::Xdg(xdg) => {
                let ret = xdg.with_pending_state(|state| {
                    state.states.set(xdg_toplevel::State::Resizing);
                    state.size = Some(self.last_window_size);
                });
                if ret.is_ok() {
                    xdg.send_configure();
                }
            }
            #[cfg(feature = "xwayland")]
            WindowSurface::X11(_) => {
                // TODO: What to do here? Send the update via X11?
            }
        }
    }

    fn button(
        &mut self,
        handle: &mut PointerInnerHandle<'_>,
        button: u32,
        state: ButtonState,
        serial: Serial,
        time: u32,
        _ddata: DispatchData,
    ) {
        handle.button(button, state, serial, time);
        if handle.current_pressed().is_empty() {
            // No more buttons are pressed, release the grab.
            handle.unset_grab(serial, time);

            if let Kind::Xdg(xdg) = self.window.toplevel() {
                let ret = xdg.with_pending_state(|state| {
                    state.states.unset(xdg_toplevel::State::Resizing);
                    state.size = Some(self.last_window_size);
                });
                if ret.is_ok() {
                    xdg.send_configure();
                }

                SurfaceData::with_mut(self.window.toplevel().get_surface().unwrap(), |data| {
                    if let ResizeState::Resizing(resize_data) = data.resize_state {
                        data.resize_state = ResizeState::WaitingForFinalAck(resize_data, serial);
                    } else {
                        panic!("invalid resize state: {:?}", data.resize_state);
                    }
                });
            } else {
                SurfaceData::with_mut(self.window.toplevel().get_surface().unwrap(), |data| {
                    if let ResizeState::Resizing(resize_data) = data.resize_state {
                        data.resize_state = ResizeState::WaitingForCommit(resize_data);
                    } else {
                        panic!("invalid resize state: {:?}", data.resize_state);
                    }
                });
            }
        }
    }

    fn axis(
        &mut self,
        handle: &mut PointerInnerHandle<'_>,
        details: AxisFrame,
        _ddata: DispatchData,
    ) {
        handle.axis(details)
    }

    fn start_data(&self) -> &PointerGrabStartData {
        &self.start_data
    }
}