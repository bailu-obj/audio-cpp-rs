//! FFI conversion helpers.

use std::collections::BTreeMap;
use std::ffi::{c_char, CStr, CString};
use std::os::raw::c_void;
use std::path::{Path, PathBuf};
use std::ptr;
use std::slice;

use audio_cpp_sys::{
    audio_cpp_artifact_view, audio_cpp_audio_chunk, audio_cpp_audio_view, audio_cpp_backend_config,
    audio_cpp_backend_device, audio_cpp_backend_devices_free, audio_cpp_capability_set,
    audio_cpp_capability_set_free, audio_cpp_inspection, audio_cpp_inspection_free, audio_cpp_kv,
    audio_cpp_last_error, audio_cpp_list_backend_devices, audio_cpp_model_load_request,
    audio_cpp_model_metadata, audio_cpp_model_metadata_free, audio_cpp_named_asset,
    audio_cpp_named_audio, audio_cpp_owned_artifact, audio_cpp_owned_audio, audio_cpp_owned_kv,
    audio_cpp_owned_transcript, audio_cpp_session_options, audio_cpp_session_prep_request,
    audio_cpp_speaker_turn, audio_cpp_speech_segment, audio_cpp_status, audio_cpp_stream_event,
    audio_cpp_stream_event_free, audio_cpp_string_list_free, audio_cpp_task_request,
    audio_cpp_task_result, audio_cpp_task_result_free, audio_cpp_task_spec, audio_cpp_transcript,
    audio_cpp_vad_event, audio_cpp_voice_condition, audio_cpp_voice_reference,
    audio_cpp_word_timestamp,
};

use crate::error::{Error, Result};
use crate::types::{
    ArtifactKind, AudioBuffer, AudioChunk, BackendDevice, CapabilitySet, ModelInspection,
    ModelLoadRequest, ModelMetadata, NamedAsset, NamedAudioBuffer, SessionOptions,
    SessionPreparationRequest, SpeakerTurn, SpeechSegment, StreamEvent, StreamingInputKind,
    StreamingOutputKind, StreamingPolicy, TaskCapability, TaskRequest, TaskResult, TaskSpec,
    Transcript, VadEventKind, VoiceActivityEvent, VoiceArtifact, VoiceCondition, VoiceTaskKind,
    WordTimestamp,
};

pub(crate) fn check(status: audio_cpp_status) -> Result<()> {
    if status == audio_cpp_status::AUDIO_CPP_OK {
        Ok(())
    } else {
        Err(Error::native(status, last_error()))
    }
}

pub(crate) fn last_error() -> String {
    unsafe {
        let ptr = audio_cpp_last_error();
        if ptr.is_null() {
            String::new()
        } else {
            CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    }
}

pub(crate) fn path_cstring(path: &Path) -> Result<CString> {
    let text = path
        .to_str()
        .ok_or_else(|| Error::InvalidPath(path.to_path_buf()))?;
    Ok(CString::new(text)?)
}

pub(crate) fn copy_c_str(ptr: *const c_char) -> Result<String> {
    if ptr.is_null() {
        Ok(String::new())
    } else {
        Ok(unsafe { CStr::from_ptr(ptr) }.to_str()?.to_owned())
    }
}

pub(crate) fn take_string_list(items: *mut *mut c_char, count: usize) -> Result<Vec<String>> {
    let mut out = Vec::with_capacity(count);
    if !items.is_null() {
        let slice = unsafe { slice::from_raw_parts(items, count) };
        for item in slice {
            out.push(copy_c_str(*item)?);
        }
    }
    unsafe { audio_cpp_string_list_free(items, count) };
    Ok(out)
}

struct StringArena {
    strings: Vec<CString>,
}

impl StringArena {
    fn new() -> Self {
        Self {
            strings: Vec::new(),
        }
    }

    fn push(&mut self, value: &str) -> Result<*const c_char> {
        self.strings.push(CString::new(value)?);
        Ok(self.strings.last().expect("just pushed").as_ptr())
    }

    fn push_opt(&mut self, value: Option<&str>) -> Result<*const c_char> {
        match value {
            Some(value) => self.push(value),
            None => Ok(ptr::null()),
        }
    }
}

struct EncodedMap {
    #[allow(dead_code)]
    strings: StringArena,
    kvs: Vec<audio_cpp_kv>,
}

impl EncodedMap {
    fn new(map: &BTreeMap<String, String>) -> Result<Self> {
        let mut strings = StringArena::new();
        let mut kvs = Vec::with_capacity(map.len());
        for (key, value) in map {
            let key_ptr = strings.push(key)?;
            let value_ptr = strings.push(value)?;
            kvs.push(audio_cpp_kv {
                key: key_ptr,
                value: value_ptr,
            });
        }
        Ok(Self { strings, kvs })
    }

    fn as_ptr(&self) -> *const audio_cpp_kv {
        if self.kvs.is_empty() {
            ptr::null()
        } else {
            self.kvs.as_ptr()
        }
    }

    fn len(&self) -> usize {
        self.kvs.len()
    }
}

pub(crate) struct EncodedLoadRequest {
    pub request: audio_cpp_model_load_request,
    _path: CString,
    _spec: Option<CString>,
    _family: Option<CString>,
    _config: Option<CString>,
    _weight: Option<CString>,
    _options: EncodedMap,
}

impl EncodedLoadRequest {
    pub(crate) fn new(request: &ModelLoadRequest) -> Result<Self> {
        let path = path_cstring(&request.model_path)?;
        let spec = request
            .model_spec_override
            .as_deref()
            .map(path_cstring)
            .transpose()?;
        let family = request
            .family_hint
            .as_deref()
            .map(CString::new)
            .transpose()?;
        let config = request.config_id.as_deref().map(CString::new).transpose()?;
        let weight = request.weight_id.as_deref().map(CString::new).transpose()?;
        let options = EncodedMap::new(&request.options)?;
        let encoded = audio_cpp_model_load_request {
            model_path: path.as_ptr(),
            model_spec_override: spec.as_ref().map_or(ptr::null(), |value| value.as_ptr()),
            family_hint: family.as_ref().map_or(ptr::null(), |value| value.as_ptr()),
            config_id: config.as_ref().map_or(ptr::null(), |value| value.as_ptr()),
            weight_id: weight.as_ref().map_or(ptr::null(), |value| value.as_ptr()),
            options: options.as_ptr(),
            option_count: options.len(),
        };
        Ok(Self {
            request: encoded,
            _path: path,
            _spec: spec,
            _family: family,
            _config: config,
            _weight: weight,
            _options: options,
        })
    }
}

pub(crate) struct EncodedTaskRequest {
    pub request: audio_cpp_task_request,
    _strings: StringArena,
    _options: EncodedMap,
    _style_tags: EncodedMap,
    _artifacts: Vec<audio_cpp_artifact_view>,
    _artifact_meta: Vec<EncodedMap>,
}

impl EncodedTaskRequest {
    pub(crate) fn new(request: &TaskRequest) -> Result<Self> {
        let mut strings = StringArena::new();
        let encoded_options = request.options.encode();
        let options = EncodedMap::new(&encoded_options)?;
        let style_tags = EncodedMap::new(
            request
                .voice
                .as_ref()
                .and_then(|voice| voice.style.as_ref())
                .map_or(&BTreeMap::new(), |style| &style.tags),
        )?;
        let mut artifact_meta = Vec::new();
        let mut artifacts = Vec::new();
        for artifact in &request.artifacts {
            let meta = EncodedMap::new(&artifact.meta)?;
            artifacts.push(audio_cpp_artifact_view {
                kind: artifact.kind.to_raw(),
                id: strings.push(&artifact.id)?,
                payload: if artifact.payload.is_empty() {
                    ptr::null()
                } else {
                    artifact.payload.as_ptr()
                },
                payload_size: artifact.payload.len(),
                meta: meta.as_ptr(),
                meta_count: meta.len(),
            });
            artifact_meta.push(meta);
        }

        let mut encoded = audio_cpp_task_request {
            text: audio_cpp_transcript {
                text: ptr::null(),
                language: ptr::null(),
            },
            has_text: 0,
            audio: audio_view(request.audio.as_ref()),
            has_audio: u8::from(request.audio.is_some()),
            voice: audio_cpp_voice_condition::default(),
            has_voice: u8::from(request.voice.is_some()),
            artifacts: if artifacts.is_empty() {
                ptr::null()
            } else {
                artifacts.as_ptr()
            },
            artifact_count: artifacts.len(),
            options: options.as_ptr(),
            option_count: options.len(),
        };

        if let Some(text) = &request.text {
            encoded.text.text = strings.push(&text.text)?;
            encoded.text.language = strings.push(&text.language)?;
            encoded.has_text = 1;
        }
        if let Some(voice) = &request.voice {
            encoded.voice = encode_voice_condition(voice, &mut strings, &style_tags)?;
        }

        Ok(Self {
            request: encoded,
            _strings: strings,
            _options: options,
            _style_tags: style_tags,
            _artifacts: artifacts,
            _artifact_meta: artifact_meta,
        })
    }
}

pub(crate) struct EncodedPrepRequest {
    pub request: audio_cpp_session_prep_request,
    _strings: StringArena,
    _options: EncodedMap,
    _style_tags: EncodedMap,
}

impl EncodedPrepRequest {
    pub(crate) fn new(request: &SessionPreparationRequest) -> Result<Self> {
        let mut strings = StringArena::new();
        let options = EncodedMap::new(&request.options)?;
        let style_tags = EncodedMap::new(
            request
                .voice
                .as_ref()
                .and_then(|voice| voice.style.as_ref())
                .map_or(&BTreeMap::new(), |style| &style.tags),
        )?;
        let mut encoded = audio_cpp_session_prep_request {
            audio_sample_rate: 0,
            audio_channels: 0,
            max_input_samples: 0,
            has_audio: 0,
            text: audio_cpp_transcript {
                text: ptr::null(),
                language: ptr::null(),
            },
            has_text: 0,
            voice: audio_cpp_voice_condition::default(),
            has_voice: u8::from(request.voice.is_some()),
            options: options.as_ptr(),
            option_count: options.len(),
        };
        if let Some(audio) = request.audio {
            encoded.audio_sample_rate = audio.sample_rate;
            encoded.audio_channels = audio.channels;
            encoded.max_input_samples = audio.max_input_samples;
            encoded.has_audio = 1;
        }
        if let Some(text) = &request.text {
            encoded.text.text = strings.push(&text.text)?;
            encoded.text.language = strings.push(&text.language)?;
            encoded.has_text = 1;
        }
        if let Some(voice) = &request.voice {
            encoded.voice = encode_voice_condition(voice, &mut strings, &style_tags)?;
        }
        Ok(Self {
            request: encoded,
            _strings: strings,
            _options: options,
            _style_tags: style_tags,
        })
    }
}

fn encode_voice_condition(
    voice: &VoiceCondition,
    strings: &mut StringArena,
    style_tags: &EncodedMap,
) -> Result<audio_cpp_voice_condition> {
    let mut encoded = audio_cpp_voice_condition {
        speaker: audio_cpp_voice_reference {
            audio: audio_cpp_audio_view::default(),
            has_audio: 0,
            cached_voice_id: ptr::null(),
        },
        has_speaker: u8::from(voice.speaker.is_some()),
        style: audio_cpp_sys::audio_cpp_style_condition::default(),
        has_style: u8::from(voice.style.is_some()),
    };
    if let Some(speaker) = &voice.speaker {
        encoded.speaker.audio = audio_view(speaker.audio.as_ref());
        encoded.speaker.has_audio = u8::from(speaker.audio.is_some());
        encoded.speaker.cached_voice_id = strings.push_opt(speaker.cached_voice_id.as_deref())?;
    }
    if let Some(style) = &voice.style {
        encoded.style.language = strings.push_opt(style.language.as_deref())?;
        encoded.style.emotion = strings.push_opt(style.emotion.as_deref())?;
        encoded.style.speaking_rate = style.speaking_rate.unwrap_or_default();
        encoded.style.has_speaking_rate = u8::from(style.speaking_rate.is_some());
        encoded.style.pitch_shift = style.pitch_shift.unwrap_or_default();
        encoded.style.has_pitch_shift = u8::from(style.pitch_shift.is_some());
        encoded.style.energy_scale = style.energy_scale.unwrap_or_default();
        encoded.style.has_energy_scale = u8::from(style.energy_scale.is_some());
        encoded.style.tags = style_tags.as_ptr();
        encoded.style.tag_count = style_tags.len();
    }
    Ok(encoded)
}

fn audio_view(audio: Option<&AudioBuffer>) -> audio_cpp_audio_view {
    match audio {
        Some(audio) => audio_cpp_audio_view {
            sample_rate: audio.sample_rate,
            channels: audio.channels,
            samples: if audio.samples.is_empty() {
                ptr::null()
            } else {
                audio.samples.as_ptr()
            },
            sample_count: audio.samples.len(),
        },
        None => audio_cpp_audio_view::default(),
    }
}

pub(crate) fn encode_task_spec(spec: TaskSpec) -> audio_cpp_task_spec {
    audio_cpp_task_spec {
        task: spec.task.to_raw(),
        mode: spec.mode.to_raw(),
    }
}

pub(crate) struct EncodedSessionOptions {
    pub options: audio_cpp_session_options,
    _options: EncodedMap,
}

impl EncodedSessionOptions {
    pub(crate) fn new(options: &SessionOptions) -> Result<Self> {
        let encoded_map = EncodedMap::new(&options.options)?;
        let encoded = audio_cpp_session_options {
            backend: audio_cpp_backend_config {
                type_: options.backend.backend.to_raw(),
                device: options.backend.device,
                threads: options.backend.threads,
            },
            options: encoded_map.as_ptr(),
            option_count: encoded_map.len(),
        };
        Ok(Self {
            options: encoded,
            _options: encoded_map,
        })
    }
}

pub(crate) fn encode_audio_chunk(chunk: &AudioChunk) -> audio_cpp_audio_chunk {
    audio_cpp_audio_chunk {
        sample_rate: chunk.sample_rate,
        channels: chunk.channels,
        start_sample: chunk.start_sample,
        samples: if chunk.samples.is_empty() {
            ptr::null()
        } else {
            chunk.samples.as_ptr()
        },
        sample_count: chunk.samples.len(),
    }
}

fn owned_kvs(values: *const audio_cpp_owned_kv, count: usize) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    if values.is_null() {
        return Ok(out);
    }
    let slice = unsafe { slice::from_raw_parts(values, count) };
    for item in slice {
        out.insert(copy_c_str(item.key)?, copy_c_str(item.value)?);
    }
    Ok(out)
}

fn owned_audio(audio: &audio_cpp_owned_audio) -> AudioBuffer {
    let samples = if audio.samples.is_null() || audio.sample_count == 0 {
        Vec::new()
    } else {
        unsafe { slice::from_raw_parts(audio.samples, audio.sample_count) }.to_vec()
    };
    AudioBuffer {
        sample_rate: audio.sample_rate,
        channels: audio.channels,
        samples,
    }
}

fn owned_transcript(text: &audio_cpp_owned_transcript) -> Result<Transcript> {
    Ok(Transcript {
        text: copy_c_str(text.text)?,
        language: copy_c_str(text.language)?,
    })
}

fn owned_artifact(artifact: &audio_cpp_owned_artifact) -> Result<VoiceArtifact> {
    let payload = if artifact.payload.is_null() || artifact.payload_size == 0 {
        Vec::new()
    } else {
        unsafe { slice::from_raw_parts(artifact.payload, artifact.payload_size) }.to_vec()
    };
    Ok(VoiceArtifact {
        kind: ArtifactKind::from_raw(artifact.kind)?,
        id: copy_c_str(artifact.id)?,
        payload,
        meta: owned_kvs(artifact.meta, artifact.meta_count)?,
    })
}

fn named_audio(value: &audio_cpp_named_audio) -> Result<NamedAudioBuffer> {
    Ok(NamedAudioBuffer {
        id: copy_c_str(value.id)?,
        audio: owned_audio(&value.audio),
        meta: owned_kvs(value.meta, value.meta_count)?,
    })
}

fn speech_segment(value: &audio_cpp_speech_segment) -> Result<SpeechSegment> {
    Ok(SpeechSegment {
        span: crate::types::TimeSpan {
            start_sample: value.start_sample,
            end_sample: value.end_sample,
        },
        confidence: value.confidence,
        text: copy_c_str(value.text)?,
    })
}

fn speaker_turn(value: &audio_cpp_speaker_turn) -> Result<SpeakerTurn> {
    Ok(SpeakerTurn {
        span: crate::types::TimeSpan {
            start_sample: value.start_sample,
            end_sample: value.end_sample,
        },
        speaker_id: copy_c_str(value.speaker_id)?,
        confidence: value.confidence,
        text: copy_c_str(value.text)?,
    })
}

fn word_timestamp(value: &audio_cpp_word_timestamp) -> Result<WordTimestamp> {
    Ok(WordTimestamp {
        span: crate::types::TimeSpan {
            start_sample: value.start_sample,
            end_sample: value.end_sample,
        },
        word: copy_c_str(value.word)?,
        confidence: value.confidence,
    })
}

fn collect_named_audio(
    values: *const audio_cpp_named_audio,
    count: usize,
) -> Result<Vec<NamedAudioBuffer>> {
    if values.is_null() {
        return Ok(Vec::new());
    }
    unsafe { slice::from_raw_parts(values, count) }
        .iter()
        .map(named_audio)
        .collect()
}

fn collect_segments(
    values: *const audio_cpp_speech_segment,
    count: usize,
) -> Result<Vec<SpeechSegment>> {
    if values.is_null() {
        return Ok(Vec::new());
    }
    unsafe { slice::from_raw_parts(values, count) }
        .iter()
        .map(speech_segment)
        .collect()
}

fn collect_turns(values: *const audio_cpp_speaker_turn, count: usize) -> Result<Vec<SpeakerTurn>> {
    if values.is_null() {
        return Ok(Vec::new());
    }
    unsafe { slice::from_raw_parts(values, count) }
        .iter()
        .map(speaker_turn)
        .collect()
}

fn collect_words(
    values: *const audio_cpp_word_timestamp,
    count: usize,
) -> Result<Vec<WordTimestamp>> {
    if values.is_null() {
        return Ok(Vec::new());
    }
    unsafe { slice::from_raw_parts(values, count) }
        .iter()
        .map(word_timestamp)
        .collect()
}

fn collect_artifacts(
    values: *const audio_cpp_owned_artifact,
    count: usize,
) -> Result<Vec<VoiceArtifact>> {
    if values.is_null() {
        return Ok(Vec::new());
    }
    unsafe { slice::from_raw_parts(values, count) }
        .iter()
        .map(owned_artifact)
        .collect()
}

pub(crate) fn take_task_result(mut raw: audio_cpp_task_result) -> Result<TaskResult> {
    let result = (|| {
        Ok(TaskResult {
            audio: (raw.has_audio != 0).then(|| owned_audio(&raw.audio)),
            named_audio: collect_named_audio(raw.named_audio, raw.named_audio_count)?,
            text: if raw.has_text != 0 {
                Some(owned_transcript(&raw.text)?)
            } else {
                None
            },
            speech_segments: collect_segments(raw.speech_segments, raw.speech_segment_count)?,
            speaker_turns: collect_turns(raw.speaker_turns, raw.speaker_turn_count)?,
            word_timestamps: collect_words(raw.word_timestamps, raw.word_timestamp_count)?,
            artifact: if raw.has_artifact != 0 {
                Some(owned_artifact(&raw.artifact)?)
            } else {
                None
            },
            artifacts: collect_artifacts(raw.artifacts, raw.artifact_count)?,
        })
    })();
    unsafe { audio_cpp_task_result_free(&mut raw) };
    result
}

fn vad_event(value: &audio_cpp_vad_event) -> Result<VoiceActivityEvent> {
    Ok(VoiceActivityEvent {
        kind: VadEventKind::from_raw(value.kind),
        sample: value.sample,
        probability: value.probability,
        segment: if value.has_segment != 0 {
            Some(speech_segment(&value.segment)?)
        } else {
            None
        },
    })
}

pub(crate) fn copy_stream_event(raw: &audio_cpp_stream_event) -> Result<StreamEvent> {
    let voice_activity = if raw.voice_activity.is_null() {
        Vec::new()
    } else {
        unsafe { slice::from_raw_parts(raw.voice_activity, raw.voice_activity_count) }
            .iter()
            .map(vad_event)
            .collect::<Result<Vec<_>>>()?
    };
    Ok(StreamEvent {
        voice_activity,
        partial_text: if raw.has_partial_text != 0 {
            Some(owned_transcript(&raw.partial_text)?)
        } else {
            None
        },
        audio: (raw.has_audio != 0).then(|| owned_audio(&raw.audio)),
        named_audio: collect_named_audio(raw.named_audio, raw.named_audio_count)?,
        speaker_turns: collect_turns(raw.speaker_turns, raw.speaker_turn_count)?,
        word_timestamps: collect_words(raw.word_timestamps, raw.word_timestamp_count)?,
        artifacts: collect_artifacts(raw.artifacts, raw.artifact_count)?,
        is_final: raw.is_final != 0,
    })
}

pub(crate) fn take_stream_event(mut raw: audio_cpp_stream_event) -> Result<StreamEvent> {
    let result = copy_stream_event(&raw);
    unsafe { audio_cpp_stream_event_free(&mut raw) };
    result
}

fn named_assets(values: *const audio_cpp_named_asset, count: usize) -> Result<Vec<NamedAsset>> {
    if values.is_null() {
        return Ok(Vec::new());
    }
    unsafe { slice::from_raw_parts(values, count) }
        .iter()
        .map(|asset| {
            Ok(NamedAsset {
                id: copy_c_str(asset.id)?,
                path: PathBuf::from(copy_c_str(asset.path)?),
            })
        })
        .collect()
}

fn string_list(values: *mut *mut c_char, count: usize) -> Result<Vec<String>> {
    if values.is_null() {
        return Ok(Vec::new());
    }
    unsafe { slice::from_raw_parts(values, count) }
        .iter()
        .map(|item| copy_c_str(*item))
        .collect()
}

pub(crate) fn take_metadata(mut raw: audio_cpp_model_metadata) -> Result<ModelMetadata> {
    let result = (|| {
        Ok(ModelMetadata {
            family: copy_c_str(raw.family)?,
            variant: copy_c_str(raw.variant)?,
            description: copy_c_str(raw.description)?,
            config_candidates: string_list(raw.config_candidates, raw.config_candidate_count)?,
            weight_candidates: string_list(raw.weight_candidates, raw.weight_candidate_count)?,
        })
    })();
    unsafe { audio_cpp_model_metadata_free(&mut raw) };
    result
}

pub(crate) fn take_capabilities(mut raw: audio_cpp_capability_set) -> Result<CapabilitySet> {
    let result = (|| {
        let tasks = if raw.tasks.is_null() {
            Vec::new()
        } else {
            unsafe { slice::from_raw_parts(raw.tasks, raw.task_count) }
                .iter()
                .map(|task| {
                    let modes = if task.modes.is_null() {
                        Vec::new()
                    } else {
                        unsafe { slice::from_raw_parts(task.modes, task.mode_count) }
                            .iter()
                            .map(|mode| crate::types::RunMode::from_raw(*mode))
                            .collect::<Result<Vec<_>>>()?
                    };
                    Ok(TaskCapability {
                        task: VoiceTaskKind::from_raw(task.task)?,
                        modes,
                    })
                })
                .collect::<Result<Vec<_>>>()?
        };
        Ok(CapabilitySet {
            tasks,
            languages: string_list(raw.languages, raw.language_count)?,
            supports_speaker_reference: raw.supports_speaker_reference != 0,
            supports_style_condition: raw.supports_style_condition != 0,
            supports_timestamps: raw.supports_timestamps != 0,
        })
    })();
    unsafe { audio_cpp_capability_set_free(&mut raw) };
    result
}

pub(crate) fn take_inspection(mut raw: audio_cpp_inspection) -> Result<ModelInspection> {
    let result = (|| {
        Ok(ModelInspection {
            metadata: {
                let metadata = raw.metadata;
                raw.metadata = audio_cpp_model_metadata::default();
                take_metadata(metadata)?
            },
            capabilities: {
                let capabilities = raw.capabilities;
                raw.capabilities = audio_cpp_capability_set::default();
                take_capabilities(capabilities)?
            },
            model_root: PathBuf::from(copy_c_str(raw.model_root)?),
            discovered_configs: named_assets(raw.discovered_configs, raw.discovered_config_count)?,
            discovered_weights: named_assets(raw.discovered_weights, raw.discovered_weight_count)?,
        })
    })();
    unsafe { audio_cpp_inspection_free(&mut raw) };
    result
}

pub(crate) fn list_backend_devices() -> Result<Vec<BackendDevice>> {
    let mut devices = ptr::null_mut();
    let mut count = 0usize;
    check(unsafe { audio_cpp_list_backend_devices(&mut devices, &mut count) })?;
    let result = (|| {
        if devices.is_null() {
            return Ok(Vec::new());
        }
        unsafe { slice::from_raw_parts(devices, count) }
            .iter()
            .map(|device: &audio_cpp_backend_device| {
                Ok(BackendDevice {
                    backend: copy_c_str(device.backend)?,
                    index: device.index,
                    name: copy_c_str(device.name)?,
                    kind: copy_c_str(device.type_)?,
                })
            })
            .collect()
    })();
    unsafe { audio_cpp_backend_devices_free(devices, count) };
    result
}

pub(crate) fn streaming_policy_from_raw(
    policy: audio_cpp_sys::audio_cpp_streaming_policy,
) -> StreamingPolicy {
    StreamingPolicy {
        input: StreamingInputKind::from_raw(policy.input),
        output: StreamingOutputKind::from_raw(policy.output),
        preferred_audio_chunk_samples: policy.preferred_audio_chunk_samples,
        preferred_audio_chunk_seconds: policy.preferred_audio_chunk_seconds,
    }
}

pub(crate) struct CallbackState {
    pub callback: Box<dyn FnMut(&StreamEvent)>,
}

pub(crate) extern "C" fn stream_event_trampoline(
    event: *const audio_cpp_stream_event,
    user: *mut c_void,
) {
    if event.is_null() || user.is_null() {
        return;
    }
    let state = unsafe { &mut *user.cast::<CallbackState>() };
    if let Ok(converted) = copy_stream_event(unsafe { &*event }) {
        (state.callback)(&converted);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_load_request_paths() {
        let request = ModelLoadRequest::new("/tmp/model")
            .family_hint("silero_vad")
            .option("foo", "bar");
        let encoded = EncodedLoadRequest::new(&request).unwrap();
        assert!(!encoded.request.model_path.is_null());
        assert!(!encoded.request.family_hint.is_null());
        assert_eq!(encoded.request.option_count, 1);
    }

    #[test]
    fn encodes_typed_task_options() {
        let request = TaskRequest::new().options(crate::TaskOptions {
            temperature: Some(0.7),
            ..crate::TaskOptions::default()
        });
        let encoded = EncodedTaskRequest::new(&request).unwrap();
        assert_eq!(encoded.request.option_count, 1);
    }
}
