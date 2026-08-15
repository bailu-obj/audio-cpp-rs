#include "audio_cpp.h"

#include "engine/framework/core/backend.h"
#include "engine/framework/core/module.h"
#include "engine/framework/runtime/model.h"
#include "engine/framework/runtime/registry.h"
#include "engine/framework/runtime/session.h"

#include <cstring>
#include <exception>
#include <filesystem>
#include <new>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <unordered_map>
#include <utility>
#include <vector>

namespace {

thread_local std::string g_last_error;

char *dup_string(std::string_view value) {
    char *out = new char[value.size() + 1];
    std::memcpy(out, value.data(), value.size());
    out[value.size()] = '\0';
    return out;
}

void free_string(char *&value) {
    delete[] value;
    value = nullptr;
}

template <typename T>
T *alloc_n(size_t count) {
    if (count == 0) {
        return nullptr;
    }
    return new T[count]();
}

std::unordered_map<std::string, std::string> to_options(const audio_cpp_kv *options, size_t count) {
    std::unordered_map<std::string, std::string> out;
    if (options == nullptr) {
        return out;
    }
    for (size_t i = 0; i < count; ++i) {
        if (options[i].key == nullptr) {
            throw std::invalid_argument("option key must not be null");
        }
        out.emplace(options[i].key, options[i].value == nullptr ? "" : options[i].value);
    }
    return out;
}

audio_cpp_owned_kv *dup_owned_map(const std::unordered_map<std::string, std::string> &values, size_t &count) {
    count = values.size();
    if (count == 0) {
        return nullptr;
    }
    auto *out = alloc_n<audio_cpp_owned_kv>(count);
    size_t i = 0;
    for (const auto &entry : values) {
        out[i].key = dup_string(entry.first);
        out[i].value = dup_string(entry.second);
        ++i;
    }
    return out;
}

void free_owned_kvs(audio_cpp_owned_kv *&values, size_t &count) {
    if (values != nullptr) {
        for (size_t i = 0; i < count; ++i) {
            free_string(values[i].key);
            free_string(values[i].value);
        }
        delete[] values;
    }
    values = nullptr;
    count = 0;
}

char **dup_string_list(const std::vector<std::string> &values, size_t &count) {
    count = values.size();
    if (count == 0) {
        return nullptr;
    }
    auto **out = alloc_n<char *>(count);
    for (size_t i = 0; i < count; ++i) {
        out[i] = dup_string(values[i]);
    }
    return out;
}

void free_string_list(char **&values, size_t &count) {
    if (values != nullptr) {
        for (size_t i = 0; i < count; ++i) {
            free_string(values[i]);
        }
        delete[] values;
    }
    values = nullptr;
    count = 0;
}

engine::runtime::AudioBuffer to_audio_buffer(const audio_cpp_audio_view &view) {
    engine::runtime::AudioBuffer audio;
    audio.sample_rate = view.sample_rate;
    audio.channels = view.channels;
    if (view.samples != nullptr && view.sample_count > 0) {
        audio.samples.assign(view.samples, view.samples + view.sample_count);
    }
    return audio;
}

void fill_owned_audio(audio_cpp_owned_audio &out, const engine::runtime::AudioBuffer &audio) {
    out.sample_rate = audio.sample_rate;
    out.channels = audio.channels;
    out.sample_count = audio.samples.size();
    if (out.sample_count == 0) {
        out.samples = nullptr;
        return;
    }
    out.samples = alloc_n<float>(out.sample_count);
    std::memcpy(out.samples, audio.samples.data(), out.sample_count * sizeof(float));
}

void free_owned_audio(audio_cpp_owned_audio &audio) {
    delete[] audio.samples;
    audio.samples = nullptr;
    audio.sample_count = 0;
    audio.sample_rate = 0;
    audio.channels = 0;
}

void fill_owned_transcript(audio_cpp_owned_transcript &out, const engine::runtime::Transcript &text) {
    out.text = dup_string(text.text);
    out.language = dup_string(text.language);
}

void free_owned_transcript(audio_cpp_owned_transcript &text) {
    free_string(text.text);
    free_string(text.language);
}

engine::runtime::VoiceReference to_voice_reference(const audio_cpp_voice_reference &value) {
    engine::runtime::VoiceReference out;
    if (value.has_audio != 0) {
        out.audio = to_audio_buffer(value.audio);
    }
    if (value.cached_voice_id != nullptr) {
        out.cached_voice_id = std::string(value.cached_voice_id);
    }
    return out;
}

engine::runtime::StyleCondition to_style_condition(const audio_cpp_style_condition &value) {
    engine::runtime::StyleCondition out;
    if (value.language != nullptr) {
        out.language = std::string(value.language);
    }
    if (value.emotion != nullptr) {
        out.emotion = std::string(value.emotion);
    }
    if (value.has_speaking_rate != 0) {
        out.speaking_rate = value.speaking_rate;
    }
    if (value.has_pitch_shift != 0) {
        out.pitch_shift = value.pitch_shift;
    }
    if (value.has_energy_scale != 0) {
        out.energy_scale = value.energy_scale;
    }
    out.tags = to_options(value.tags, value.tag_count);
    return out;
}

engine::runtime::VoiceCondition to_voice_condition(const audio_cpp_voice_condition &value) {
    engine::runtime::VoiceCondition out;
    if (value.has_speaker != 0) {
        out.speaker = to_voice_reference(value.speaker);
    }
    if (value.has_style != 0) {
        out.style = to_style_condition(value.style);
    }
    return out;
}

engine::runtime::VoiceArtifact to_artifact(const audio_cpp_artifact_view &value) {
    engine::runtime::VoiceArtifact artifact;
    artifact.kind = static_cast<engine::runtime::ArtifactKind>(value.kind);
    artifact.id = value.id == nullptr ? std::string() : std::string(value.id);
    if (value.payload != nullptr && value.payload_size > 0) {
        artifact.payload.resize(value.payload_size);
        std::memcpy(artifact.payload.data(), value.payload, value.payload_size);
    }
    artifact.meta = to_options(value.meta, value.meta_count);
    return artifact;
}

void fill_owned_artifact(audio_cpp_owned_artifact &out, const engine::runtime::VoiceArtifact &artifact) {
    out.kind = static_cast<audio_cpp_artifact_kind>(artifact.kind);
    out.id = dup_string(artifact.id);
    out.payload_size = artifact.payload.size();
    if (out.payload_size == 0) {
        out.payload = nullptr;
    } else {
        out.payload = alloc_n<uint8_t>(out.payload_size);
        std::memcpy(out.payload, artifact.payload.data(), out.payload_size);
    }
    out.meta = dup_owned_map(artifact.meta, out.meta_count);
}

void free_owned_artifact(audio_cpp_owned_artifact &artifact) {
    free_string(artifact.id);
    delete[] artifact.payload;
    artifact.payload = nullptr;
    artifact.payload_size = 0;
    free_owned_kvs(artifact.meta, artifact.meta_count);
}

engine::runtime::TaskRequest to_task_request(const audio_cpp_task_request &request) {
    engine::runtime::TaskRequest out;
    if (request.has_text != 0) {
        out.text_input = engine::runtime::Transcript{
            request.text.text == nullptr ? std::string() : std::string(request.text.text),
            request.text.language == nullptr ? std::string() : std::string(request.text.language),
        };
    }
    if (request.has_audio != 0) {
        out.audio_input = to_audio_buffer(request.audio);
    }
    if (request.has_voice != 0) {
        out.voice = to_voice_condition(request.voice);
    }
    if (request.artifacts != nullptr) {
        out.input_artifacts.reserve(request.artifact_count);
        for (size_t i = 0; i < request.artifact_count; ++i) {
            out.input_artifacts.push_back(to_artifact(request.artifacts[i]));
        }
    }
    out.options = to_options(request.options, request.option_count);
    return out;
}

engine::runtime::SessionPreparationRequest to_prep_request(const audio_cpp_session_prep_request &request) {
    engine::runtime::SessionPreparationRequest out;
    if (request.has_audio != 0) {
        out.audio = engine::runtime::AudioPreparationContract{
            request.audio_sample_rate,
            request.audio_channels,
            request.max_input_samples,
        };
    }
    if (request.has_text != 0) {
        out.text = engine::runtime::Transcript{
            request.text.text == nullptr ? std::string() : std::string(request.text.text),
            request.text.language == nullptr ? std::string() : std::string(request.text.language),
        };
    }
    if (request.has_voice != 0) {
        out.voice = to_voice_condition(request.voice);
    }
    out.options = to_options(request.options, request.option_count);
    return out;
}

engine::runtime::ModelLoadRequest to_load_request(const audio_cpp_model_load_request &request) {
    if (request.model_path == nullptr || request.model_path[0] == '\0') {
        throw std::invalid_argument("model_path is required");
    }
    engine::runtime::ModelLoadRequest out;
    out.model_path = request.model_path;
    if (request.model_spec_override != nullptr && request.model_spec_override[0] != '\0') {
        out.model_spec_override = request.model_spec_override;
    }
    if (request.family_hint != nullptr && request.family_hint[0] != '\0') {
        out.family_hint = request.family_hint;
    }
    if (request.config_id != nullptr && request.config_id[0] != '\0') {
        out.config_id = request.config_id;
    }
    if (request.weight_id != nullptr && request.weight_id[0] != '\0') {
        out.weight_id = request.weight_id;
    }
    out.options = to_options(request.options, request.option_count);
    return out;
}

void fill_named_audio(audio_cpp_named_audio &out, const engine::runtime::NamedAudioBuffer &value) {
    out.id = dup_string(value.id);
    fill_owned_audio(out.audio, value.audio);
    out.meta = dup_owned_map(value.meta, out.meta_count);
}

void free_named_audio(audio_cpp_named_audio &value) {
    free_string(value.id);
    free_owned_audio(value.audio);
    free_owned_kvs(value.meta, value.meta_count);
}

void fill_speech_segment(audio_cpp_speech_segment &out, const engine::runtime::SpeechSegment &value) {
    out.start_sample = value.span.start_sample;
    out.end_sample = value.span.end_sample;
    out.confidence = value.confidence;
    out.text = dup_string(value.text);
}

void free_speech_segment(audio_cpp_speech_segment &value) {
    free_string(value.text);
}

void fill_speaker_turn(audio_cpp_speaker_turn &out, const engine::runtime::SpeakerTurn &value) {
    out.start_sample = value.span.start_sample;
    out.end_sample = value.span.end_sample;
    out.speaker_id = dup_string(value.speaker_id);
    out.confidence = value.confidence;
    out.text = dup_string(value.text);
}

void free_speaker_turn(audio_cpp_speaker_turn &value) {
    free_string(value.speaker_id);
    free_string(value.text);
}

void fill_word_timestamp(audio_cpp_word_timestamp &out, const engine::runtime::WordTimestamp &value) {
    out.start_sample = value.span.start_sample;
    out.end_sample = value.span.end_sample;
    out.word = dup_string(value.word);
    out.confidence = value.confidence;
}

void free_word_timestamp(audio_cpp_word_timestamp &value) {
    free_string(value.word);
}

void fill_task_result(audio_cpp_task_result &out, const engine::runtime::TaskResult &result) {
    out = audio_cpp_task_result{};
    if (result.audio_output.has_value()) {
        fill_owned_audio(out.audio, *result.audio_output);
        out.has_audio = 1;
    }
    out.named_audio_count = result.named_audio_outputs.size();
    out.named_audio = alloc_n<audio_cpp_named_audio>(out.named_audio_count);
    for (size_t i = 0; i < out.named_audio_count; ++i) {
        fill_named_audio(out.named_audio[i], result.named_audio_outputs[i]);
    }
    if (result.text_output.has_value()) {
        fill_owned_transcript(out.text, *result.text_output);
        out.has_text = 1;
    }
    out.speech_segment_count = result.speech_segments.size();
    out.speech_segments = alloc_n<audio_cpp_speech_segment>(out.speech_segment_count);
    for (size_t i = 0; i < out.speech_segment_count; ++i) {
        fill_speech_segment(out.speech_segments[i], result.speech_segments[i]);
    }
    out.speaker_turn_count = result.speaker_turns.size();
    out.speaker_turns = alloc_n<audio_cpp_speaker_turn>(out.speaker_turn_count);
    for (size_t i = 0; i < out.speaker_turn_count; ++i) {
        fill_speaker_turn(out.speaker_turns[i], result.speaker_turns[i]);
    }
    out.word_timestamp_count = result.word_timestamps.size();
    out.word_timestamps = alloc_n<audio_cpp_word_timestamp>(out.word_timestamp_count);
    for (size_t i = 0; i < out.word_timestamp_count; ++i) {
        fill_word_timestamp(out.word_timestamps[i], result.word_timestamps[i]);
    }
    if (result.artifact_output.has_value()) {
        fill_owned_artifact(out.artifact, *result.artifact_output);
        out.has_artifact = 1;
    }
    out.artifact_count = result.output_artifacts.size();
    out.artifacts = alloc_n<audio_cpp_owned_artifact>(out.artifact_count);
    for (size_t i = 0; i < out.artifact_count; ++i) {
        fill_owned_artifact(out.artifacts[i], result.output_artifacts[i]);
    }
}

void fill_vad_event(audio_cpp_vad_event &out, const engine::runtime::VoiceActivityEvent &value) {
    out.kind = static_cast<audio_cpp_vad_event_kind>(value.kind);
    out.sample = value.sample;
    out.probability = value.probability;
    if (value.segment.has_value()) {
        fill_speech_segment(out.segment, *value.segment);
        out.has_segment = 1;
    }
}

void fill_stream_event(audio_cpp_stream_event &out, const engine::runtime::StreamEvent &event) {
    out = audio_cpp_stream_event{};
    out.voice_activity_count = event.voice_activity.size();
    out.voice_activity = alloc_n<audio_cpp_vad_event>(out.voice_activity_count);
    for (size_t i = 0; i < out.voice_activity_count; ++i) {
        fill_vad_event(out.voice_activity[i], event.voice_activity[i]);
    }
    if (event.partial_text.has_value()) {
        fill_owned_transcript(out.partial_text, *event.partial_text);
        out.has_partial_text = 1;
    }
    if (event.audio_output.has_value()) {
        fill_owned_audio(out.audio, *event.audio_output);
        out.has_audio = 1;
    }
    out.named_audio_count = event.named_audio_outputs.size();
    out.named_audio = alloc_n<audio_cpp_named_audio>(out.named_audio_count);
    for (size_t i = 0; i < out.named_audio_count; ++i) {
        fill_named_audio(out.named_audio[i], event.named_audio_outputs[i]);
    }
    out.speaker_turn_count = event.speaker_turns.size();
    out.speaker_turns = alloc_n<audio_cpp_speaker_turn>(out.speaker_turn_count);
    for (size_t i = 0; i < out.speaker_turn_count; ++i) {
        fill_speaker_turn(out.speaker_turns[i], event.speaker_turns[i]);
    }
    out.word_timestamp_count = event.word_timestamps.size();
    out.word_timestamps = alloc_n<audio_cpp_word_timestamp>(out.word_timestamp_count);
    for (size_t i = 0; i < out.word_timestamp_count; ++i) {
        fill_word_timestamp(out.word_timestamps[i], event.word_timestamps[i]);
    }
    out.artifact_count = event.output_artifacts.size();
    out.artifacts = alloc_n<audio_cpp_owned_artifact>(out.artifact_count);
    for (size_t i = 0; i < out.artifact_count; ++i) {
        fill_owned_artifact(out.artifacts[i], event.output_artifacts[i]);
    }
    out.is_final = event.is_final ? 1 : 0;
}

void fill_metadata(audio_cpp_model_metadata &out, const engine::runtime::ModelMetadata &metadata) {
    out = audio_cpp_model_metadata{};
    out.family = dup_string(metadata.family);
    out.variant = dup_string(metadata.variant);
    out.description = dup_string(metadata.description);
    out.config_candidates = dup_string_list(metadata.config_candidates, out.config_candidate_count);
    out.weight_candidates = dup_string_list(metadata.weight_candidates, out.weight_candidate_count);
}

void fill_capabilities(audio_cpp_capability_set &out, const engine::runtime::CapabilitySet &capabilities) {
    out = audio_cpp_capability_set{};
    out.task_count = capabilities.supported_tasks.size();
    out.tasks = alloc_n<audio_cpp_task_capability>(out.task_count);
    for (size_t i = 0; i < out.task_count; ++i) {
        const auto &task = capabilities.supported_tasks[i];
        out.tasks[i].task = static_cast<audio_cpp_voice_task_kind>(task.task);
        out.tasks[i].mode_count = task.modes.size();
        out.tasks[i].modes = alloc_n<audio_cpp_run_mode>(out.tasks[i].mode_count);
        for (size_t j = 0; j < out.tasks[i].mode_count; ++j) {
            out.tasks[i].modes[j] = static_cast<audio_cpp_run_mode>(task.modes[j]);
        }
    }
    out.languages = dup_string_list(capabilities.languages, out.language_count);
    out.supports_speaker_reference = capabilities.supports_speaker_reference ? 1 : 0;
    out.supports_style_condition = capabilities.supports_style_condition ? 1 : 0;
    out.supports_timestamps = capabilities.supports_timestamps ? 1 : 0;
}

void fill_named_assets(
    audio_cpp_named_asset *&out,
    size_t &count,
    const std::vector<engine::runtime::NamedAsset> &assets) {
    count = assets.size();
    out = alloc_n<audio_cpp_named_asset>(count);
    for (size_t i = 0; i < count; ++i) {
        out[i].id = dup_string(assets[i].id);
        out[i].path = dup_string(assets[i].path.string());
    }
}

void free_named_assets(audio_cpp_named_asset *&assets, size_t &count) {
    if (assets != nullptr) {
        for (size_t i = 0; i < count; ++i) {
            free_string(assets[i].id);
            free_string(assets[i].path);
        }
        delete[] assets;
    }
    assets = nullptr;
    count = 0;
}

audio_cpp_status set_error(audio_cpp_status status, const char *message) {
    g_last_error = message == nullptr ? "" : message;
    return status;
}

audio_cpp_status map_exception_message(const char *message) {
    const std::string text = message == nullptr ? "" : message;
    const auto contains = [&](const char *needle) {
        return text.find(needle) != std::string::npos;
    };
    if (contains("not found") || contains("unknown family") || contains("no matching")) {
        return set_error(AUDIO_CPP_ERR_NOT_FOUND, message);
    }
    if (contains("unsupported") || contains("not supported")) {
        return set_error(AUDIO_CPP_ERR_UNSUPPORTED, message);
    }
    return set_error(AUDIO_CPP_ERR_RUNTIME, message);
}

template <typename Fn>
audio_cpp_status catch_status(Fn &&fn) {
    try {
        g_last_error.clear();
        fn();
        return AUDIO_CPP_OK;
    } catch (const std::invalid_argument &error) {
        return set_error(AUDIO_CPP_ERR_INVALID_ARG, error.what());
    } catch (const std::bad_alloc &error) {
        return set_error(AUDIO_CPP_ERR_OOM, error.what());
    } catch (const std::exception &error) {
        return map_exception_message(error.what());
    } catch (...) {
        return set_error(AUDIO_CPP_ERR_RUNTIME, "unknown native error");
    }
}

engine::core::BackendType to_backend_type(audio_cpp_backend_type type) {
    return static_cast<engine::core::BackendType>(type);
}

engine::runtime::TaskSpec to_task_spec(audio_cpp_task_spec spec) {
    engine::runtime::TaskSpec out;
    out.task = static_cast<engine::runtime::VoiceTaskKind>(spec.task);
    out.mode = static_cast<engine::runtime::RunMode>(spec.mode);
    return out;
}

engine::runtime::SessionOptions to_session_options(audio_cpp_session_options options) {
    engine::runtime::SessionOptions out;
    out.backend.type = to_backend_type(options.backend.type);
    out.backend.device = options.backend.device;
    out.backend.threads = options.backend.threads <= 0 ? 1 : options.backend.threads;
    out.options = to_options(options.options, options.option_count);
    return out;
}

}  // namespace

struct audio_cpp_registry {
    engine::runtime::ModelRegistry registry;
};

struct audio_cpp_model {
    std::unique_ptr<engine::runtime::ILoadedVoiceModel> model;
};

struct audio_cpp_session {
    std::unique_ptr<engine::runtime::IVoiceTaskSession> session;
    engine::runtime::IOfflineVoiceTaskSession *offline = nullptr;
    engine::runtime::IStreamingVoiceTaskSession *streaming = nullptr;
    std::string family;
    audio_cpp_stream_event_fn callback = nullptr;
    void *callback_user = nullptr;
};

extern "C" {

const char *audio_cpp_last_error(void) {
    return g_last_error.c_str();
}

const char *audio_cpp_voice_task_kind_name(audio_cpp_voice_task_kind kind) {
    return engine::runtime::to_string(static_cast<engine::runtime::VoiceTaskKind>(kind));
}

audio_cpp_status audio_cpp_voice_task_kind_parse(const char *value, audio_cpp_voice_task_kind *out) {
    return catch_status([&] {
        if (value == nullptr || out == nullptr) {
            throw std::invalid_argument("value and out are required");
        }
        *out = static_cast<audio_cpp_voice_task_kind>(engine::runtime::parse_voice_task_kind(value));
    });
}

const char *audio_cpp_run_mode_name(audio_cpp_run_mode mode) {
    return engine::runtime::to_string(static_cast<engine::runtime::RunMode>(mode));
}

audio_cpp_status audio_cpp_run_mode_parse(const char *value, audio_cpp_run_mode *out) {
    return catch_status([&] {
        if (value == nullptr || out == nullptr) {
            throw std::invalid_argument("value and out are required");
        }
        *out = static_cast<audio_cpp_run_mode>(engine::runtime::parse_run_mode(value));
    });
}

const char *audio_cpp_backend_type_name(audio_cpp_backend_type type) {
    switch (type) {
    case AUDIO_CPP_BACKEND_CPU:
        return "cpu";
    case AUDIO_CPP_BACKEND_CUDA:
        return "cuda";
    case AUDIO_CPP_BACKEND_HIP:
        return "hip";
    case AUDIO_CPP_BACKEND_VULKAN:
        return "vulkan";
    case AUDIO_CPP_BACKEND_METAL:
        return "metal";
    case AUDIO_CPP_BACKEND_BEST_AVAILABLE:
        return "best";
    }
    return "unknown";
}

audio_cpp_status audio_cpp_backend_type_parse(const char *value, audio_cpp_backend_type *out) {
    return catch_status([&] {
        if (value == nullptr || out == nullptr) {
            throw std::invalid_argument("value and out are required");
        }
        const std::string text = value;
        if (text == "cpu") {
            *out = AUDIO_CPP_BACKEND_CPU;
        } else if (text == "cuda") {
            *out = AUDIO_CPP_BACKEND_CUDA;
        } else if (text == "hip" || text == "rocm") {
            *out = AUDIO_CPP_BACKEND_HIP;
        } else if (text == "vulkan") {
            *out = AUDIO_CPP_BACKEND_VULKAN;
        } else if (text == "metal") {
            *out = AUDIO_CPP_BACKEND_METAL;
        } else if (text == "best") {
            *out = AUDIO_CPP_BACKEND_BEST_AVAILABLE;
        } else {
            throw std::runtime_error("unsupported backend: " + text);
        }
    });
}

const char *audio_cpp_artifact_kind_name(audio_cpp_artifact_kind kind) {
    switch (kind) {
    case AUDIO_CPP_ARTIFACT_SPEAKER_EMBEDDING:
        return "speaker_embedding";
    case AUDIO_CPP_ARTIFACT_STYLE_EMBEDDING:
        return "style_embedding";
    case AUDIO_CPP_ARTIFACT_PROMPT_EMBEDDING:
        return "prompt_embedding";
    case AUDIO_CPP_ARTIFACT_ACOUSTIC_TOKENS:
        return "acoustic_tokens";
    case AUDIO_CPP_ARTIFACT_MIDI:
        return "midi";
    case AUDIO_CPP_ARTIFACT_TRANSCRIPT_ALIGNMENT:
        return "transcript_alignment";
    case AUDIO_CPP_ARTIFACT_DIARIZATION_STATE:
        return "diarization_state";
    case AUDIO_CPP_ARTIFACT_VAD_STATE:
        return "vad_state";
    case AUDIO_CPP_ARTIFACT_CUSTOM:
        return "custom";
    }
    return "custom";
}

audio_cpp_status audio_cpp_artifact_kind_parse(const char *value, audio_cpp_artifact_kind *out) {
    return catch_status([&] {
        if (value == nullptr || out == nullptr) {
            throw std::invalid_argument("value and out are required");
        }
        const std::string text = value;
        if (text == "speaker_embedding") {
            *out = AUDIO_CPP_ARTIFACT_SPEAKER_EMBEDDING;
        } else if (text == "style_embedding") {
            *out = AUDIO_CPP_ARTIFACT_STYLE_EMBEDDING;
        } else if (text == "prompt_embedding") {
            *out = AUDIO_CPP_ARTIFACT_PROMPT_EMBEDDING;
        } else if (text == "acoustic_tokens") {
            *out = AUDIO_CPP_ARTIFACT_ACOUSTIC_TOKENS;
        } else if (text == "midi") {
            *out = AUDIO_CPP_ARTIFACT_MIDI;
        } else if (text == "transcript_alignment") {
            *out = AUDIO_CPP_ARTIFACT_TRANSCRIPT_ALIGNMENT;
        } else if (text == "diarization_state") {
            *out = AUDIO_CPP_ARTIFACT_DIARIZATION_STATE;
        } else if (text == "vad_state") {
            *out = AUDIO_CPP_ARTIFACT_VAD_STATE;
        } else if (text == "custom") {
            *out = AUDIO_CPP_ARTIFACT_CUSTOM;
        } else {
            throw std::runtime_error("unsupported artifact kind: " + text);
        }
    });
}

audio_cpp_status audio_cpp_list_backend_devices(audio_cpp_backend_device **out, size_t *count) {
    return catch_status([&] {
        if (out == nullptr || count == nullptr) {
            throw std::invalid_argument("out and count are required");
        }
        const auto devices = engine::core::list_backend_devices();
        *count = devices.size();
        *out = alloc_n<audio_cpp_backend_device>(*count);
        for (size_t i = 0; i < *count; ++i) {
            (*out)[i].backend = dup_string(devices[i].backend);
            (*out)[i].index = devices[i].index;
            (*out)[i].name = dup_string(devices[i].name);
            (*out)[i].type = dup_string(devices[i].type);
        }
    });
}

void audio_cpp_backend_devices_free(audio_cpp_backend_device *devices, size_t count) {
    if (devices == nullptr) {
        return;
    }
    for (size_t i = 0; i < count; ++i) {
        free_string(devices[i].backend);
        free_string(devices[i].name);
        free_string(devices[i].type);
    }
    delete[] devices;
}

void audio_cpp_string_list_free(char **items, size_t count) {
    if (items == nullptr) {
        return;
    }
    for (size_t i = 0; i < count; ++i) {
        delete[] items[i];
    }
    delete[] items;
}

audio_cpp_status audio_cpp_registry_create_default(const char *config_path, audio_cpp_registry **out) {
    return catch_status([&] {
        if (out == nullptr) {
            throw std::invalid_argument("out is required");
        }
        std::optional<std::filesystem::path> path;
        if (config_path != nullptr && config_path[0] != '\0') {
            path = config_path;
        }
        auto *registry = new audio_cpp_registry();
        registry->registry = engine::runtime::make_default_registry(path);
        *out = registry;
    });
}

void audio_cpp_registry_destroy(audio_cpp_registry *registry) {
    delete registry;
}

size_t audio_cpp_registry_size(const audio_cpp_registry *registry) {
    return registry == nullptr ? 0 : registry->registry.size();
}

uint8_t audio_cpp_registry_empty(const audio_cpp_registry *registry) {
    return registry == nullptr || registry->registry.empty() ? 1 : 0;
}

uint8_t audio_cpp_registry_supports_family(const audio_cpp_registry *registry, const char *family) {
    if (registry == nullptr || family == nullptr) {
        return 0;
    }
    return registry->registry.supports_family(family) ? 1 : 0;
}

audio_cpp_status audio_cpp_registry_families(
    const audio_cpp_registry *registry,
    char ***out,
    size_t *count) {
    return catch_status([&] {
        if (registry == nullptr || out == nullptr || count == nullptr) {
            throw std::invalid_argument("registry, out, and count are required");
        }
        *out = dup_string_list(registry->registry.families(), *count);
    });
}

audio_cpp_status audio_cpp_registry_inspect(
    const audio_cpp_registry *registry,
    const audio_cpp_model_load_request *request,
    audio_cpp_inspection *out) {
    return catch_status([&] {
        if (registry == nullptr || request == nullptr || out == nullptr) {
            throw std::invalid_argument("registry, request, and out are required");
        }
        const auto inspection = registry->registry.inspect(to_load_request(*request));
        *out = audio_cpp_inspection{};
        fill_metadata(out->metadata, inspection.metadata);
        fill_capabilities(out->capabilities, inspection.capabilities);
        out->model_root = dup_string(inspection.model_root.string());
        fill_named_assets(out->discovered_configs, out->discovered_config_count, inspection.discovered_configs);
        fill_named_assets(out->discovered_weights, out->discovered_weight_count, inspection.discovered_weights);
    });
}

audio_cpp_status audio_cpp_registry_load(
    const audio_cpp_registry *registry,
    const audio_cpp_model_load_request *request,
    audio_cpp_model **out) {
    return catch_status([&] {
        if (registry == nullptr || request == nullptr || out == nullptr) {
            throw std::invalid_argument("registry, request, and out are required");
        }
        auto *model = new audio_cpp_model();
        model->model = registry->registry.load(to_load_request(*request));
        *out = model;
    });
}

void audio_cpp_model_metadata_free(audio_cpp_model_metadata *metadata) {
    if (metadata == nullptr) {
        return;
    }
    free_string(metadata->family);
    free_string(metadata->variant);
    free_string(metadata->description);
    free_string_list(metadata->config_candidates, metadata->config_candidate_count);
    free_string_list(metadata->weight_candidates, metadata->weight_candidate_count);
    *metadata = audio_cpp_model_metadata{};
}

void audio_cpp_capability_set_free(audio_cpp_capability_set *capabilities) {
    if (capabilities == nullptr) {
        return;
    }
    if (capabilities->tasks != nullptr) {
        for (size_t i = 0; i < capabilities->task_count; ++i) {
            delete[] capabilities->tasks[i].modes;
        }
        delete[] capabilities->tasks;
    }
    free_string_list(capabilities->languages, capabilities->language_count);
    *capabilities = audio_cpp_capability_set{};
}

void audio_cpp_inspection_free(audio_cpp_inspection *inspection) {
    if (inspection == nullptr) {
        return;
    }
    audio_cpp_model_metadata_free(&inspection->metadata);
    audio_cpp_capability_set_free(&inspection->capabilities);
    free_string(inspection->model_root);
    free_named_assets(inspection->discovered_configs, inspection->discovered_config_count);
    free_named_assets(inspection->discovered_weights, inspection->discovered_weight_count);
    *inspection = audio_cpp_inspection{};
}

void audio_cpp_model_destroy(audio_cpp_model *model) {
    delete model;
}

audio_cpp_status audio_cpp_model_get_metadata(const audio_cpp_model *model, audio_cpp_model_metadata *out) {
    return catch_status([&] {
        if (model == nullptr || model->model == nullptr || out == nullptr) {
            throw std::invalid_argument("model and out are required");
        }
        fill_metadata(*out, model->model->metadata());
    });
}

audio_cpp_status audio_cpp_model_capabilities(
    const audio_cpp_model *model,
    audio_cpp_capability_set *out) {
    return catch_status([&] {
        if (model == nullptr || model->model == nullptr || out == nullptr) {
            throw std::invalid_argument("model and out are required");
        }
        fill_capabilities(*out, model->model->capabilities());
    });
}

audio_cpp_status audio_cpp_model_create_session(
    const audio_cpp_model *model,
    audio_cpp_task_spec spec,
    audio_cpp_session_options options,
    audio_cpp_session **out) {
    return catch_status([&] {
        if (model == nullptr || model->model == nullptr || out == nullptr) {
            throw std::invalid_argument("model and out are required");
        }
        auto session = model->model->create_task_session(to_task_spec(spec), to_session_options(options));
        if (session == nullptr) {
            throw std::runtime_error("create_task_session returned null");
        }
        auto *wrapper = new audio_cpp_session();
        wrapper->family = session->family();
        wrapper->offline = dynamic_cast<engine::runtime::IOfflineVoiceTaskSession *>(session.get());
        wrapper->streaming = dynamic_cast<engine::runtime::IStreamingVoiceTaskSession *>(session.get());
        wrapper->session = std::move(session);
        *out = wrapper;
    });
}

void audio_cpp_session_destroy(audio_cpp_session *session) {
    delete session;
}

audio_cpp_status audio_cpp_session_family(const audio_cpp_session *session, const char **out) {
    return catch_status([&] {
        if (session == nullptr || out == nullptr) {
            throw std::invalid_argument("session and out are required");
        }
        *out = session->family.c_str();
    });
}

audio_cpp_status audio_cpp_session_task_kind(
    const audio_cpp_session *session,
    audio_cpp_voice_task_kind *out) {
    return catch_status([&] {
        if (session == nullptr || session->session == nullptr || out == nullptr) {
            throw std::invalid_argument("session and out are required");
        }
        *out = static_cast<audio_cpp_voice_task_kind>(session->session->task_kind());
    });
}

audio_cpp_status audio_cpp_session_run_mode(const audio_cpp_session *session, audio_cpp_run_mode *out) {
    return catch_status([&] {
        if (session == nullptr || session->session == nullptr || out == nullptr) {
            throw std::invalid_argument("session and out are required");
        }
        *out = static_cast<audio_cpp_run_mode>(session->session->run_mode());
    });
}

uint8_t audio_cpp_session_supports_offline(const audio_cpp_session *session) {
    return session != nullptr && session->offline != nullptr ? 1 : 0;
}

uint8_t audio_cpp_session_supports_streaming(const audio_cpp_session *session) {
    return session != nullptr && session->streaming != nullptr ? 1 : 0;
}

audio_cpp_status audio_cpp_build_prep_from_request(
    const audio_cpp_task_request *request,
    audio_cpp_session_prep_request *out) {
    return catch_status([&] {
        if (request == nullptr || out == nullptr) {
            throw std::invalid_argument("request and out are required");
        }
        const auto prep = engine::runtime::build_preparation_request(to_task_request(*request));
        *out = audio_cpp_session_prep_request{};
        if (prep.audio.has_value()) {
            out->audio_sample_rate = prep.audio->sample_rate;
            out->audio_channels = prep.audio->channels;
            out->max_input_samples = prep.audio->max_input_samples;
            out->has_audio = 1;
        }
        if (prep.text.has_value()) {
            out->text.text = request->has_text != 0 ? request->text.text : nullptr;
            out->text.language = request->has_text != 0 ? request->text.language : nullptr;
            out->has_text = 1;
        }
        if (prep.voice.has_value()) {
            out->voice = request->voice;
            out->has_voice = request->has_voice;
        }
        out->options = request->options;
        out->option_count = request->option_count;
    });
}

audio_cpp_status audio_cpp_session_prepare(
    audio_cpp_session *session,
    const audio_cpp_session_prep_request *request) {
    return catch_status([&] {
        if (session == nullptr || session->session == nullptr || request == nullptr) {
            throw std::invalid_argument("session and request are required");
        }
        session->session->prepare(to_prep_request(*request));
    });
}

audio_cpp_status audio_cpp_session_run(
    audio_cpp_session *session,
    const audio_cpp_task_request *request,
    audio_cpp_task_result *out) {
    return catch_status([&] {
        if (session == nullptr || session->offline == nullptr || request == nullptr || out == nullptr) {
            throw std::invalid_argument("offline session, request, and out are required");
        }
        fill_task_result(*out, session->offline->run(to_task_request(*request)));
    });
}

audio_cpp_status audio_cpp_session_streaming_policy(
    const audio_cpp_session *session,
    audio_cpp_streaming_policy *out) {
    return catch_status([&] {
        if (session == nullptr || session->streaming == nullptr || out == nullptr) {
            throw std::invalid_argument("streaming session and out are required");
        }
        const auto policy = session->streaming->streaming_policy();
        out->input = static_cast<audio_cpp_streaming_input_kind>(policy.input);
        out->output = static_cast<audio_cpp_streaming_output_kind>(policy.output);
        out->preferred_audio_chunk_samples = policy.preferred_audio_chunk_samples;
        out->preferred_audio_chunk_seconds = policy.preferred_audio_chunk_seconds;
    });
}

audio_cpp_status audio_cpp_session_set_stream_callback(
    audio_cpp_session *session,
    audio_cpp_stream_event_fn callback,
    void *user_data) {
    return catch_status([&] {
        if (session == nullptr || session->streaming == nullptr) {
            throw std::invalid_argument("streaming session is required");
        }
        session->callback = callback;
        session->callback_user = user_data;
        if (callback == nullptr) {
            session->streaming->set_stream_event_sink(nullptr);
            return;
        }
        session->streaming->set_stream_event_sink([session](const engine::runtime::StreamEvent &event) {
            audio_cpp_stream_event converted{};
            fill_stream_event(converted, event);
            session->callback(&converted, session->callback_user);
            audio_cpp_stream_event_free(&converted);
        });
    });
}

audio_cpp_status audio_cpp_session_start_stream(
    audio_cpp_session *session,
    const audio_cpp_task_request *request) {
    return catch_status([&] {
        if (session == nullptr || session->streaming == nullptr || request == nullptr) {
            throw std::invalid_argument("streaming session and request are required");
        }
        session->streaming->start_stream(to_task_request(*request));
    });
}

audio_cpp_status audio_cpp_session_process_audio_chunk(
    audio_cpp_session *session,
    const audio_cpp_audio_chunk *chunk,
    audio_cpp_stream_event *out) {
    return catch_status([&] {
        if (session == nullptr || session->streaming == nullptr || chunk == nullptr || out == nullptr) {
            throw std::invalid_argument("streaming session, chunk, and out are required");
        }
        engine::runtime::AudioChunk native;
        native.sample_rate = chunk->sample_rate;
        native.channels = chunk->channels;
        native.start_sample = chunk->start_sample;
        if (chunk->samples != nullptr && chunk->sample_count > 0) {
            native.samples.assign(chunk->samples, chunk->samples + chunk->sample_count);
        }
        fill_stream_event(*out, session->streaming->process_audio_chunk(native));
    });
}

audio_cpp_status audio_cpp_session_next_stream_event(
    audio_cpp_session *session,
    audio_cpp_stream_event *out,
    uint8_t *has_event) {
    return catch_status([&] {
        if (session == nullptr || session->streaming == nullptr || out == nullptr || has_event == nullptr) {
            throw std::invalid_argument("streaming session, out, and has_event are required");
        }
        const auto event = session->streaming->next_stream_event();
        if (!event.has_value()) {
            *has_event = 0;
            *out = audio_cpp_stream_event{};
            return;
        }
        *has_event = 1;
        fill_stream_event(*out, *event);
    });
}

audio_cpp_status audio_cpp_session_finish_stream(
    audio_cpp_session *session,
    audio_cpp_task_result *out) {
    return catch_status([&] {
        if (session == nullptr || session->streaming == nullptr || out == nullptr) {
            throw std::invalid_argument("streaming session and out are required");
        }
        fill_task_result(*out, session->streaming->finish_stream());
    });
}

audio_cpp_status audio_cpp_session_reset(audio_cpp_session *session) {
    return catch_status([&] {
        if (session == nullptr || session->streaming == nullptr) {
            throw std::invalid_argument("streaming session is required");
        }
        session->streaming->reset();
    });
}

void audio_cpp_task_result_free(audio_cpp_task_result *result) {
    if (result == nullptr) {
        return;
    }
    free_owned_audio(result->audio);
    if (result->named_audio != nullptr) {
        for (size_t i = 0; i < result->named_audio_count; ++i) {
            free_named_audio(result->named_audio[i]);
        }
        delete[] result->named_audio;
    }
    free_owned_transcript(result->text);
    if (result->speech_segments != nullptr) {
        for (size_t i = 0; i < result->speech_segment_count; ++i) {
            free_speech_segment(result->speech_segments[i]);
        }
        delete[] result->speech_segments;
    }
    if (result->speaker_turns != nullptr) {
        for (size_t i = 0; i < result->speaker_turn_count; ++i) {
            free_speaker_turn(result->speaker_turns[i]);
        }
        delete[] result->speaker_turns;
    }
    if (result->word_timestamps != nullptr) {
        for (size_t i = 0; i < result->word_timestamp_count; ++i) {
            free_word_timestamp(result->word_timestamps[i]);
        }
        delete[] result->word_timestamps;
    }
    free_owned_artifact(result->artifact);
    if (result->artifacts != nullptr) {
        for (size_t i = 0; i < result->artifact_count; ++i) {
            free_owned_artifact(result->artifacts[i]);
        }
        delete[] result->artifacts;
    }
    *result = audio_cpp_task_result{};
}

void audio_cpp_stream_event_free(audio_cpp_stream_event *event) {
    if (event == nullptr) {
        return;
    }
    if (event->voice_activity != nullptr) {
        for (size_t i = 0; i < event->voice_activity_count; ++i) {
            if (event->voice_activity[i].has_segment != 0) {
                free_speech_segment(event->voice_activity[i].segment);
            }
        }
        delete[] event->voice_activity;
    }
    free_owned_transcript(event->partial_text);
    free_owned_audio(event->audio);
    if (event->named_audio != nullptr) {
        for (size_t i = 0; i < event->named_audio_count; ++i) {
            free_named_audio(event->named_audio[i]);
        }
        delete[] event->named_audio;
    }
    if (event->speaker_turns != nullptr) {
        for (size_t i = 0; i < event->speaker_turn_count; ++i) {
            free_speaker_turn(event->speaker_turns[i]);
        }
        delete[] event->speaker_turns;
    }
    if (event->word_timestamps != nullptr) {
        for (size_t i = 0; i < event->word_timestamp_count; ++i) {
            free_word_timestamp(event->word_timestamps[i]);
        }
        delete[] event->word_timestamps;
    }
    if (event->artifacts != nullptr) {
        for (size_t i = 0; i < event->artifact_count; ++i) {
            free_owned_artifact(event->artifacts[i]);
        }
        delete[] event->artifacts;
    }
    *event = audio_cpp_stream_event{};
}

}  // extern "C"
