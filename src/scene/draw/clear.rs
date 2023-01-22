use crate::graphics::context::DrawContext;

pub struct ClearScreen;

impl ClearScreen {
    pub fn draw(&self, _: &mut DrawContext) {
        unsafe {
            gl::ClearColor(0.0, 0.0, 0.0, 0.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
    }
}
