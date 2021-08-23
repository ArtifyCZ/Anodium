use crate::{backend::winit::WinitData, state::BackendState};

use smithay::{
    backend::{
        input::{InputBackend, InputEvent},
        winit::WinitEvent,
    },
    wayland::output::Mode,
};

impl BackendState<WinitData> {
    pub fn process_input_event<B>(&mut self, event: InputEvent<B>)
    where
        B: InputBackend<SpecialEvent = WinitEvent>,
    {
        match event {
            InputEvent::Special(WinitEvent::Resized { size, .. }) => {
                self.anodium
                    .desktop_layout
                    .borrow_mut()
                    .update_output_mode_by_name(
                        Mode {
                            size,
                            refresh: 60_000,
                        },
                        crate::backend::winit::OUTPUT_NAME,
                    );
            }
            event => {
                self.anodium.process_input_event(&mut self.backend_data, event);
            }
        }
    }
}