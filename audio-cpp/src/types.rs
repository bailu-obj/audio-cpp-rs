//! Idiomatic value types for the generic `audio.cpp` runtime.

use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::str::FromStr;

use audio_cpp_sys::{
    audio_cpp_artifact_kind, audio_cpp_artifact_kind_parse, audio_cpp_backend_type,
    audio_cpp_backend_type_parse, audio_cpp_run_mode, audio_cpp_run_mode_parse,
    audio_cpp_streaming_input_kind, audio_cpp_streaming_output_kind, audio_cpp_vad_event_kind,
    audio_cpp_voice_task_kind, audio_cpp_voice_task_kind_parse,
};

use crate::error::{Error, Result};
use crate::options::TaskOptions;

macro_rules! c_enum {
    ($(#[$meta:meta])* $name:ident, $raw:ty, $parse:ident, $($variant:ident => $const:path, $str:literal),+ $(,)?) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum $name {
            $(
                #[doc = concat!("Upstream name `", $str, "`.")]
                $variant,
            )+
        }

        impl $name {
            pub(crate) fn from_raw(value: $raw) -> Result<Self> {
                match value {
                    $($const => Ok(Self::$variant),)+
                }
            }

            pub(crate) fn to_raw(self) -> $raw {
                match self {
                    $(Self::$variant => $const,)+
                }
            }

            /// Canonical `audio.cpp` string name.
            #[must_use]
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $str,)+
                }
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = Error;

            fn from_str(s: &str) -> Result<Self> {
                let cstr = std::ffi::CString::new(s)?;
                let mut raw = unsafe { std::mem::zeroed::<$raw>() };
                crate::convert::check(unsafe { $parse(cstr.as_ptr(), &mut raw) })?;
                Self::from_raw(raw)
            }
        }
    };
}

c_enum!(
    /// Voice task advertised by a model.
    VoiceTaskKind,
    audio_cpp_voice_task_kind,
    audio_cpp_voice_task_kind_parse,
    Vad => audio_cpp_voice_task_kind::AUDIO_CPP_TASK_VAD, "vad",
    Asr => audio_cpp_voice_task_kind::AUDIO_CPP_TASK_ASR, "asr",
    Diarization => audio_cpp_voice_task_kind::AUDIO_CPP_TASK_DIARIZATION, "diar",
    SourceSeparation => audio_cpp_voice_task_kind::AUDIO_CPP_TASK_SOURCE_SEPARATION, "sep",
    AudioGeneration => audio_cpp_voice_task_kind::AUDIO_CPP_TASK_AUDIO_GENERATION, "gen",
    Tts => audio_cpp_voice_task_kind::AUDIO_CPP_TASK_TTS, "tts",
    VoiceCloning => audio_cpp_voice_task_kind::AUDIO_CPP_TASK_VOICE_CLONING, "clon",
    VoiceConversion => audio_cpp_voice_task_kind::AUDIO_CPP_TASK_VOICE_CONVERSION, "vc",
    SpeechToSpeech => audio_cpp_voice_task_kind::AUDIO_CPP_TASK_SPEECH_TO_SPEECH, "s2s",
    Alignment => audio_cpp_voice_task_kind::AUDIO_CPP_TASK_ALIGNMENT, "align",
    VoiceDesign => audio_cpp_voice_task_kind::AUDIO_CPP_TASK_VOICE_DESIGN, "vdes",
    SpeakerRecognition => audio_cpp_voice_task_kind::AUDIO_CPP_TASK_SPEAKER_RECOGNITION, "spk",
    Svc => audio_cpp_voice_task_kind::AUDIO_CPP_TASK_SVC, "svc",
    Midi => audio_cpp_voice_task_kind::AUDIO_CPP_TASK_MIDI, "midi",
);

c_enum!(
    /// Offline or streaming execution mode.
    RunMode,
    audio_cpp_run_mode,
    audio_cpp_run_mode_parse,
    Offline => audio_cpp_run_mode::AUDIO_CPP_MODE_OFFLINE, "offline",
    Streaming => audio_cpp_run_mode::AUDIO_CPP_MODE_STREAMING, "streaming",
);

c_enum!(
    /// Compute backend requested for a session.
    BackendType,
    audio_cpp_backend_type,
    audio_cpp_backend_type_parse,
    Cpu => audio_cpp_backend_type::AUDIO_CPP_BACKEND_CPU, "cpu",
    Cuda => audio_cpp_backend_type::AUDIO_CPP_BACKEND_CUDA, "cuda",
    Hip => audio_cpp_backend_type::AUDIO_CPP_BACKEND_HIP, "hip",
    Vulkan => audio_cpp_backend_type::AUDIO_CPP_BACKEND_VULKAN, "vulkan",
    Metal => audio_cpp_backend_type::AUDIO_CPP_BACKEND_METAL, "metal",
    BestAvailable => audio_cpp_backend_type::AUDIO_CPP_BACKEND_BEST_AVAILABLE, "best",
);

c_enum!(
    /// Kind of opaque voice artifact.
    ArtifactKind,
    audio_cpp_artifact_kind,
    audio_cpp_artifact_kind_parse,
    SpeakerEmbedding => audio_cpp_artifact_kind::AUDIO_CPP_ARTIFACT_SPEAKER_EMBEDDING, "speaker_embedding",
    StyleEmbedding => audio_cpp_artifact_kind::AUDIO_CPP_ARTIFACT_STYLE_EMBEDDING, "style_embedding",
    PromptEmbedding => audio_cpp_artifact_kind::AUDIO_CPP_ARTIFACT_PROMPT_EMBEDDING, "prompt_embedding",
    AcousticTokens => audio_cpp_artifact_kind::AUDIO_CPP_ARTIFACT_ACOUSTIC_TOKENS, "acoustic_tokens",
    Midi => audio_cpp_artifact_kind::AUDIO_CPP_ARTIFACT_MIDI, "midi",
    TranscriptAlignment => audio_cpp_artifact_kind::AUDIO_CPP_ARTIFACT_TRANSCRIPT_ALIGNMENT, "transcript_alignment",
    DiarizationState => audio_cpp_artifact_kind::AUDIO_CPP_ARTIFACT_DIARIZATION_STATE, "diarization_state",
    VadState => audio_cpp_artifact_kind::AUDIO_CPP_ARTIFACT_VAD_STATE, "vad_state",
    Custom => audio_cpp_artifact_kind::AUDIO_CPP_ARTIFACT_CUSTOM, "custom",
);

/// How a streaming session consumes input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StreamingInputKind {
    /// The session does not consume audio chunks.
    None,
    /// The session accepts incremental PCM chunks.
    AudioChunks,
}

impl StreamingInputKind {
    pub(crate) fn from_raw(value: audio_cpp_streaming_input_kind) -> Self {
        match value {
            audio_cpp_streaming_input_kind::AUDIO_CPP_STREAMING_INPUT_NONE => Self::None,
            audio_cpp_streaming_input_kind::AUDIO_CPP_STREAMING_INPUT_AUDIO_CHUNKS => {
                Self::AudioChunks
            }
        }
    }
}

/// How a streaming session produces output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StreamingOutputKind {
    /// Only the final [`crate::TaskResult`] is meaningful.
    FinalResult,
    /// Intermediate events can be pulled or delivered via callback.
    PullEvents,
}

impl StreamingOutputKind {
    pub(crate) fn from_raw(value: audio_cpp_streaming_output_kind) -> Self {
        match value {
            audio_cpp_streaming_output_kind::AUDIO_CPP_STREAMING_OUTPUT_FINAL_RESULT => {
                Self::FinalResult
            }
            audio_cpp_streaming_output_kind::AUDIO_CPP_STREAMING_OUTPUT_PULL_EVENTS => {
                Self::PullEvents
            }
        }
    }
}

/// Voice-activity event class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum VadEventKind {
    /// Speech onset.
    SpeechStart,
    /// Speech offset.
    SpeechEnd,
    /// A completed speech segment.
    SpeechSegment,
}

impl VadEventKind {
    pub(crate) fn from_raw(value: audio_cpp_vad_event_kind) -> Self {
        match value {
            audio_cpp_vad_event_kind::AUDIO_CPP_VAD_SPEECH_START => Self::SpeechStart,
            audio_cpp_vad_event_kind::AUDIO_CPP_VAD_SPEECH_END => Self::SpeechEnd,
            audio_cpp_vad_event_kind::AUDIO_CPP_VAD_SPEECH_SEGMENT => Self::SpeechSegment,
        }
    }
}

/// Interleaved `f32` PCM buffer.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioBuffer {
    /// Sample rate in Hz.
    pub sample_rate: i32,
    /// Channel count.
    pub channels: i32,
    /// Interleaved samples.
    pub samples: Vec<f32>,
}

impl AudioBuffer {
    /// Create a PCM buffer.
    #[must_use]
    pub fn new(sample_rate: i32, channels: i32, samples: Vec<f32>) -> Self {
        Self {
            sample_rate,
            channels,
            samples,
        }
    }
}

/// Text plus optional language tag.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Transcript {
    /// Transcript or prompt text.
    pub text: String,
    /// BCP-47 or model-specific language tag.
    pub language: String,
}

impl Transcript {
    /// Create a transcript.
    pub fn new(text: impl Into<String>, language: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            language: language.into(),
        }
    }
}

/// Half-open sample range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TimeSpan {
    /// Inclusive start sample.
    pub start_sample: i64,
    /// Exclusive end sample.
    pub end_sample: i64,
}

/// Speech segment with optional text.
#[derive(Debug, Clone, PartialEq)]
pub struct SpeechSegment {
    /// Sample span.
    pub span: TimeSpan,
    /// Model confidence in `[0, 1]`.
    pub confidence: f32,
    /// Recognized text when available.
    pub text: String,
}

/// Diarized speaker turn.
#[derive(Debug, Clone, PartialEq)]
pub struct SpeakerTurn {
    /// Sample span.
    pub span: TimeSpan,
    /// Speaker label.
    pub speaker_id: String,
    /// Model confidence in `[0, 1]`.
    pub confidence: f32,
    /// Recognized text when available.
    pub text: String,
}

/// Word-level timestamp.
#[derive(Debug, Clone, PartialEq)]
pub struct WordTimestamp {
    /// Sample span.
    pub span: TimeSpan,
    /// Word token.
    pub word: String,
    /// Model confidence in `[0, 1]`.
    pub confidence: f32,
}

/// Speaker reference for cloning or conditioning.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VoiceReference {
    /// Reference audio.
    pub audio: Option<AudioBuffer>,
    /// Previously cached voice identifier.
    pub cached_voice_id: Option<String>,
}

/// Style controls for generation tasks.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StyleCondition {
    /// Language hint.
    pub language: Option<String>,
    /// Emotion hint.
    pub emotion: Option<String>,
    /// Speaking-rate multiplier.
    pub speaking_rate: Option<f32>,
    /// Pitch shift.
    pub pitch_shift: Option<f32>,
    /// Energy scale.
    pub energy_scale: Option<f32>,
    /// Model-specific tags.
    pub tags: BTreeMap<String, String>,
}

/// Combined speaker and style condition.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VoiceCondition {
    /// Speaker reference.
    pub speaker: Option<VoiceReference>,
    /// Style controls.
    pub style: Option<StyleCondition>,
}

/// Opaque model artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceArtifact {
    /// Artifact class.
    pub kind: ArtifactKind,
    /// Artifact identifier.
    pub id: String,
    /// Raw payload bytes.
    pub payload: Vec<u8>,
    /// Artifact metadata.
    pub meta: BTreeMap<String, String>,
}

/// Named PCM output with metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedAudioBuffer {
    /// Output identifier.
    pub id: String,
    /// Audio samples.
    pub audio: AudioBuffer,
    /// Output metadata.
    pub meta: BTreeMap<String, String>,
}

/// Incremental PCM chunk for streaming sessions.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioChunk {
    /// Sample rate in Hz.
    pub sample_rate: i32,
    /// Channel count.
    pub channels: i32,
    /// Absolute start sample of this chunk.
    pub start_sample: i64,
    /// Interleaved samples.
    pub samples: Vec<f32>,
}

impl AudioChunk {
    /// Create a streaming audio chunk.
    #[must_use]
    pub fn new(sample_rate: i32, channels: i32, start_sample: i64, samples: Vec<f32>) -> Self {
        Self {
            sample_rate,
            channels,
            start_sample,
            samples,
        }
    }
}

/// Voice-activity event emitted by a streaming session.
#[derive(Debug, Clone, PartialEq)]
pub struct VoiceActivityEvent {
    /// Event class.
    pub kind: VadEventKind,
    /// Sample index of the event.
    pub sample: i64,
    /// Speech probability when available.
    pub probability: f32,
    /// Completed segment when `kind` is [`VadEventKind::SpeechSegment`].
    pub segment: Option<SpeechSegment>,
}

/// Task and run-mode pair used to create a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskSpec {
    /// Voice task to run.
    pub task: VoiceTaskKind,
    /// Offline or streaming execution.
    pub mode: RunMode,
}

impl TaskSpec {
    /// Create a task specification.
    #[must_use]
    pub fn new(task: VoiceTaskKind, mode: RunMode) -> Self {
        Self { task, mode }
    }

    /// Offline task helper.
    #[must_use]
    pub fn offline(task: VoiceTaskKind) -> Self {
        Self::new(task, RunMode::Offline)
    }

    /// Streaming task helper.
    #[must_use]
    pub fn streaming(task: VoiceTaskKind) -> Self {
        Self::new(task, RunMode::Streaming)
    }
}

/// Compute-backend selection for a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendConfig {
    /// Backend family.
    pub backend: BackendType,
    /// Device index within that backend.
    pub device: i32,
    /// Host thread count.
    pub threads: i32,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            backend: BackendType::Cpu,
            device: 0,
            threads: 1,
        }
    }
}

impl BackendConfig {
    /// CPU backend with the given thread count.
    #[must_use]
    pub fn cpu(threads: i32) -> Self {
        Self {
            backend: BackendType::Cpu,
            device: 0,
            threads,
        }
    }
}

/// Options used when creating a task session.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SessionOptions {
    /// Backend selection.
    pub backend: BackendConfig,
    /// Model-specific string options.
    pub options: BTreeMap<String, String>,
}

impl SessionOptions {
    /// Start a builder with default CPU settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the backend configuration.
    #[must_use]
    pub fn backend(mut self, backend: BackendConfig) -> Self {
        self.backend = backend;
        self
    }

    /// Insert a string option.
    #[must_use]
    pub fn option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }
}

/// Request used to inspect or load a model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelLoadRequest {
    /// Model file or directory.
    pub model_path: PathBuf,
    /// Optional model-spec override path.
    pub model_spec_override: Option<PathBuf>,
    /// Optional family hint that skips loader probing.
    pub family_hint: Option<String>,
    /// Optional config identifier.
    pub config_id: Option<String>,
    /// Optional weight identifier.
    pub weight_id: Option<String>,
    /// Loader-specific options.
    pub options: BTreeMap<String, String>,
}

impl ModelLoadRequest {
    /// Load the model at `model_path`.
    pub fn new(model_path: impl Into<PathBuf>) -> Self {
        Self {
            model_path: model_path.into(),
            model_spec_override: None,
            family_hint: None,
            config_id: None,
            weight_id: None,
            options: BTreeMap::new(),
        }
    }

    /// Override the model spec path.
    #[must_use]
    pub fn model_spec_override(mut self, path: impl Into<PathBuf>) -> Self {
        self.model_spec_override = Some(path.into());
        self
    }

    /// Set a family hint.
    #[must_use]
    pub fn family_hint(mut self, family: impl Into<String>) -> Self {
        self.family_hint = Some(family.into());
        self
    }

    /// Set a config identifier.
    #[must_use]
    pub fn config_id(mut self, id: impl Into<String>) -> Self {
        self.config_id = Some(id.into());
        self
    }

    /// Set a weight identifier.
    #[must_use]
    pub fn weight_id(mut self, id: impl Into<String>) -> Self {
        self.weight_id = Some(id.into());
        self
    }

    /// Insert a load option.
    #[must_use]
    pub fn option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }
}

/// Generic task input.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TaskRequest {
    /// Text prompt or transcript.
    pub text: Option<Transcript>,
    /// Input audio.
    pub audio: Option<AudioBuffer>,
    /// Voice/style condition.
    pub voice: Option<VoiceCondition>,
    /// Input artifacts.
    pub artifacts: Vec<VoiceArtifact>,
    /// Typed request options. Unset fields are omitted at the FFI boundary.
    pub options: TaskOptions,
}

impl TaskRequest {
    /// Empty request.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set text input.
    #[must_use]
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(Transcript::new(text, ""));
        self
    }

    /// Set a full transcript.
    #[must_use]
    pub fn transcript(mut self, transcript: Transcript) -> Self {
        self.text = Some(transcript);
        self
    }

    /// Set input audio.
    #[must_use]
    pub fn audio(mut self, audio: AudioBuffer) -> Self {
        self.audio = Some(audio);
        self
    }

    /// Set voice conditioning.
    #[must_use]
    pub fn voice(mut self, voice: VoiceCondition) -> Self {
        self.voice = Some(voice);
        self
    }

    /// Set typed request options, including per-family structs.
    #[must_use]
    pub fn options(mut self, options: impl Into<TaskOptions>) -> Self {
        self.options = options.into();
        self
    }
}

/// Audio-shape contract used during session preparation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AudioPreparationContract {
    /// Expected sample rate.
    pub sample_rate: i32,
    /// Expected channel count.
    pub channels: i32,
    /// Maximum input samples the session should prepare for.
    pub max_input_samples: i64,
}

/// Values passed to [`crate::Session::prepare`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SessionPreparationRequest {
    /// Optional audio contract.
    pub audio: Option<AudioPreparationContract>,
    /// Optional text used to size graphs.
    pub text: Option<Transcript>,
    /// Optional voice condition.
    pub voice: Option<VoiceCondition>,
    /// Model-specific options.
    pub options: BTreeMap<String, String>,
}

/// Streaming chunk and event policy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StreamingPolicy {
    /// How audio is consumed.
    pub input: StreamingInputKind,
    /// How events are produced.
    pub output: StreamingOutputKind,
    /// Preferred chunk length in samples.
    pub preferred_audio_chunk_samples: i64,
    /// Preferred chunk length in seconds.
    pub preferred_audio_chunk_seconds: f64,
}

/// Offline or finalized task output.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TaskResult {
    /// Primary audio output.
    pub audio: Option<AudioBuffer>,
    /// Additional named audio outputs.
    pub named_audio: Vec<NamedAudioBuffer>,
    /// Primary text output.
    pub text: Option<Transcript>,
    /// Speech segments.
    pub speech_segments: Vec<SpeechSegment>,
    /// Speaker turns.
    pub speaker_turns: Vec<SpeakerTurn>,
    /// Word timestamps.
    pub word_timestamps: Vec<WordTimestamp>,
    /// Primary artifact.
    pub artifact: Option<VoiceArtifact>,
    /// Additional artifacts.
    pub artifacts: Vec<VoiceArtifact>,
}

/// Incremental streaming event.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StreamEvent {
    /// Voice-activity updates.
    pub voice_activity: Vec<VoiceActivityEvent>,
    /// Partial transcript.
    pub partial_text: Option<Transcript>,
    /// Incremental audio.
    pub audio: Option<AudioBuffer>,
    /// Named incremental audio.
    pub named_audio: Vec<NamedAudioBuffer>,
    /// Speaker turns so far.
    pub speaker_turns: Vec<SpeakerTurn>,
    /// Word timestamps so far.
    pub word_timestamps: Vec<WordTimestamp>,
    /// Incremental artifacts.
    pub artifacts: Vec<VoiceArtifact>,
    /// Whether this event ends the stream.
    pub is_final: bool,
}

/// Discovered model file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedAsset {
    /// Asset identifier.
    pub id: String,
    /// Filesystem path.
    pub path: PathBuf,
}

/// One supported task and its run modes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCapability {
    /// Task kind.
    pub task: VoiceTaskKind,
    /// Supported run modes.
    pub modes: Vec<RunMode>,
}

/// Advertised model capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CapabilitySet {
    /// Supported tasks.
    pub tasks: Vec<TaskCapability>,
    /// Supported languages.
    pub languages: Vec<String>,
    /// Whether speaker reference is accepted.
    pub supports_speaker_reference: bool,
    /// Whether style conditioning is accepted.
    pub supports_style_condition: bool,
    /// Whether timestamps are produced.
    pub supports_timestamps: bool,
}

/// Identity metadata for a loaded or inspected model.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModelMetadata {
    /// Model family.
    pub family: String,
    /// Family variant.
    pub variant: String,
    /// Human-readable description.
    pub description: String,
    /// Known config identifiers.
    pub config_candidates: Vec<String>,
    /// Known weight identifiers.
    pub weight_candidates: Vec<String>,
}

/// Result of inspecting a model path without fully loading weights.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModelInspection {
    /// Model identity.
    pub metadata: ModelMetadata,
    /// Advertised capabilities.
    pub capabilities: CapabilitySet,
    /// Resolved model root.
    pub model_root: PathBuf,
    /// Discovered config assets.
    pub discovered_configs: Vec<NamedAsset>,
    /// Discovered weight assets.
    pub discovered_weights: Vec<NamedAsset>,
}

/// A ggml backend device discovered at runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendDevice {
    /// ggml registry name, such as `CPU` or `METAL`.
    pub backend: String,
    /// Device index within that registry.
    pub index: i32,
    /// Human-readable device name.
    pub name: String,
    /// Device class such as `CPU` or `GPU`.
    pub kind: String,
}
