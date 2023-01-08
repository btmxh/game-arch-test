use std::{
    borrow::Cow,
    ffi::{CStr, CString},
    ptr::{null, null_mut},
};

use anyhow::bail;
use gl::types::{GLchar, GLenum, GLuint};

use crate::{
    enclose,
    exec::{dispatch::ReturnMechanism, executor::GameServerExecutor, server::draw},
    graphics::GfxHandle,
};

use super::{GLGfxHandle, GLHandle, GLHandleContainer, GLHandleTrait, SendGLHandleContainer};

pub struct ShaderTrait;
pub enum ShaderType {
    Vertex = gl::VERTEX_SHADER as isize,
    Fragment = gl::FRAGMENT_SHADER as isize,
}
pub type Shader = GLHandle<ShaderTrait, ShaderType>;
pub type ShaderContainer = GLHandleContainer<ShaderTrait, ShaderType>;
pub type ShaderHandle = GfxHandle<Shader>;

impl GLHandleTrait<ShaderType> for ShaderTrait {
    fn create(typ: ShaderType) -> GLuint {
        unsafe { gl::CreateShader(typ as GLenum) }
    }

    fn delete(handle: GLuint) {
        unsafe { gl::DeleteShader(handle) }
    }

    fn identifier() -> GLenum {
        gl::SHADER
    }
}
pub struct ProgramTrait;
pub type Program = GLHandle<ProgramTrait>;
pub type ProgramContainer = GLHandleContainer<ProgramTrait>;
pub type SendProgramContainer = SendGLHandleContainer<ProgramTrait>;
pub type ProgramHandle = GLGfxHandle<ProgramTrait>;

impl GLHandleTrait for ProgramTrait {
    fn create(_: ()) -> GLuint {
        unsafe { gl::CreateProgram() }
    }

    fn delete(handle: GLuint) {
        unsafe { gl::DeleteProgram(handle) }
    }

    fn identifier() -> GLenum {
        gl::PROGRAM
    }

    fn get_container_mut(server: &mut draw::Server) -> Option<&mut GLHandleContainer<Self, ()>> {
        Some(&mut server.handles.programs)
    }

    fn get_container(server: &draw::Server) -> Option<&GLHandleContainer<Self, ()>> {
        Some(&server.handles.programs)
    }
}

impl Shader {
    pub fn new_sourced(
        name: impl Into<Cow<'static, str>>,
        typ: ShaderType,
        source: &str,
    ) -> anyhow::Result<Self> {
        let shader = Self::new_args(name, typ)?;
        unsafe {
            let c_source = CString::new(source)?;
            let ptr = c_source.as_ptr();
            gl::ShaderSource(*shader, 1, &ptr, null());
            gl::CompileShader(*shader);
            let mut status = 0;
            gl::GetShaderiv(*shader, gl::COMPILE_STATUS, &mut status);
            if status == gl::FALSE.into() {
                let mut length = 0;
                gl::GetShaderiv(*shader, gl::INFO_LOG_LENGTH, &mut length);
                let mut buffer = Vec::<u8>::new();
                buffer.resize(length.try_into()?, 0);
                gl::GetShaderInfoLog(
                    *shader,
                    length,
                    null_mut(),
                    buffer.as_mut_ptr() as *mut GLchar,
                );
                let log = CStr::from_bytes_with_nul(buffer.as_slice())
                    .map(|l| l.to_string_lossy())
                    .unwrap_or_else(|_| Cow::Borrowed("unknown error occurred"));
                bail!("unable to compile {}, log: {}", shader.name(), log);
            }
        }
        Ok(shader)
    }
}

impl Program {
    pub fn new_vf(
        name: impl Into<Cow<'static, str>>,
        vertex: &str,
        fragment: &str,
    ) -> anyhow::Result<Self> {
        let program = Self::new(name)?;
        let vertex = Shader::new_sourced(
            format!("{} vertex shader", program.name()),
            ShaderType::Vertex,
            vertex,
        )?;
        let fragment = Shader::new_sourced(
            format!("{} fragment shader", program.name()),
            ShaderType::Fragment,
            fragment,
        )?;

        unsafe {
            gl::AttachShader(*program, *vertex);
            gl::AttachShader(*program, *fragment);
            gl::LinkProgram(*program);
            gl::ValidateProgram(*program);
            let mut status = 0;
            gl::GetProgramiv(*program, gl::LINK_STATUS, &mut status);
            if status == gl::FALSE.into() {
                let mut length = 0;
                gl::GetProgramiv(*program, gl::INFO_LOG_LENGTH, &mut length);
                let mut buffer = Vec::<u8>::new();
                buffer.resize(length.try_into()?, 0);
                gl::GetProgramInfoLog(
                    *program,
                    length,
                    null_mut(),
                    buffer.as_mut_ptr() as *mut GLchar,
                );
                let log = CStr::from_bytes_with_nul(buffer.as_slice())
                    .map(|l| l.to_string_lossy())
                    .unwrap_or_else(|_| Cow::Borrowed("unknown error occurred"));
                bail!("unable to link {}, log: {}", program.name(), log);
            }
            gl::DetachShader(*program, *vertex);
            gl::DetachShader(*program, *fragment);
        }

        Ok(program)
    }
}

impl ProgramHandle {
    #[allow(unused_mut)]
    pub fn new_vf(
        executor: &mut GameServerExecutor,
        draw: &mut draw::ServerChannel,
        name: impl Into<Cow<'static, str>> + Send + 'static,
        ret: Option<ReturnMechanism>,
        vertex: &'static str,
        fragment: &'static str,
    ) -> anyhow::Result<Self> {
        let handle = unsafe { Self::new_uninit(draw) };
        executor.execute_draw(
            draw,
            ret,
            enclose!((handle) move |server| {
                server.handles.create_vf_program(name, &handle, vertex, fragment)?;
                Ok(Box::new(()))
            }),
        )?;
        Ok(handle)
    }
}
