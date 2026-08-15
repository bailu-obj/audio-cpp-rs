//! Typed request options for the generic `audio.cpp` runtime.
//!
//! Common knobs live on [`TaskOptions`]. Families with extra request keys have
//! dedicated structs that convert into [`TaskOptions`] and serialize to the
//! native string map only at the FFI boundary. Unset fields are omitted so
//! schema-v1 models are not sent unknown keys.

use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

/// How long-form text is split before synthesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextChunkMode {
    /// Upstream default chunker.
    Default,
    /// Split on a word/token budget.
    WordBudget,
    /// Honor markup tags when splitting.
    TagAware,
    /// Japanese-aware sentence splitting.
    Japanese,
    /// Split on newlines.
    Endline,
}

impl TextChunkMode {
    /// Canonical `audio.cpp` string name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::WordBudget => "word_budget",
            Self::TagAware => "tag_aware",
            Self::Japanese => "japanese",
            Self::Endline => "endline",
        }
    }
}

impl Display for TextChunkMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How long-form audio is split before recognition or conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AudioChunkMode {
    /// Let the model choose a chunker.
    Auto,
    /// Fixed-duration windows.
    Fixed,
    /// Voice-activity windows.
    Vad,
    /// Energy-based quiet detection.
    QuietEnergy,
    /// Disable chunking.
    None,
}

impl AudioChunkMode {
    /// Canonical `audio.cpp` string name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Fixed => "fixed",
            Self::Vad => "vad",
            Self::QuietEnergy => "quiet_energy",
            Self::None => "none",
        }
    }
}

impl Display for AudioChunkMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Shared request-scope options accepted across many model families.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TaskOptions {
    /// RNG seed.
    pub seed: Option<u32>,
    /// Sampler temperature.
    pub temperature: Option<f32>,
    /// Top-k sampling cutoff.
    pub top_k: Option<i32>,
    /// Nucleus sampling cutoff.
    pub top_p: Option<f32>,
    /// Min-p sampling cutoff.
    pub min_p: Option<f32>,
    /// Enable stochastic sampling.
    pub do_sample: Option<bool>,
    /// Beam-search width.
    pub num_beams: Option<i32>,
    /// Sampler name, such as `euler`.
    pub sampler: Option<String>,
    /// Maximum generated tokens.
    pub max_tokens: Option<i32>,
    /// Alias for [`Self::max_tokens`] used by some families.
    pub max_new_tokens: Option<i32>,
    /// Repetition penalty.
    pub repetition_penalty: Option<f32>,
    /// Beam length penalty.
    pub length_penalty: Option<f32>,
    /// Classifier-free guidance scale.
    pub guidance_scale: Option<f32>,
    /// Diffusion or flow-matching steps.
    pub num_inference_steps: Option<i32>,
    /// Negative prompt text.
    pub negative_prompt: Option<String>,
    /// Target duration in seconds.
    pub duration_sec: Option<f32>,
    /// Duration multiplier.
    pub duration_scale: Option<f32>,
    /// Spatio-temporal guidance scale.
    pub spatio_temporal_guidance_scale: Option<f32>,
    /// Guidance rescale, such as `auto` or a numeric string.
    pub guidance_rescale: Option<String>,
    /// Language hint.
    pub language: Option<String>,
    /// Clone-reference transcript.
    pub reference_text: Option<String>,
    /// Clone-reference language.
    pub reference_language: Option<String>,
    /// Clone-reference crop length in seconds.
    pub reference_duration_sec: Option<f32>,
    /// Long-form text chunker.
    pub text_chunk_mode: Option<TextChunkMode>,
    /// Long-form text chunk budget.
    pub text_chunk_size: Option<i32>,
    /// Long-form audio chunker.
    pub audio_chunk_mode: Option<AudioChunkMode>,
    /// Audio chunk length in seconds.
    pub audio_chunk_duration_sec: Option<f32>,
    /// Long-form audio chunking threshold in seconds.
    pub audio_chunk_threshold_sec: Option<f32>,
    /// Cross-fade between audio chunks in seconds.
    pub cross_fade_duration_sec: Option<f32>,
    /// Request word timestamps when the model supports them.
    pub return_timestamps: Option<bool>,
    /// Clone or target-voice audio path.
    pub target_voice: Option<PathBuf>,
    /// Voice-conversion source audio path.
    pub source_audio: Option<PathBuf>,
    /// Lyrics for music-generation models.
    pub lyrics: Option<String>,
    /// Model route name.
    pub route: Option<String>,
    /// Family-specific extras.
    pub family: Option<FamilyOptions>,
}

impl TaskOptions {
    /// Empty options; the model uses its defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn encode(&self) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        encode_common(self, &mut map);
        if let Some(family) = &self.family {
            family.encode_into(&mut map);
        }
        map
    }
}

/// Per-family request extras that are not in [`TaskOptions`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FamilyOptions {
    /// [`VoxCpm2Options`] extras.
    VoxCpm2(VoxCpm2Options),
    /// [`Qwen3TtsOptions`] extras.
    Qwen3Tts(Qwen3TtsOptions),
    /// [`SileroVadOptions`] extras.
    SileroVad(SileroVadOptions),
    /// [`DramaBoxOptions`] extras.
    DramaBox(DramaBoxOptions),
    /// [`ChatterboxOptions`] extras.
    Chatterbox(ChatterboxOptions),
    /// [`PocketTtsOptions`] extras.
    PocketTts(PocketTtsOptions),
    /// [`NemotronAsrOptions`] extras.
    NemotronAsr(NemotronAsrOptions),
    /// [`IndexTts2Options`] extras.
    IndexTts2(IndexTts2Options),
    /// [`AceStepOptions`] extras.
    AceStep(Box<AceStepOptions>),
    /// [`OmnivoiceOptions`] extras.
    Omnivoice(OmnivoiceOptions),
    /// [`StableAudioOptions`] extras.
    StableAudio(StableAudioOptions),
}

impl FamilyOptions {
    fn encode_into(&self, map: &mut BTreeMap<String, String>) {
        match self {
            Self::VoxCpm2(options) => options.encode_family(map),
            Self::Qwen3Tts(options) => options.encode_family(map),
            Self::SileroVad(options) => options.encode_family(map),
            Self::DramaBox(options) => options.encode_family(map),
            Self::Chatterbox(options) => options.encode_family(map),
            Self::PocketTts(options) => options.encode_family(map),
            Self::NemotronAsr(options) => options.encode_family(map),
            Self::IndexTts2(options) => options.encode_family(map),
            Self::AceStep(options) => options.encode_family(map),
            Self::Omnivoice(options) => options.encode_family(map),
            Self::StableAudio(options) => options.encode_family(map),
        }
    }
}

/// `VoxCPM2` generation options.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VoxCpm2Options {
    /// RNG seed.
    pub seed: Option<u32>,
    /// Maximum generated tokens.
    pub max_tokens: Option<i32>,
    /// Classifier-free guidance scale.
    pub guidance_scale: Option<f32>,
    /// Flow-matching steps.
    pub num_inference_steps: Option<i32>,
    /// Long-form text chunker.
    pub text_chunk_mode: Option<TextChunkMode>,
    /// Long-form text chunk budget.
    pub text_chunk_size: Option<i32>,
    /// Minimum generated tokens.
    pub min_tokens: Option<i32>,
    /// Retry failed utterances.
    pub retry_badcase: Option<bool>,
    /// Maximum retry attempts.
    pub retry_badcase_max_times: Option<i32>,
    /// Retry quality threshold.
    pub retry_badcase_ratio_threshold: Option<f32>,
    /// Optional CFM noise file.
    pub cfm_noise_file: Option<PathBuf>,
    /// Style or prompt-text override.
    pub prompt_text: Option<String>,
}

impl VoxCpm2Options {
    fn encode_family(&self, map: &mut BTreeMap<String, String>) {
        insert_i32(map, "min_tokens", self.min_tokens);
        insert_bool(map, "retry_badcase", self.retry_badcase);
        insert_i32(map, "retry_badcase_max_times", self.retry_badcase_max_times);
        insert_f32(
            map,
            "retry_badcase_ratio_threshold",
            self.retry_badcase_ratio_threshold,
        );
        insert_path(map, "cfm_noise_file", self.cfm_noise_file.as_deref());
        insert_str(map, "prompt_text", self.prompt_text.as_deref());
    }
}

impl From<VoxCpm2Options> for TaskOptions {
    fn from(value: VoxCpm2Options) -> Self {
        Self {
            seed: value.seed,
            max_tokens: value.max_tokens,
            guidance_scale: value.guidance_scale,
            num_inference_steps: value.num_inference_steps,
            text_chunk_mode: value.text_chunk_mode,
            text_chunk_size: value.text_chunk_size,
            family: Some(FamilyOptions::VoxCpm2(value)),
            ..Self::default()
        }
    }
}

/// `Qwen3-TTS` generation options.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Qwen3TtsOptions {
    /// RNG seed.
    pub seed: Option<u32>,
    /// Maximum generated tokens.
    pub max_tokens: Option<i32>,
    /// Enable stochastic sampling.
    pub do_sample: Option<bool>,
    /// Sampler temperature.
    pub temperature: Option<f32>,
    /// Top-k sampling cutoff.
    pub top_k: Option<i32>,
    /// Nucleus sampling cutoff.
    pub top_p: Option<f32>,
    /// Repetition penalty.
    pub repetition_penalty: Option<f32>,
    /// Clone-reference transcript.
    pub reference_text: Option<String>,
    /// Subtalker stochastic sampling.
    pub subtalker_do_sample: Option<bool>,
    /// Subtalker temperature.
    pub subtalker_temperature: Option<f32>,
    /// Subtalker top-k cutoff.
    pub subtalker_top_k: Option<i32>,
    /// Subtalker nucleus cutoff.
    pub subtalker_top_p: Option<f32>,
    /// Voice-design instruction.
    pub instruct: Option<String>,
    /// Custom-voice speaker name.
    pub speaker: Option<String>,
    /// Use speaker embedding only.
    pub x_vector_only_mode: Option<bool>,
}

impl Qwen3TtsOptions {
    fn encode_family(&self, map: &mut BTreeMap<String, String>) {
        insert_bool(map, "subtalker_do_sample", self.subtalker_do_sample);
        insert_f32(map, "subtalker_temperature", self.subtalker_temperature);
        insert_i32(map, "subtalker_top_k", self.subtalker_top_k);
        insert_f32(map, "subtalker_top_p", self.subtalker_top_p);
        insert_str(map, "instruct", self.instruct.as_deref());
        insert_str(map, "speaker", self.speaker.as_deref());
        insert_bool(map, "x_vector_only_mode", self.x_vector_only_mode);
    }
}

impl From<Qwen3TtsOptions> for TaskOptions {
    fn from(value: Qwen3TtsOptions) -> Self {
        Self {
            seed: value.seed,
            max_tokens: value.max_tokens,
            do_sample: value.do_sample,
            temperature: value.temperature,
            top_k: value.top_k,
            top_p: value.top_p,
            repetition_penalty: value.repetition_penalty,
            reference_text: value.reference_text.clone(),
            family: Some(FamilyOptions::Qwen3Tts(value)),
            ..Self::default()
        }
    }
}

/// Silero voice-activity options.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SileroVadOptions {
    /// Speech-probability threshold.
    pub threshold: Option<f32>,
    /// Minimum speech span in milliseconds.
    pub min_speech_duration_ms: Option<i32>,
    /// Minimum silence span in milliseconds.
    pub min_silence_duration_ms: Option<i32>,
    /// Padding added around speech in milliseconds.
    pub speech_pad_ms: Option<i32>,
    /// Maximum speech span in seconds.
    pub max_speech_duration_s: Option<f32>,
    /// Negative-threshold used after speech starts.
    pub neg_threshold: Option<f32>,
    /// Silence required when the max-speech cap is hit.
    pub min_silence_at_max_speech_ms: Option<i32>,
    /// Use the longest possible silence at the max-speech cap.
    pub use_max_poss_sil_at_max_speech: Option<bool>,
}

impl SileroVadOptions {
    fn encode_family(&self, map: &mut BTreeMap<String, String>) {
        insert_f32(map, "threshold", self.threshold);
        insert_i32(map, "min_speech_duration_ms", self.min_speech_duration_ms);
        insert_i32(map, "min_silence_duration_ms", self.min_silence_duration_ms);
        insert_i32(map, "speech_pad_ms", self.speech_pad_ms);
        insert_f32(map, "max_speech_duration_s", self.max_speech_duration_s);
        insert_f32(map, "neg_threshold", self.neg_threshold);
        insert_i32(
            map,
            "min_silence_at_max_speech_ms",
            self.min_silence_at_max_speech_ms,
        );
        insert_bool(
            map,
            "use_max_poss_sil_at_max_speech",
            self.use_max_poss_sil_at_max_speech,
        );
    }
}

impl From<SileroVadOptions> for TaskOptions {
    fn from(value: SileroVadOptions) -> Self {
        Self {
            family: Some(FamilyOptions::SileroVad(value)),
            ..Self::default()
        }
    }
}

/// `DramaBox` generation options.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DramaBoxOptions {
    /// RNG seed.
    pub seed: Option<u32>,
    /// Diffusion steps.
    pub num_inference_steps: Option<i32>,
    /// Classifier-free guidance scale.
    pub guidance_scale: Option<f32>,
    /// Spatio-temporal guidance scale.
    pub spatio_temporal_guidance_scale: Option<f32>,
    /// Duration multiplier.
    pub duration_scale: Option<f32>,
    /// Target duration in seconds.
    pub duration_sec: Option<f32>,
    /// Clone-reference crop length in seconds.
    pub reference_duration_sec: Option<f32>,
    /// Long-form audio chunking threshold in seconds.
    pub audio_chunk_threshold_sec: Option<f32>,
    /// Audio chunk length in seconds.
    pub audio_chunk_duration_sec: Option<f32>,
    /// Cross-fade between audio chunks in seconds.
    pub cross_fade_duration_sec: Option<f32>,
    /// Negative prompt text.
    pub negative_prompt: Option<String>,
    /// Guidance rescale, such as `auto`.
    pub guidance_rescale: Option<String>,
    /// Clone or target-voice audio path.
    pub target_voice: Option<PathBuf>,
}

impl DramaBoxOptions {
    #[allow(clippy::unused_self)]
    fn encode_family(&self, _map: &mut BTreeMap<String, String>) {}
}

impl From<DramaBoxOptions> for TaskOptions {
    fn from(value: DramaBoxOptions) -> Self {
        Self {
            seed: value.seed,
            num_inference_steps: value.num_inference_steps,
            guidance_scale: value.guidance_scale,
            spatio_temporal_guidance_scale: value.spatio_temporal_guidance_scale,
            duration_scale: value.duration_scale,
            duration_sec: value.duration_sec,
            reference_duration_sec: value.reference_duration_sec,
            audio_chunk_threshold_sec: value.audio_chunk_threshold_sec,
            audio_chunk_duration_sec: value.audio_chunk_duration_sec,
            cross_fade_duration_sec: value.cross_fade_duration_sec,
            negative_prompt: value.negative_prompt.clone(),
            guidance_rescale: value.guidance_rescale.clone(),
            target_voice: value.target_voice.clone(),
            family: Some(FamilyOptions::DramaBox(value)),
            ..Self::default()
        }
    }
}

/// Chatterbox generation options.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChatterboxOptions {
    /// RNG seed.
    pub seed: Option<u32>,
    /// Sampler temperature.
    pub temperature: Option<f32>,
    /// Nucleus sampling cutoff.
    pub top_p: Option<f32>,
    /// Min-p sampling cutoff.
    pub min_p: Option<f32>,
    /// Enable stochastic sampling.
    pub do_sample: Option<bool>,
    /// Maximum generated tokens.
    pub max_tokens: Option<i32>,
    /// Repetition penalty.
    pub repetition_penalty: Option<f32>,
    /// Classifier-free guidance scale.
    pub guidance_scale: Option<f32>,
    /// Clone or target-voice audio path.
    pub target_voice: Option<PathBuf>,
    /// Voice-conversion source audio path.
    pub source_audio: Option<PathBuf>,
    /// Expressiveness exaggeration.
    pub exaggeration: Option<f32>,
    /// `S3Gen` classifier-free guidance rate.
    pub s3gen_cfg_rate: Option<f32>,
    /// Stop decoding on EOS.
    pub stop_on_eos: Option<bool>,
    /// Force greedy decoding.
    pub greedy: Option<bool>,
}

impl ChatterboxOptions {
    fn encode_family(&self, map: &mut BTreeMap<String, String>) {
        insert_f32(map, "exaggeration", self.exaggeration);
        insert_f32(map, "s3gen_cfg_rate", self.s3gen_cfg_rate);
        insert_bool(map, "stop_on_eos", self.stop_on_eos);
        insert_bool(map, "greedy", self.greedy);
    }
}

impl From<ChatterboxOptions> for TaskOptions {
    fn from(value: ChatterboxOptions) -> Self {
        Self {
            seed: value.seed,
            temperature: value.temperature,
            top_p: value.top_p,
            min_p: value.min_p,
            do_sample: value.do_sample,
            max_tokens: value.max_tokens,
            repetition_penalty: value.repetition_penalty,
            guidance_scale: value.guidance_scale,
            target_voice: value.target_voice.clone(),
            source_audio: value.source_audio.clone(),
            family: Some(FamilyOptions::Chatterbox(value)),
            ..Self::default()
        }
    }
}

/// Pocket TTS generation options.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PocketTtsOptions {
    /// RNG seed.
    pub seed: Option<u32>,
    /// Sampler temperature.
    pub temperature: Option<f32>,
    /// Maximum generated tokens.
    pub max_tokens: Option<i32>,
    /// Language hint.
    pub language: Option<String>,
    /// Flow-matching steps.
    pub max_steps: Option<i32>,
    /// Noise clamp.
    pub noise_clamp: Option<f32>,
    /// End-of-speech threshold.
    pub eos_threshold: Option<f32>,
    /// Optional noise file.
    pub noise_file: Option<PathBuf>,
    /// Extra frames after EOS.
    pub frames_after_eos: Option<i32>,
    /// Precomputed voice-embedding path.
    pub voice_embedding_path: Option<PathBuf>,
    /// Clone-reference transcript.
    pub voice_clone_text: Option<String>,
    /// Truncate clone audio to the model window.
    pub truncate_clone_audio: Option<bool>,
}

impl PocketTtsOptions {
    fn encode_family(&self, map: &mut BTreeMap<String, String>) {
        insert_i32(map, "max_steps", self.max_steps);
        insert_f32(map, "noise_clamp", self.noise_clamp);
        insert_f32(map, "eos_threshold", self.eos_threshold);
        insert_path(map, "noise_file", self.noise_file.as_deref());
        insert_i32(map, "frames_after_eos", self.frames_after_eos);
        insert_path(
            map,
            "voice_embedding_path",
            self.voice_embedding_path.as_deref(),
        );
        insert_str(map, "voice_clone_text", self.voice_clone_text.as_deref());
        insert_bool(map, "truncate_clone_audio", self.truncate_clone_audio);
    }
}

impl From<PocketTtsOptions> for TaskOptions {
    fn from(value: PocketTtsOptions) -> Self {
        Self {
            seed: value.seed,
            temperature: value.temperature,
            max_tokens: value.max_tokens,
            language: value.language.clone(),
            family: Some(FamilyOptions::PocketTts(value)),
            ..Self::default()
        }
    }
}

/// Nemotron ASR options.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NemotronAsrOptions {
    /// Language hint.
    pub language: Option<String>,
    /// Maximum generated tokens.
    pub max_tokens: Option<i32>,
    /// Encoder lookahead tokens.
    pub lookahead_tokens: Option<i32>,
    /// Keep language tags in the transcript.
    pub keep_language_tags: Option<bool>,
    /// Force streaming decode on an offline session.
    pub streaming: Option<bool>,
}

impl NemotronAsrOptions {
    fn encode_family(&self, map: &mut BTreeMap<String, String>) {
        insert_i32(map, "lookahead_tokens", self.lookahead_tokens);
        insert_bool(map, "keep_language_tags", self.keep_language_tags);
        insert_bool(map, "streaming", self.streaming);
    }
}

impl From<NemotronAsrOptions> for TaskOptions {
    fn from(value: NemotronAsrOptions) -> Self {
        Self {
            language: value.language.clone(),
            max_tokens: value.max_tokens,
            family: Some(FamilyOptions::NemotronAsr(value)),
            ..Self::default()
        }
    }
}

/// `IndexTTS2` generation options.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IndexTts2Options {
    /// RNG seed.
    pub seed: Option<u32>,
    /// Enable stochastic sampling.
    pub do_sample: Option<bool>,
    /// Sampler temperature.
    pub temperature: Option<f32>,
    /// Top-k sampling cutoff.
    pub top_k: Option<i32>,
    /// Nucleus sampling cutoff.
    pub top_p: Option<f32>,
    /// Maximum generated tokens.
    pub max_tokens: Option<i32>,
    /// Beam-search width.
    pub num_beams: Option<i32>,
    /// Repetition penalty.
    pub repetition_penalty: Option<f32>,
    /// Beam length penalty.
    pub length_penalty: Option<f32>,
    /// Language hint.
    pub language: Option<String>,
    /// Duration multiplier.
    pub duration_factor: Option<f32>,
    /// Emotion mix in `[0, 1]`.
    pub emotion_alpha: Option<f32>,
    /// Explicit emotion vector.
    pub emotion_vector: Option<String>,
    /// Derive emotion from text.
    pub use_emotion_text: Option<bool>,
    /// Emotion-description text.
    pub emotion_text: Option<String>,
    /// Sample a random emotion.
    pub use_random_emotion: Option<bool>,
    /// Silence inserted between sentences, in milliseconds.
    pub interval_silence_ms: Option<i32>,
}

impl IndexTts2Options {
    fn encode_family(&self, map: &mut BTreeMap<String, String>) {
        insert_f32(map, "duration_factor", self.duration_factor);
        insert_f32(map, "emotion_alpha", self.emotion_alpha);
        insert_str(map, "emotion_vector", self.emotion_vector.as_deref());
        insert_bool(map, "use_emotion_text", self.use_emotion_text);
        insert_str(map, "emotion_text", self.emotion_text.as_deref());
        insert_bool(map, "use_random_emotion", self.use_random_emotion);
        insert_i32(map, "interval_silence_ms", self.interval_silence_ms);
    }
}

impl From<IndexTts2Options> for TaskOptions {
    fn from(value: IndexTts2Options) -> Self {
        Self {
            seed: value.seed,
            do_sample: value.do_sample,
            temperature: value.temperature,
            top_k: value.top_k,
            top_p: value.top_p,
            max_tokens: value.max_tokens,
            num_beams: value.num_beams,
            repetition_penalty: value.repetition_penalty,
            length_penalty: value.length_penalty,
            language: value.language.clone(),
            family: Some(FamilyOptions::IndexTts2(value)),
            ..Self::default()
        }
    }
}

/// `ACE-Step` music-generation options.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AceStepOptions {
    /// RNG seed.
    pub seed: Option<u32>,
    /// Diffusion steps.
    pub num_inference_steps: Option<i32>,
    /// Classifier-free guidance scale.
    pub guidance_scale: Option<f32>,
    /// Target duration in seconds.
    pub duration_sec: Option<f32>,
    /// Negative prompt text.
    pub negative_prompt: Option<String>,
    /// Lyrics.
    pub lyrics: Option<String>,
    /// Model route name.
    pub route: Option<String>,
    /// Sampler name.
    pub sampler: Option<String>,
    /// Instruction prompt.
    pub instruction: Option<String>,
    /// Serialized audio codes.
    pub audio_codes: Option<String>,
    /// Alternate audio-code string.
    pub audio_code_string: Option<String>,
    /// Track name.
    pub track_name: Option<String>,
    /// Beats per minute.
    pub bpm: Option<i32>,
    /// Musical key and scale.
    pub keyscale: Option<String>,
    /// Time signature.
    pub timesignature: Option<String>,
    /// Chunk-mask mode.
    pub chunk_mask_mode: Option<String>,
    /// Repaint mode.
    pub repaint_mode: Option<String>,
    /// Repaint strength.
    pub repaint_strength: Option<f32>,
    /// Repaint window start in seconds.
    pub repainting_start: Option<f32>,
    /// Repaint window end in seconds.
    pub repainting_end: Option<f32>,
    /// Enable thinking / chain-of-thought.
    pub thinking: Option<bool>,
    /// Use chain-of-thought metadata.
    pub use_cot_metas: Option<bool>,
    /// Use chain-of-thought caption.
    pub use_cot_caption: Option<bool>,
    /// Use chain-of-thought language.
    pub use_cot_language: Option<bool>,
    /// Enable ADG.
    pub use_adg: Option<bool>,
    /// CFG interval start.
    pub cfg_interval_start: Option<f32>,
    /// CFG interval end.
    pub cfg_interval_end: Option<f32>,
    /// Language-model temperature.
    pub lm_temperature: Option<f32>,
    /// Language-model CFG scale.
    pub lm_cfg_scale: Option<f32>,
    /// Language-model top-k cutoff.
    pub lm_top_k: Option<i32>,
    /// Language-model nucleus cutoff.
    pub lm_top_p: Option<f32>,
    /// Language-model repetition penalty.
    pub lm_repetition_penalty: Option<f32>,
    /// Flow-matching shift.
    pub shift: Option<f32>,
    /// Inference method name.
    pub infer_method: Option<String>,
    /// Cover-audio strength.
    pub audio_cover_strength: Option<f32>,
    /// Cover-noise strength.
    pub cover_noise_strength: Option<f32>,
    /// Retake seed.
    pub retake_seed: Option<i32>,
    /// Retake variance.
    pub retake_variance: Option<f32>,
    /// Sampler mode name.
    pub sampler_mode: Option<String>,
    /// Velocity-norm threshold.
    pub velocity_norm_threshold: Option<f32>,
    /// Velocity EMA factor.
    pub velocity_ema_factor: Option<f32>,
    /// Enable DCW.
    pub dcw_enabled: Option<bool>,
    /// DCW mode name.
    pub dcw_mode: Option<String>,
    /// DCW scaler.
    pub dcw_scaler: Option<f32>,
    /// DCW high-band scaler.
    pub dcw_high_scaler: Option<f32>,
    /// DCW wavelet name.
    pub dcw_wavelet: Option<String>,
    /// Repaint cross-fade frames.
    pub repaint_crossfade_frames: Option<i32>,
    /// Repaint injection ratio.
    pub repaint_injection_ratio: Option<f32>,
}

impl AceStepOptions {
    fn encode_family(&self, map: &mut BTreeMap<String, String>) {
        insert_str(map, "instruction", self.instruction.as_deref());
        insert_str(map, "audio_codes", self.audio_codes.as_deref());
        insert_str(map, "audio_code_string", self.audio_code_string.as_deref());
        insert_str(map, "track_name", self.track_name.as_deref());
        insert_i32(map, "bpm", self.bpm);
        insert_str(map, "keyscale", self.keyscale.as_deref());
        insert_str(map, "timesignature", self.timesignature.as_deref());
        insert_str(map, "chunk_mask_mode", self.chunk_mask_mode.as_deref());
        insert_str(map, "repaint_mode", self.repaint_mode.as_deref());
        insert_f32(map, "repaint_strength", self.repaint_strength);
        insert_f32(map, "repainting_start", self.repainting_start);
        insert_f32(map, "repainting_end", self.repainting_end);
        insert_bool(map, "thinking", self.thinking);
        insert_bool(map, "use_cot_metas", self.use_cot_metas);
        insert_bool(map, "use_cot_caption", self.use_cot_caption);
        insert_bool(map, "use_cot_language", self.use_cot_language);
        insert_bool(map, "use_adg", self.use_adg);
        insert_f32(map, "cfg_interval_start", self.cfg_interval_start);
        insert_f32(map, "cfg_interval_end", self.cfg_interval_end);
        insert_f32(map, "lm_temperature", self.lm_temperature);
        insert_f32(map, "lm_cfg_scale", self.lm_cfg_scale);
        insert_i32(map, "lm_top_k", self.lm_top_k);
        insert_f32(map, "lm_top_p", self.lm_top_p);
        insert_f32(map, "lm_repetition_penalty", self.lm_repetition_penalty);
        insert_f32(map, "shift", self.shift);
        insert_str(map, "infer_method", self.infer_method.as_deref());
        insert_f32(map, "audio_cover_strength", self.audio_cover_strength);
        insert_f32(map, "cover_noise_strength", self.cover_noise_strength);
        insert_i32(map, "retake_seed", self.retake_seed);
        insert_f32(map, "retake_variance", self.retake_variance);
        insert_str(map, "sampler_mode", self.sampler_mode.as_deref());
        insert_f32(map, "velocity_norm_threshold", self.velocity_norm_threshold);
        insert_f32(map, "velocity_ema_factor", self.velocity_ema_factor);
        insert_bool(map, "dcw_enabled", self.dcw_enabled);
        insert_str(map, "dcw_mode", self.dcw_mode.as_deref());
        insert_f32(map, "dcw_scaler", self.dcw_scaler);
        insert_f32(map, "dcw_high_scaler", self.dcw_high_scaler);
        insert_str(map, "dcw_wavelet", self.dcw_wavelet.as_deref());
        insert_i32(
            map,
            "repaint_crossfade_frames",
            self.repaint_crossfade_frames,
        );
        insert_f32(map, "repaint_injection_ratio", self.repaint_injection_ratio);
        if self.duration_sec.is_some() {
            insert_f32(map, "duration_seconds", self.duration_sec);
        }
    }
}

impl From<AceStepOptions> for TaskOptions {
    fn from(value: AceStepOptions) -> Self {
        Self {
            seed: value.seed,
            num_inference_steps: value.num_inference_steps,
            guidance_scale: value.guidance_scale,
            duration_sec: value.duration_sec,
            negative_prompt: value.negative_prompt.clone(),
            lyrics: value.lyrics.clone(),
            route: value.route.clone(),
            sampler: value.sampler.clone(),
            family: Some(FamilyOptions::AceStep(Box::new(value))),
            ..Self::default()
        }
    }
}

/// `OmniVoice` generation options.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OmnivoiceOptions {
    /// RNG seed.
    pub seed: Option<u32>,
    /// Diffusion or flow-matching steps.
    pub num_inference_steps: Option<i32>,
    /// Classifier-free guidance scale.
    pub guidance_scale: Option<f32>,
    /// Long-form text chunker.
    pub text_chunk_mode: Option<TextChunkMode>,
    /// Long-form text chunk budget.
    pub text_chunk_size: Option<i32>,
    /// Speaking-rate multiplier.
    pub speed: Option<f32>,
    /// Target duration in seconds (`duration`).
    pub duration: Option<f32>,
    /// Flow-matching time shift.
    pub t_shift: Option<f32>,
    /// Enable the denoiser.
    pub denoise: Option<bool>,
    /// Preprocess the prompt.
    pub preprocess_prompt: Option<bool>,
    /// Postprocess the output audio.
    pub postprocess_output: Option<bool>,
    /// Layer-penalty factor.
    pub layer_penalty_factor: Option<f32>,
    /// Position temperature.
    pub position_temperature: Option<f32>,
    /// Class temperature.
    pub class_temperature: Option<f32>,
    /// Audio chunk length in seconds (`audio_chunk_duration`).
    pub audio_chunk_duration: Option<f32>,
    /// Long-form audio chunking threshold (`audio_chunk_threshold`).
    pub audio_chunk_threshold: Option<f32>,
}

impl OmnivoiceOptions {
    fn encode_family(&self, map: &mut BTreeMap<String, String>) {
        insert_f32(map, "speed", self.speed);
        insert_f32(map, "duration", self.duration);
        insert_f32(map, "t_shift", self.t_shift);
        insert_bool(map, "denoise", self.denoise);
        insert_bool(map, "preprocess_prompt", self.preprocess_prompt);
        insert_bool(map, "postprocess_output", self.postprocess_output);
        insert_f32(map, "layer_penalty_factor", self.layer_penalty_factor);
        insert_f32(map, "position_temperature", self.position_temperature);
        insert_f32(map, "class_temperature", self.class_temperature);
        insert_f32(map, "audio_chunk_duration", self.audio_chunk_duration);
        insert_f32(map, "audio_chunk_threshold", self.audio_chunk_threshold);
    }
}

impl From<OmnivoiceOptions> for TaskOptions {
    fn from(value: OmnivoiceOptions) -> Self {
        Self {
            seed: value.seed,
            num_inference_steps: value.num_inference_steps,
            guidance_scale: value.guidance_scale,
            text_chunk_mode: value.text_chunk_mode,
            text_chunk_size: value.text_chunk_size,
            family: Some(FamilyOptions::Omnivoice(value)),
            ..Self::default()
        }
    }
}

/// `Stable Audio` generation options.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StableAudioOptions {
    /// RNG seed.
    pub seed: Option<u32>,
    /// Diffusion steps.
    pub num_inference_steps: Option<i32>,
    /// Classifier-free guidance scale.
    pub guidance_scale: Option<f32>,
    /// Target duration in seconds.
    pub duration_sec: Option<f32>,
    /// Negative prompt text.
    pub negative_prompt: Option<String>,
    /// Sampler name.
    pub sampler: Option<String>,
    /// Prompt text when not using [`crate::TaskRequest::text`].
    pub prompt: Option<String>,
    /// Batch size.
    pub batch_size: Option<i32>,
    /// APG scale.
    pub apg_scale: Option<f32>,
    /// Truncate output to the requested duration.
    pub truncate_output_to_duration: Option<bool>,
    /// Decode in chunks.
    pub chunked_decode: Option<bool>,
    /// Duration padding in seconds.
    pub duration_padding_seconds: Option<f32>,
    /// Initial noise level.
    pub init_noise_level: Option<f32>,
    /// How input audio is interpreted.
    pub audio_input_kind: Option<String>,
    /// Inpaint window start in seconds.
    pub inpaint_mask_start_seconds: Option<f32>,
    /// Inpaint window end in seconds.
    pub inpaint_mask_end_seconds: Option<f32>,
    /// Sampler sigma minimum.
    pub sigma_min: Option<f32>,
    /// Sampler sigma maximum.
    pub sigma_max: Option<f32>,
    /// Sampler rho.
    pub rho: Option<f32>,
}

impl StableAudioOptions {
    fn encode_family(&self, map: &mut BTreeMap<String, String>) {
        insert_str(map, "prompt", self.prompt.as_deref());
        insert_i32(map, "batch_size", self.batch_size);
        insert_f32(map, "apg_scale", self.apg_scale);
        insert_bool(
            map,
            "truncate_output_to_duration",
            self.truncate_output_to_duration,
        );
        insert_bool(map, "chunked_decode", self.chunked_decode);
        insert_f32(
            map,
            "duration_padding_seconds",
            self.duration_padding_seconds,
        );
        insert_f32(map, "init_noise_level", self.init_noise_level);
        insert_str(map, "audio_input_kind", self.audio_input_kind.as_deref());
        insert_f32(
            map,
            "inpaint_mask_start_seconds",
            self.inpaint_mask_start_seconds,
        );
        insert_f32(
            map,
            "inpaint_mask_end_seconds",
            self.inpaint_mask_end_seconds,
        );
        insert_f32(map, "sigma_min", self.sigma_min);
        insert_f32(map, "sigma_max", self.sigma_max);
        insert_f32(map, "rho", self.rho);
        if self.duration_sec.is_some() {
            insert_f32(map, "duration_seconds", self.duration_sec);
        }
    }
}

impl From<StableAudioOptions> for TaskOptions {
    fn from(value: StableAudioOptions) -> Self {
        Self {
            seed: value.seed,
            num_inference_steps: value.num_inference_steps,
            guidance_scale: value.guidance_scale,
            duration_sec: value.duration_sec,
            negative_prompt: value.negative_prompt.clone(),
            sampler: value.sampler.clone(),
            family: Some(FamilyOptions::StableAudio(value)),
            ..Self::default()
        }
    }
}

fn encode_common(options: &TaskOptions, map: &mut BTreeMap<String, String>) {
    insert_u32(map, "seed", options.seed);
    insert_f32(map, "temperature", options.temperature);
    insert_i32(map, "top_k", options.top_k);
    insert_f32(map, "top_p", options.top_p);
    insert_f32(map, "min_p", options.min_p);
    insert_bool(map, "do_sample", options.do_sample);
    insert_i32(map, "num_beams", options.num_beams);
    insert_str(map, "sampler", options.sampler.as_deref());
    insert_i32(map, "max_tokens", options.max_tokens);
    insert_i32(map, "max_new_tokens", options.max_new_tokens);
    insert_f32(map, "repetition_penalty", options.repetition_penalty);
    insert_f32(map, "length_penalty", options.length_penalty);
    insert_f32(map, "guidance_scale", options.guidance_scale);
    insert_i32(map, "num_inference_steps", options.num_inference_steps);
    insert_str(map, "negative_prompt", options.negative_prompt.as_deref());
    insert_f32(map, "duration_sec", options.duration_sec);
    insert_f32(map, "duration_scale", options.duration_scale);
    insert_f32(
        map,
        "spatio_temporal_guidance_scale",
        options.spatio_temporal_guidance_scale,
    );
    insert_str(map, "guidance_rescale", options.guidance_rescale.as_deref());
    insert_str(map, "language", options.language.as_deref());
    insert_str(map, "reference_text", options.reference_text.as_deref());
    insert_str(
        map,
        "reference_language",
        options.reference_language.as_deref(),
    );
    insert_f32(
        map,
        "reference_duration_sec",
        options.reference_duration_sec,
    );
    if let Some(mode) = options.text_chunk_mode {
        map.insert("text_chunk_mode".into(), mode.as_str().into());
    }
    insert_i32(map, "text_chunk_size", options.text_chunk_size);
    if let Some(mode) = options.audio_chunk_mode {
        map.insert("audio_chunk_mode".into(), mode.as_str().into());
    }
    insert_f32(
        map,
        "audio_chunk_duration_sec",
        options.audio_chunk_duration_sec,
    );
    insert_f32(
        map,
        "audio_chunk_threshold_sec",
        options.audio_chunk_threshold_sec,
    );
    insert_f32(
        map,
        "cross_fade_duration_sec",
        options.cross_fade_duration_sec,
    );
    insert_bool(map, "return_timestamps", options.return_timestamps);
    insert_path(map, "target_voice", options.target_voice.as_deref());
    insert_path(map, "source_audio", options.source_audio.as_deref());
    insert_str(map, "lyrics", options.lyrics.as_deref());
    insert_str(map, "route", options.route.as_deref());
}

fn insert_u32(map: &mut BTreeMap<String, String>, key: &str, value: Option<u32>) {
    if let Some(value) = value {
        map.insert(key.into(), value.to_string());
    }
}

fn insert_i32(map: &mut BTreeMap<String, String>, key: &str, value: Option<i32>) {
    if let Some(value) = value {
        map.insert(key.into(), value.to_string());
    }
}

fn insert_f32(map: &mut BTreeMap<String, String>, key: &str, value: Option<f32>) {
    if let Some(value) = value {
        map.insert(key.into(), value.to_string());
    }
}

fn insert_bool(map: &mut BTreeMap<String, String>, key: &str, value: Option<bool>) {
    if let Some(value) = value {
        map.insert(key.into(), if value { "true" } else { "false" }.into());
    }
}

fn insert_str(map: &mut BTreeMap<String, String>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        map.insert(key.into(), value.to_owned());
    }
}

fn insert_path(map: &mut BTreeMap<String, String>, key: &str, value: Option<&Path>) {
    if let Some(value) = value {
        map.insert(key.into(), value.to_string_lossy().into_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omits_none_fields() {
        let encoded = TaskOptions {
            seed: Some(7),
            ..TaskOptions::default()
        }
        .encode();
        assert_eq!(encoded.get("seed").map(String::as_str), Some("7"));
        assert!(!encoded.contains_key("temperature"));
        assert!(!encoded.contains_key("guidance_scale"));
    }

    #[test]
    fn voxcpm2_encodes_unique_and_common_keys() {
        let encoded = TaskOptions::from(VoxCpm2Options {
            seed: Some(1234),
            guidance_scale: Some(2.0),
            num_inference_steps: Some(10),
            retry_badcase: Some(false),
            prompt_text: Some("warm".into()),
            ..VoxCpm2Options::default()
        })
        .encode();
        assert_eq!(encoded.get("seed").map(String::as_str), Some("1234"));
        assert_eq!(encoded.get("guidance_scale").map(String::as_str), Some("2"));
        assert_eq!(
            encoded.get("num_inference_steps").map(String::as_str),
            Some("10")
        );
        assert_eq!(
            encoded.get("retry_badcase").map(String::as_str),
            Some("false")
        );
        assert_eq!(encoded.get("prompt_text").map(String::as_str), Some("warm"));
        assert!(!encoded.contains_key("min_tokens"));
    }
}
