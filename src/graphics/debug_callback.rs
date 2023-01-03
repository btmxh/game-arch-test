use std::{env::args, ffi::CStr, ptr::null};

use gl::types::{GLenum, GLint, GLuint, GLvoid};

extern "system" fn debug_callback(
    source: GLenum,
    typ: GLenum,
    id: GLuint,
    severity: GLuint,
    _: GLint,
    message: *const i8,
    _: *mut GLvoid,
) {
    let mut level = match severity {
        gl::DEBUG_SEVERITY_LOW => tracing::Level::INFO,
        gl::DEBUG_SEVERITY_NOTIFICATION => tracing::Level::TRACE,
        _ => tracing::Level::WARN,
    };
    let source = match source {
        gl::DEBUG_SOURCE_API => "API",
        gl::DEBUG_SOURCE_APPLICATION => "APPLICATION",
        gl::DEBUG_SOURCE_OTHER => "OTHER",
        gl::DEBUG_SOURCE_SHADER_COMPILER => "SHADER_COMPILER",
        gl::DEBUG_SOURCE_WINDOW_SYSTEM => "WINDOW_SYSTEM",
        gl::DEBUG_SOURCE_THIRD_PARTY => "THIRD_PARTY",
        _ => "UNKNOWN",
    };

    let typ = match typ {
        gl::DEBUG_TYPE_DEPRECATED_BEHAVIOR => "DEPRECATED_BEHAVIOR",
        gl::DEBUG_TYPE_ERROR => {
            level = tracing::Level::ERROR;
            "ERROR"
        }
        gl::DEBUG_TYPE_MARKER => "MARKER",
        gl::DEBUG_TYPE_OTHER => "OTHER",
        gl::DEBUG_TYPE_PERFORMANCE => "PERFORMANCE",
        gl::DEBUG_TYPE_POP_GROUP => "POP_GROUP",
        gl::DEBUG_TYPE_PUSH_GROUP => "PUSH_GROUP",
        gl::DEBUG_TYPE_PORTABILITY => "PORTABILITY",
        gl::DEBUG_TYPE_UNDEFINED_BEHAVIOR => "UNDEFINED_BEHAVIOR",
        _ => "UNKNOWN",
    };
    let message = unsafe { CStr::from_ptr(message).to_string_lossy() };
    match level {
        tracing::Level::TRACE => {
            tracing::trace!(target: "gl", "(source: {}, type: {}, id: {}) {}", source, typ, id, message)
        }
        tracing::Level::INFO => {
            tracing::info!(target: "gl", "(source: {}, type: {}, id: {}) {}", source, typ, id, message)
        }
        tracing::Level::WARN => {
            tracing::warn!(target: "gl", "(source: {}, type: {}, id: {}) {}", source, typ, id, message)
        }
        tracing::Level::ERROR => {
            tracing::error!(target: "gl", "(source: {}, type: {}, id: {}) {}", source, typ, id, message)
        }
        _ => {}
    };
}

pub fn enable_gl_debug_callback() -> bool {
    let no_debug_output = args().any(|s| s == "--no-gl-debug-output");
    unsafe {
        if no_debug_output {
            tracing::info!("OpenGL debug callback was explicitly turned off via command-line argument --no-gl-debug-output");
            false
        } else if gl::DebugMessageCallback::is_loaded() {
            gl::Enable(gl::DEBUG_OUTPUT);
            gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
            gl::DebugMessageCallback(Some(debug_callback), null());
            tracing::info!("OpenGL debug callback enabled");
            true
        } else {
            tracing::info!("OpenGL debug callback not supported");
            false
        }
    }
}
