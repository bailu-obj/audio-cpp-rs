//! Safe Rust bindings for the [audio.cpp](https://github.com/bailu-obj/audio.cpp)
//! generic voice runtime.
//!
//! The crate wraps the C ABI from [`audio-cpp-sys`](audio_cpp_sys) with RAII
//! handles, owned value types, and explicit prepare/run semantics.
//!
//! ```no_run
//! use audio_cpp::{
//!     ModelLoadRequest, Registry, SessionOptions, TaskOptions, TaskRequest, TaskSpec,
//!     VoiceTaskKind,
//! };
//!
//! let registry = Registry::new()?;
//! let model = registry.load(&ModelLoadRequest::new("/path/to/model").family_hint("qwen3_tts"))?;
//! let mut session = model.create_session(TaskSpec::offline(VoiceTaskKind::Tts), &SessionOptions::new())?;
//! let result = session.prepare_and_run(
//!     &TaskRequest::new()
//!         .text("Hello from Rust")
//!         .options(TaskOptions {
//!             seed: Some(1234),
//!             temperature: Some(0.7),
//!             ..TaskOptions::default()
//!         }),
//! )?;
//! let _audio = result.audio;
//! # Ok::<(), audio_cpp::Error>(())
//! ```

#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]

mod convert;
mod error;
mod model;
mod options;
mod registry;
mod session;
mod types;

pub use error::{Error, ErrorKind, Result};
pub use model::Model;
pub use options::{
    AceStepOptions, AudioChunkMode, ChatterboxOptions, DramaBoxOptions, FamilyOptions,
    IndexTts2Options, NemotronAsrOptions, OmnivoiceOptions, PocketTtsOptions, Qwen3TtsOptions,
    SileroVadOptions, StableAudioOptions, TaskOptions, TextChunkMode, VoxCpm2Options,
};
pub use registry::{backend_devices, Registry};
pub use session::{OfflineSession, Session, StreamingSession};
pub use types::{
    ArtifactKind, AudioBuffer, AudioChunk, AudioPreparationContract, BackendConfig, BackendDevice,
    BackendType, CapabilitySet, ModelInspection, ModelLoadRequest, ModelMetadata, NamedAsset,
    NamedAudioBuffer, RunMode, SessionOptions, SessionPreparationRequest, SpeakerTurn,
    SpeechSegment, StreamEvent, StreamingInputKind, StreamingOutputKind, StreamingPolicy,
    StyleCondition, TaskCapability, TaskRequest, TaskResult, TaskSpec, TimeSpan, Transcript,
    VadEventKind, VoiceActivityEvent, VoiceArtifact, VoiceCondition, VoiceReference, VoiceTaskKind,
    WordTimestamp,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_well_known_enums() {
        assert_eq!("tts".parse::<VoiceTaskKind>().unwrap(), VoiceTaskKind::Tts);
        assert_eq!("offline".parse::<RunMode>().unwrap(), RunMode::Offline);
        assert_eq!("metal".parse::<BackendType>().unwrap(), BackendType::Metal);
        assert_eq!("rocm".parse::<BackendType>().unwrap(), BackendType::Hip);
        assert_eq!(
            "speaker_embedding".parse::<ArtifactKind>().unwrap(),
            ArtifactKind::SpeakerEmbedding
        );
        assert!("nope".parse::<VoiceTaskKind>().is_err());
    }

    #[test]
    fn builders_preserve_fields() {
        let request = ModelLoadRequest::new("/tmp/model")
            .family_hint("silero_vad")
            .config_id("default")
            .option("key", "value");
        assert_eq!(request.model_path, PathBuf::from("/tmp/model"));
        assert_eq!(request.family_hint.as_deref(), Some("silero_vad"));
        assert_eq!(
            request.options.get("key").map(String::as_str),
            Some("value")
        );

        let task = TaskRequest::new()
            .text("hello")
            .audio(AudioBuffer::new(16_000, 1, vec![0.0, 0.1]))
            .options(TaskOptions {
                temperature: Some(0.7),
                ..TaskOptions::default()
            });
        assert_eq!(
            task.text.as_ref().map(|text| text.text.as_str()),
            Some("hello")
        );
        assert_eq!(
            task.audio.as_ref().map(|audio| audio.sample_rate),
            Some(16_000)
        );
        assert_eq!(task.options.temperature, Some(0.7));
    }

    #[test]
    fn registry_lists_compiled_families() {
        let registry = Registry::new().unwrap();
        assert!(!registry.is_empty());
        let families = registry.families().unwrap();
        assert!(!families.is_empty());
        assert!(registry.supports_family(&families[0]));
        assert!(Registry::bundled_model_spec("qwen3_tts").is_some());
        assert!(Registry::bundled_model_spec("../nope").is_none());
    }

    #[test]
    fn backend_enumeration_is_non_empty() {
        let devices = backend_devices().unwrap();
        assert!(!devices.is_empty());
        assert!(devices.iter().any(|device| device.kind.contains("CPU")
            || device.backend.to_ascii_uppercase().contains("CPU")
            || device.backend.to_ascii_uppercase().contains("METAL")));
    }

    #[test]
    fn missing_model_returns_native_error() {
        let registry = Registry::new().unwrap();
        let error = registry
            .inspect(&ModelLoadRequest::new(
                "/definitely/missing/audio-cpp-model",
            ))
            .unwrap_err();
        match error {
            Error::Native { .. } | Error::Unsupported(_) => {}
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    #[ignore = "set AUDIO_CPP_TEST_MODEL to a local model path"]
    fn load_model_from_env() {
        let path = std::env::var("AUDIO_CPP_TEST_MODEL").expect("AUDIO_CPP_TEST_MODEL");
        let registry = Registry::new().unwrap();
        let request = ModelLoadRequest::new(path);
        let inspection = registry.inspect(&request).unwrap();
        assert!(!inspection.metadata.family.is_empty());

        let model = registry.load(&request).unwrap();
        let capabilities = model.capabilities().unwrap();
        let task = capabilities
            .tasks
            .first()
            .cloned()
            .expect("model advertises at least one task");
        let mode = task.modes.first().copied().unwrap_or(RunMode::Offline);
        let mut session = model
            .create_session(TaskSpec::new(task.task, mode), &SessionOptions::new())
            .unwrap();
        session
            .prepare_from_request(&TaskRequest::new().text("hello from audio-cpp"))
            .unwrap();
        if session.supports_offline() {
            let _ = session.run(&TaskRequest::new().text("hello from audio-cpp"));
        }
        if session.supports_streaming() {
            let _ = session.streaming_policy().unwrap();
            let _ = session.reset();
        }
    }
}
