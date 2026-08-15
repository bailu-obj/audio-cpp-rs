//! Loaded model handle.

use std::marker::PhantomData;
use std::ptr;

use audio_cpp_sys::{
    audio_cpp_model, audio_cpp_model_capabilities, audio_cpp_model_create_session,
    audio_cpp_model_destroy, audio_cpp_model_get_metadata,
};

use crate::convert::{
    check, encode_task_spec, take_capabilities, take_metadata, EncodedSessionOptions,
};
use crate::error::Result;
use crate::session::Session;
use crate::types::{CapabilitySet, ModelMetadata, SessionOptions, TaskSpec};

/// Loaded voice model. Sessions created from this model must not outlive it.
#[derive(Debug)]
pub struct Model {
    ptr: *mut audio_cpp_model,
}

unsafe impl Send for Model {}

impl Model {
    pub(crate) unsafe fn from_ptr(ptr: *mut audio_cpp_model) -> Self {
        Self { ptr }
    }

    /// Model identity metadata.
    pub fn metadata(&self) -> Result<ModelMetadata> {
        let mut raw = audio_cpp_sys::audio_cpp_model_metadata::default();
        check(unsafe { audio_cpp_model_get_metadata(self.ptr, &mut raw) })?;
        take_metadata(raw)
    }

    /// Advertised task capabilities.
    pub fn capabilities(&self) -> Result<CapabilitySet> {
        let mut raw = audio_cpp_sys::audio_cpp_capability_set::default();
        check(unsafe { audio_cpp_model_capabilities(self.ptr, &mut raw) })?;
        take_capabilities(raw)
    }

    /// Create a task session bound to this model's lifetime.
    pub fn create_session(&self, spec: TaskSpec, options: &SessionOptions) -> Result<Session<'_>> {
        let encoded = EncodedSessionOptions::new(options)?;
        let mut session = ptr::null_mut();
        check(unsafe {
            audio_cpp_model_create_session(
                self.ptr,
                encode_task_spec(spec),
                encoded.options,
                &mut session,
            )
        })?;
        Ok(unsafe { Session::from_ptr(session, PhantomData) })
    }
}

impl Drop for Model {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { audio_cpp_model_destroy(self.ptr) };
            self.ptr = ptr::null_mut();
        }
    }
}
