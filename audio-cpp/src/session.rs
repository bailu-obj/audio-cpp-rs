//! Offline and streaming task sessions.

use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ptr;

use audio_cpp_sys::{
    audio_cpp_build_prep_from_request, audio_cpp_session, audio_cpp_session_destroy,
    audio_cpp_session_family, audio_cpp_session_finish_stream, audio_cpp_session_next_stream_event,
    audio_cpp_session_prepare, audio_cpp_session_process_audio_chunk, audio_cpp_session_reset,
    audio_cpp_session_run, audio_cpp_session_run_mode, audio_cpp_session_set_stream_callback,
    audio_cpp_session_start_stream, audio_cpp_session_streaming_policy,
    audio_cpp_session_supports_offline, audio_cpp_session_supports_streaming,
    audio_cpp_session_task_kind, audio_cpp_stream_event, audio_cpp_task_result,
};

use crate::convert::{
    check, copy_c_str, encode_audio_chunk, stream_event_trampoline, streaming_policy_from_raw,
    take_stream_event, take_task_result, CallbackState, EncodedPrepRequest, EncodedTaskRequest,
};
use crate::error::{Error, Result};
use crate::model::Model;
use crate::types::{
    AudioChunk, RunMode, SessionPreparationRequest, StreamEvent, StreamingPolicy, TaskRequest,
    TaskResult, VoiceTaskKind,
};

/// A task session created from a [`Model`].
///
/// Sessions are neither [`Send`] nor [`Sync`]. Prepare the session before
/// [`Session::run`] or streaming calls.
pub struct Session<'model> {
    ptr: *mut audio_cpp_session,
    callback: Option<Box<CallbackState>>,
    _model: PhantomData<&'model Model>,
}

impl<'model> Session<'model> {
    pub(crate) unsafe fn from_ptr(
        ptr: *mut audio_cpp_session,
        model: PhantomData<&'model Model>,
    ) -> Self {
        Self {
            ptr,
            callback: None,
            _model: model,
        }
    }

    /// Model family that created this session.
    pub fn family(&self) -> Result<String> {
        let mut family = ptr::null();
        check(unsafe { audio_cpp_session_family(self.ptr, &mut family) })?;
        copy_c_str(family)
    }

    /// Task kind selected at creation time.
    pub fn task_kind(&self) -> Result<VoiceTaskKind> {
        let mut kind = unsafe { std::mem::zeroed() };
        check(unsafe { audio_cpp_session_task_kind(self.ptr, &mut kind) })?;
        VoiceTaskKind::from_raw(kind)
    }

    /// Run mode selected at creation time.
    pub fn run_mode(&self) -> Result<RunMode> {
        let mut mode = unsafe { std::mem::zeroed() };
        check(unsafe { audio_cpp_session_run_mode(self.ptr, &mut mode) })?;
        RunMode::from_raw(mode)
    }

    /// Whether [`Session::run`] is available.
    #[must_use]
    pub fn supports_offline(&self) -> bool {
        unsafe { audio_cpp_session_supports_offline(self.ptr) != 0 }
    }

    /// Whether streaming methods are available.
    #[must_use]
    pub fn supports_streaming(&self) -> bool {
        unsafe { audio_cpp_session_supports_streaming(self.ptr) != 0 }
    }

    /// Convert into a typed offline session.
    pub fn into_offline(self) -> Result<OfflineSession<'model>> {
        if !self.supports_offline() {
            return Err(Error::unsupported("session does not support offline runs"));
        }
        Ok(OfflineSession { inner: self })
    }

    /// Convert into a typed streaming session.
    pub fn into_streaming(self) -> Result<StreamingSession<'model>> {
        if !self.supports_streaming() {
            return Err(Error::unsupported(
                "session does not support streaming runs",
            ));
        }
        Ok(StreamingSession { inner: self })
    }

    /// Prepare graphs and caches for a later run.
    pub fn prepare(&mut self, request: &SessionPreparationRequest) -> Result<()> {
        let encoded = EncodedPrepRequest::new(request)?;
        check(unsafe { audio_cpp_session_prepare(self.ptr, &encoded.request) })
    }

    /// Build a preparation request from `request` and prepare the session.
    pub fn prepare_from_request(&mut self, request: &TaskRequest) -> Result<()> {
        let encoded = EncodedTaskRequest::new(request)?;
        let mut prep = audio_cpp_sys::audio_cpp_session_prep_request::default();
        check(unsafe { audio_cpp_build_prep_from_request(&encoded.request, &mut prep) })?;
        check(unsafe { audio_cpp_session_prepare(self.ptr, &prep) })
    }

    /// Run an offline task. The session must already be prepared.
    pub fn run(&mut self, request: &TaskRequest) -> Result<TaskResult> {
        if !self.supports_offline() {
            return Err(Error::unsupported("session does not support offline runs"));
        }
        let encoded = EncodedTaskRequest::new(request)?;
        let mut raw = audio_cpp_task_result::default();
        check(unsafe { audio_cpp_session_run(self.ptr, &encoded.request, &mut raw) })?;
        take_task_result(raw)
    }

    /// Prepare from `request` and then run it.
    pub fn prepare_and_run(&mut self, request: &TaskRequest) -> Result<TaskResult> {
        self.prepare_from_request(request)?;
        self.run(request)
    }

    /// Streaming policy advertised by the session.
    pub fn streaming_policy(&self) -> Result<StreamingPolicy> {
        if !self.supports_streaming() {
            return Err(Error::unsupported(
                "session does not support streaming runs",
            ));
        }
        let mut policy = audio_cpp_sys::audio_cpp_streaming_policy::default();
        check(unsafe { audio_cpp_session_streaming_policy(self.ptr, &mut policy) })?;
        Ok(streaming_policy_from_raw(policy))
    }

    /// Install a stream-event callback for the duration of `f`.
    ///
    /// The callback receives an owned [`StreamEvent`] copied from the native
    /// event, so it must not retain pointers into the C ABI.
    pub fn with_event_callback<R>(
        &mut self,
        callback: impl FnMut(&StreamEvent) + 'static,
        f: impl FnOnce(&mut Self) -> Result<R>,
    ) -> Result<R> {
        self.install_callback(Box::new(callback))?;
        let result = f(self);
        self.clear_callback()?;
        result
    }

    /// Start a streaming run. The session must already be prepared.
    pub fn start_stream(&mut self, request: &TaskRequest) -> Result<()> {
        self.require_streaming()?;
        let encoded = EncodedTaskRequest::new(request)?;
        check(unsafe { audio_cpp_session_start_stream(self.ptr, &encoded.request) })
    }

    /// Feed one audio chunk and return the immediate event.
    pub fn process_audio_chunk(&mut self, chunk: &AudioChunk) -> Result<StreamEvent> {
        self.require_streaming()?;
        let encoded = encode_audio_chunk(chunk);
        let mut event = audio_cpp_stream_event::default();
        check(unsafe { audio_cpp_session_process_audio_chunk(self.ptr, &encoded, &mut event) })?;
        take_stream_event(event)
    }

    /// Pull the next buffered stream event, if any.
    pub fn next_stream_event(&mut self) -> Result<Option<StreamEvent>> {
        self.require_streaming()?;
        let mut event = audio_cpp_stream_event::default();
        let mut has_event = 0u8;
        check(unsafe {
            audio_cpp_session_next_stream_event(self.ptr, &mut event, &mut has_event)
        })?;
        if has_event == 0 {
            return Ok(None);
        }
        Ok(Some(take_stream_event(event)?))
    }

    /// Finish the current stream and return the final result.
    pub fn finish_stream(&mut self) -> Result<TaskResult> {
        self.require_streaming()?;
        let mut raw = audio_cpp_task_result::default();
        check(unsafe { audio_cpp_session_finish_stream(self.ptr, &mut raw) })?;
        take_task_result(raw)
    }

    /// Reset streaming state so another stream can be started.
    pub fn reset(&mut self) -> Result<()> {
        self.require_streaming()?;
        check(unsafe { audio_cpp_session_reset(self.ptr) })
    }

    fn require_streaming(&self) -> Result<()> {
        if self.supports_streaming() {
            Ok(())
        } else {
            Err(Error::unsupported(
                "session does not support streaming runs",
            ))
        }
    }

    fn install_callback(&mut self, callback: Box<dyn FnMut(&StreamEvent)>) -> Result<()> {
        self.require_streaming()?;
        self.clear_callback()?;
        let mut state = Box::new(CallbackState { callback });
        check(unsafe {
            audio_cpp_session_set_stream_callback(
                self.ptr,
                Some(stream_event_trampoline),
                ptr::from_mut(state.as_mut()).cast(),
            )
        })?;
        self.callback = Some(state);
        Ok(())
    }

    fn clear_callback(&mut self) -> Result<()> {
        if self.supports_streaming() {
            check(unsafe {
                audio_cpp_session_set_stream_callback(self.ptr, None, ptr::null_mut())
            })?;
        }
        self.callback = None;
        Ok(())
    }
}

impl Debug for Session<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("ptr", &self.ptr)
            .field("has_callback", &self.callback.is_some())
            .finish()
    }
}

impl Drop for Session<'_> {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            let _ = self.clear_callback();
            unsafe { audio_cpp_session_destroy(self.ptr) };
            self.ptr = ptr::null_mut();
        }
    }
}

/// Offline-only view of a [`Session`].
#[derive(Debug)]
pub struct OfflineSession<'model> {
    inner: Session<'model>,
}

impl<'model> OfflineSession<'model> {
    /// Underlying session.
    #[must_use]
    pub fn session(&self) -> &Session<'model> {
        &self.inner
    }

    /// Mutable underlying session.
    pub fn session_mut(&mut self) -> &mut Session<'model> {
        &mut self.inner
    }

    /// Prepare graphs and caches.
    pub fn prepare(&mut self, request: &SessionPreparationRequest) -> Result<()> {
        self.inner.prepare(request)
    }

    /// Prepare from a task request.
    pub fn prepare_from_request(&mut self, request: &TaskRequest) -> Result<()> {
        self.inner.prepare_from_request(request)
    }

    /// Run a prepared offline task.
    pub fn run(&mut self, request: &TaskRequest) -> Result<TaskResult> {
        self.inner.run(request)
    }

    /// Prepare and run in one call.
    pub fn prepare_and_run(&mut self, request: &TaskRequest) -> Result<TaskResult> {
        self.inner.prepare_and_run(request)
    }
}

/// Streaming-only view of a [`Session`].
#[derive(Debug)]
pub struct StreamingSession<'model> {
    inner: Session<'model>,
}

impl<'model> StreamingSession<'model> {
    /// Underlying session.
    #[must_use]
    pub fn session(&self) -> &Session<'model> {
        &self.inner
    }

    /// Mutable underlying session.
    pub fn session_mut(&mut self) -> &mut Session<'model> {
        &mut self.inner
    }

    /// Prepare graphs and caches.
    pub fn prepare(&mut self, request: &SessionPreparationRequest) -> Result<()> {
        self.inner.prepare(request)
    }

    /// Prepare from a task request.
    pub fn prepare_from_request(&mut self, request: &TaskRequest) -> Result<()> {
        self.inner.prepare_from_request(request)
    }

    /// Advertised streaming policy.
    pub fn policy(&self) -> Result<StreamingPolicy> {
        self.inner.streaming_policy()
    }

    /// Run `f` with a scoped event callback.
    pub fn with_event_callback<R>(
        &mut self,
        callback: impl FnMut(&StreamEvent) + 'static,
        f: impl FnOnce(&mut Session<'model>) -> Result<R>,
    ) -> Result<R> {
        self.inner.with_event_callback(callback, f)
    }

    /// Start a stream.
    pub fn start(&mut self, request: &TaskRequest) -> Result<()> {
        self.inner.start_stream(request)
    }

    /// Process one audio chunk.
    pub fn process_audio_chunk(&mut self, chunk: &AudioChunk) -> Result<StreamEvent> {
        self.inner.process_audio_chunk(chunk)
    }

    /// Pull the next event.
    pub fn next_event(&mut self) -> Result<Option<StreamEvent>> {
        self.inner.next_stream_event()
    }

    /// Finish the stream.
    pub fn finish(&mut self) -> Result<TaskResult> {
        self.inner.finish_stream()
    }

    /// Reset the stream.
    pub fn reset(&mut self) -> Result<()> {
        self.inner.reset()
    }
}
