use std::cell::{Cell, RefMut};

use smithay::desktop;
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::wayland::output::Output as SmithayOutput;

use smithay::{
    reexports::wayland_server::{protocol::wl_output, Display},
    utils::{Logical, Point},
    wayland::output::{Mode, PhysicalProperties},
};

#[derive(Default)]
struct Data {
    pending_mode_change: Cell<bool>,
    possible_modes: Cell<Vec<Mode>>,
}

#[derive(Debug, Clone)]
pub struct Output {
    output: SmithayOutput,
}

impl Output {
    pub fn wrap(output: SmithayOutput) -> Self {
        Self { output }
    }

    pub fn from_resource(output: &WlOutput) -> Option<Self> {
        SmithayOutput::from_resource(output).map(|o| Self::wrap(o))
    }
}

impl Output {
    pub fn new<N>(
        name: N,
        location: Point<i32, Logical>,
        display: &mut Display,
        physical: PhysicalProperties,
        transform: wl_output::Transform,
        mode: Mode,
        possible_modes: Vec<Mode>,
    ) -> Self
    where
        N: AsRef<str>,
    {
        let (output, _global) = SmithayOutput::new(
            display,
            name.as_ref().into(),
            physical,
            slog_scope::logger(),
        );

        let scale = 1.0f64;

        output.change_current_state(
            Some(mode),
            Some(transform),
            Some(scale.round() as i32),
            Some(location),
        );
        output.set_preferred(mode);

        let added = output.user_data().insert_if_missing(move || Data {
            pending_mode_change: Default::default(),
            possible_modes: Cell::new(possible_modes),
        });
        assert!(added);

        Self {
            // inner: Rc::new(RefCell::new(Inner {
            //     name: name.as_ref().to_owned(),
            //     global: Some(global),
            //     output,
            //     location,
            //     pending_mode_change: false,
            //     current_mode: mode,
            //     possible_modes,
            //     scale,

            //     active_workspace,
            //     userdata: Default::default(),

            //     layer_map: Default::default(),
            //     wallpaper: None,
            //     wallpaper_texture: None,
            //     imgui: Some((imgui_context.suspend(), imgui_pipeline)),
            //     shell: Shell::new(),
            //     fps: 0.0,
            // })),
            output,
        }
    }

    fn data(&self) -> &Data {
        self.output.user_data().get().unwrap()
    }

    pub fn pending_mode_change(&self) -> bool {
        self.data().pending_mode_change.get()
    }

    pub fn layer_map(&self) -> RefMut<desktop::LayerMap> {
        desktop::layer_map_for_output(&self.output)
    }
}

impl std::ops::Deref for Output {
    type Target = SmithayOutput;

    fn deref(&self) -> &Self::Target {
        &self.output
    }
}

impl std::ops::DerefMut for Output {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.output
    }
}