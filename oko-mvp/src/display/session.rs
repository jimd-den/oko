//! DisplaySession: apply the surface protocol to a compositor and a presenter.
//!
//! This is the in-process half of the display server. It owns one scene, one
//! focused surface and one output. A transport adapter will feed it the same
//! `ClientMessage`s from a socket and carry `InputEvent`s back to the app.

use super::compositor::Compositor;
use super::presenter::{Presenter, PresenterError};
use super::protocol::{
    ClientMessage, DisplayError, InputEvent, ServerMessage, MAX_SURFACE_DIMENSION,
};
use super::surface::{Surface, SurfaceId};

pub struct DisplaySession<P: Presenter> {
    pub compositor: Compositor,
    presenter: P,
    focused: Option<SurfaceId>,
}

impl<P: Presenter> DisplaySession<P> {
    pub fn new(width: u32, height: u32, presenter: P) -> Self {
        Self {
            compositor: Compositor::new(width, height),
            presenter,
            focused: None,
        }
    }

    /// Apply one protocol message. Invalid messages do not mutate the scene.
    pub fn apply(&mut self, message: ClientMessage) -> Result<ServerMessage, DisplayError> {
        match message {
            ClientMessage::Create { id, width, height } => self.create(id, width, height),
            ClientMessage::Commit { id, frame } => self.commit(id, frame),
            ClientMessage::Destroy { id } => self.destroy(id),
        }
    }

    /// Compose and present the current scene.
    pub fn present(&mut self) -> Result<(), PresenterError> {
        let frame = self.compositor.composite();
        self.presenter.present(&frame)
    }

    /// Give focus to an existing surface.
    pub fn focus(&mut self, id: SurfaceId) -> bool {
        if self.compositor.surfaces().iter().any(|s| s.id == id) {
            self.focused = Some(id);
            true
        } else {
            false
        }
    }

    pub fn focused(&self) -> Option<SurfaceId> {
        self.focused
    }

    pub fn presenter(&self) -> &P {
        &self.presenter
    }

    /// Route one input event to the focused surface.
    ///
    /// Contract: returns `None` when nothing has focus. The transport sends the
    /// event only to the returned owner; a surface outside focus never sees it.
    pub fn dispatch_input(&self, event: InputEvent) -> Option<(SurfaceId, InputEvent)> {
        self.focused.map(|id| (id, event))
    }

    fn create(
        &mut self,
        id: SurfaceId,
        width: u32,
        height: u32,
    ) -> Result<ServerMessage, DisplayError> {
        if width == 0 || height == 0 || width > MAX_SURFACE_DIMENSION || height > MAX_SURFACE_DIMENSION
        {
            return Err(DisplayError::TooLarge { width, height });
        }
        if self.compositor.surfaces().iter().any(|s| s.id == id) {
            return Err(DisplayError::SurfaceExists(id));
        }
        let surface = Surface::new(id, width, height);
        self.compositor.add(surface);
        if self.focused.is_none() {
            self.focused = Some(id);
        }
        Ok(ServerMessage::Created { id })
    }

    fn commit(
        &mut self,
        id: SurfaceId,
        frame: super::surface::Frame,
    ) -> Result<ServerMessage, DisplayError> {
        if frame.width == 0 || frame.height == 0 {
            return Err(DisplayError::EmptyFrame);
        }
        let surface = self
            .compositor
            .surfaces()
            .iter()
            .position(|s| s.id == id)
            .ok_or(DisplayError::UnknownSurface(id))?;
        let expected = (
            self.compositor.surfaces()[surface].width,
            self.compositor.surfaces()[surface].height,
        );
        if !self.compositor.surfaces_mut()[surface].replace_frame(&frame) {
            return Err(DisplayError::SizeMismatch {
                expected,
                got: (frame.width, frame.height),
            });
        }
        Ok(ServerMessage::Committed { id })
    }

    fn destroy(&mut self, id: SurfaceId) -> Result<ServerMessage, DisplayError> {
        self.compositor
            .remove(id)
            .ok_or(DisplayError::UnknownSurface(id))?;
        if self.focused == Some(id) {
            self.focused = self
                .compositor
                .surfaces()
                .iter()
                .max_by_key(|s| s.z)
                .map(|s| s.id);
        }
        Ok(ServerMessage::Destroyed { id })
    }
}
