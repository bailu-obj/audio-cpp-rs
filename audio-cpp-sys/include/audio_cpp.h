#ifndef AUDIO_CPP_H
#define AUDIO_CPP_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#ifndef AUDIO_CPP_API
#if defined(_WIN32) && defined(AUDIO_CPP_SHARED)
#ifdef AUDIO_CPP_BUILD
#define AUDIO_CPP_API __declspec(dllexport)
#else
#define AUDIO_CPP_API __declspec(dllimport)
#endif
#else
#define AUDIO_CPP_API
#endif
#endif

typedef struct audio_cpp_registry audio_cpp_registry;
typedef struct audio_cpp_model audio_cpp_model;
typedef struct audio_cpp_session audio_cpp_session;

typedef enum audio_cpp_status {
    AUDIO_CPP_OK = 0,
    AUDIO_CPP_ERR_INVALID_ARG = 1,
    AUDIO_CPP_ERR_NOT_FOUND = 2,
    AUDIO_CPP_ERR_UNSUPPORTED = 3,
    AUDIO_CPP_ERR_RUNTIME = 4,
    AUDIO_CPP_ERR_OOM = 5,
} audio_cpp_status;

typedef enum audio_cpp_voice_task_kind {
    AUDIO_CPP_TASK_VAD = 0,
    AUDIO_CPP_TASK_ASR = 1,
    AUDIO_CPP_TASK_DIARIZATION = 2,
    AUDIO_CPP_TASK_SOURCE_SEPARATION = 3,
    AUDIO_CPP_TASK_AUDIO_GENERATION = 4,
    AUDIO_CPP_TASK_TTS = 5,
    AUDIO_CPP_TASK_VOICE_CLONING = 6,
    AUDIO_CPP_TASK_VOICE_CONVERSION = 7,
    AUDIO_CPP_TASK_SPEECH_TO_SPEECH = 8,
    AUDIO_CPP_TASK_ALIGNMENT = 9,
    AUDIO_CPP_TASK_VOICE_DESIGN = 10,
    AUDIO_CPP_TASK_SPEAKER_RECOGNITION = 11,
    AUDIO_CPP_TASK_SVC = 12,
    AUDIO_CPP_TASK_MIDI = 13,
} audio_cpp_voice_task_kind;

typedef enum audio_cpp_run_mode {
    AUDIO_CPP_MODE_OFFLINE = 0,
    AUDIO_CPP_MODE_STREAMING = 1,
} audio_cpp_run_mode;

typedef enum audio_cpp_backend_type {
    AUDIO_CPP_BACKEND_CPU = 0,
    AUDIO_CPP_BACKEND_CUDA = 1,
    AUDIO_CPP_BACKEND_HIP = 2,
    AUDIO_CPP_BACKEND_VULKAN = 3,
    AUDIO_CPP_BACKEND_METAL = 4,
    AUDIO_CPP_BACKEND_BEST_AVAILABLE = 5,
} audio_cpp_backend_type;

typedef enum audio_cpp_artifact_kind {
    AUDIO_CPP_ARTIFACT_SPEAKER_EMBEDDING = 0,
    AUDIO_CPP_ARTIFACT_STYLE_EMBEDDING = 1,
    AUDIO_CPP_ARTIFACT_PROMPT_EMBEDDING = 2,
    AUDIO_CPP_ARTIFACT_ACOUSTIC_TOKENS = 3,
    AUDIO_CPP_ARTIFACT_MIDI = 4,
    AUDIO_CPP_ARTIFACT_TRANSCRIPT_ALIGNMENT = 5,
    AUDIO_CPP_ARTIFACT_DIARIZATION_STATE = 6,
    AUDIO_CPP_ARTIFACT_VAD_STATE = 7,
    AUDIO_CPP_ARTIFACT_CUSTOM = 8,
} audio_cpp_artifact_kind;

typedef enum audio_cpp_streaming_input_kind {
    AUDIO_CPP_STREAMING_INPUT_NONE = 0,
    AUDIO_CPP_STREAMING_INPUT_AUDIO_CHUNKS = 1,
} audio_cpp_streaming_input_kind;

typedef enum audio_cpp_streaming_output_kind {
    AUDIO_CPP_STREAMING_OUTPUT_FINAL_RESULT = 0,
    AUDIO_CPP_STREAMING_OUTPUT_PULL_EVENTS = 1,
} audio_cpp_streaming_output_kind;

typedef enum audio_cpp_vad_event_kind {
    AUDIO_CPP_VAD_SPEECH_START = 0,
    AUDIO_CPP_VAD_SPEECH_END = 1,
    AUDIO_CPP_VAD_SPEECH_SEGMENT = 2,
} audio_cpp_vad_event_kind;

typedef struct audio_cpp_kv {
    const char *key;
    const char *value;
} audio_cpp_kv;

typedef struct audio_cpp_owned_kv {
    char *key;
    char *value;
} audio_cpp_owned_kv;

typedef struct audio_cpp_backend_config {
    audio_cpp_backend_type type;
    int32_t device;
    int32_t threads;
} audio_cpp_backend_config;

typedef struct audio_cpp_backend_device {
    char *backend;
    int32_t index;
    char *name;
    char *type;
} audio_cpp_backend_device;

typedef struct audio_cpp_task_spec {
    audio_cpp_voice_task_kind task;
    audio_cpp_run_mode mode;
} audio_cpp_task_spec;

typedef struct audio_cpp_session_options {
    audio_cpp_backend_config backend;
    const audio_cpp_kv *options;
    size_t option_count;
} audio_cpp_session_options;

typedef struct audio_cpp_model_load_request {
    const char *model_path;
    const char *model_spec_override;
    const char *family_hint;
    const char *config_id;
    const char *weight_id;
    const audio_cpp_kv *options;
    size_t option_count;
} audio_cpp_model_load_request;

typedef struct audio_cpp_audio_view {
    int32_t sample_rate;
    int32_t channels;
    const float *samples;
    size_t sample_count;
} audio_cpp_audio_view;

typedef struct audio_cpp_owned_audio {
    int32_t sample_rate;
    int32_t channels;
    float *samples;
    size_t sample_count;
} audio_cpp_owned_audio;

typedef struct audio_cpp_audio_chunk {
    int32_t sample_rate;
    int32_t channels;
    int64_t start_sample;
    const float *samples;
    size_t sample_count;
} audio_cpp_audio_chunk;

typedef struct audio_cpp_transcript {
    const char *text;
    const char *language;
} audio_cpp_transcript;

typedef struct audio_cpp_owned_transcript {
    char *text;
    char *language;
} audio_cpp_owned_transcript;

typedef struct audio_cpp_voice_reference {
    audio_cpp_audio_view audio;
    uint8_t has_audio;
    const char *cached_voice_id;
} audio_cpp_voice_reference;

typedef struct audio_cpp_style_condition {
    const char *language;
    const char *emotion;
    float speaking_rate;
    uint8_t has_speaking_rate;
    float pitch_shift;
    uint8_t has_pitch_shift;
    float energy_scale;
    uint8_t has_energy_scale;
    const audio_cpp_kv *tags;
    size_t tag_count;
} audio_cpp_style_condition;

typedef struct audio_cpp_voice_condition {
    audio_cpp_voice_reference speaker;
    uint8_t has_speaker;
    audio_cpp_style_condition style;
    uint8_t has_style;
} audio_cpp_voice_condition;

typedef struct audio_cpp_artifact_view {
    audio_cpp_artifact_kind kind;
    const char *id;
    const uint8_t *payload;
    size_t payload_size;
    const audio_cpp_kv *meta;
    size_t meta_count;
} audio_cpp_artifact_view;

typedef struct audio_cpp_owned_artifact {
    audio_cpp_artifact_kind kind;
    char *id;
    uint8_t *payload;
    size_t payload_size;
    audio_cpp_owned_kv *meta;
    size_t meta_count;
} audio_cpp_owned_artifact;

typedef struct audio_cpp_task_request {
    audio_cpp_transcript text;
    uint8_t has_text;
    audio_cpp_audio_view audio;
    uint8_t has_audio;
    audio_cpp_voice_condition voice;
    uint8_t has_voice;
    const audio_cpp_artifact_view *artifacts;
    size_t artifact_count;
    const audio_cpp_kv *options;
    size_t option_count;
} audio_cpp_task_request;

typedef struct audio_cpp_session_prep_request {
    int32_t audio_sample_rate;
    int32_t audio_channels;
    int64_t max_input_samples;
    uint8_t has_audio;
    audio_cpp_transcript text;
    uint8_t has_text;
    audio_cpp_voice_condition voice;
    uint8_t has_voice;
    const audio_cpp_kv *options;
    size_t option_count;
} audio_cpp_session_prep_request;

typedef struct audio_cpp_named_audio {
    char *id;
    audio_cpp_owned_audio audio;
    audio_cpp_owned_kv *meta;
    size_t meta_count;
} audio_cpp_named_audio;

typedef struct audio_cpp_speech_segment {
    int64_t start_sample;
    int64_t end_sample;
    float confidence;
    char *text;
} audio_cpp_speech_segment;

typedef struct audio_cpp_speaker_turn {
    int64_t start_sample;
    int64_t end_sample;
    char *speaker_id;
    float confidence;
    char *text;
} audio_cpp_speaker_turn;

typedef struct audio_cpp_word_timestamp {
    int64_t start_sample;
    int64_t end_sample;
    char *word;
    float confidence;
} audio_cpp_word_timestamp;

typedef struct audio_cpp_task_result {
    audio_cpp_owned_audio audio;
    uint8_t has_audio;
    audio_cpp_named_audio *named_audio;
    size_t named_audio_count;
    audio_cpp_owned_transcript text;
    uint8_t has_text;
    audio_cpp_speech_segment *speech_segments;
    size_t speech_segment_count;
    audio_cpp_speaker_turn *speaker_turns;
    size_t speaker_turn_count;
    audio_cpp_word_timestamp *word_timestamps;
    size_t word_timestamp_count;
    audio_cpp_owned_artifact artifact;
    uint8_t has_artifact;
    audio_cpp_owned_artifact *artifacts;
    size_t artifact_count;
} audio_cpp_task_result;

typedef struct audio_cpp_vad_event {
    audio_cpp_vad_event_kind kind;
    int64_t sample;
    float probability;
    audio_cpp_speech_segment segment;
    uint8_t has_segment;
} audio_cpp_vad_event;

typedef struct audio_cpp_stream_event {
    audio_cpp_vad_event *voice_activity;
    size_t voice_activity_count;
    audio_cpp_owned_transcript partial_text;
    uint8_t has_partial_text;
    audio_cpp_owned_audio audio;
    uint8_t has_audio;
    audio_cpp_named_audio *named_audio;
    size_t named_audio_count;
    audio_cpp_speaker_turn *speaker_turns;
    size_t speaker_turn_count;
    audio_cpp_word_timestamp *word_timestamps;
    size_t word_timestamp_count;
    audio_cpp_owned_artifact *artifacts;
    size_t artifact_count;
    uint8_t is_final;
} audio_cpp_stream_event;

typedef struct audio_cpp_streaming_policy {
    audio_cpp_streaming_input_kind input;
    audio_cpp_streaming_output_kind output;
    int64_t preferred_audio_chunk_samples;
    double preferred_audio_chunk_seconds;
} audio_cpp_streaming_policy;

typedef struct audio_cpp_named_asset {
    char *id;
    char *path;
} audio_cpp_named_asset;

typedef struct audio_cpp_task_capability {
    audio_cpp_voice_task_kind task;
    audio_cpp_run_mode *modes;
    size_t mode_count;
} audio_cpp_task_capability;

typedef struct audio_cpp_capability_set {
    audio_cpp_task_capability *tasks;
    size_t task_count;
    char **languages;
    size_t language_count;
    uint8_t supports_speaker_reference;
    uint8_t supports_style_condition;
    uint8_t supports_timestamps;
} audio_cpp_capability_set;

typedef struct audio_cpp_model_metadata {
    char *family;
    char *variant;
    char *description;
    char **config_candidates;
    size_t config_candidate_count;
    char **weight_candidates;
    size_t weight_candidate_count;
} audio_cpp_model_metadata;

typedef struct audio_cpp_inspection {
    audio_cpp_model_metadata metadata;
    audio_cpp_capability_set capabilities;
    char *model_root;
    audio_cpp_named_asset *discovered_configs;
    size_t discovered_config_count;
    audio_cpp_named_asset *discovered_weights;
    size_t discovered_weight_count;
} audio_cpp_inspection;

typedef void (*audio_cpp_stream_event_fn)(const audio_cpp_stream_event *event, void *user_data);

AUDIO_CPP_API const char *audio_cpp_last_error(void);

AUDIO_CPP_API const char *audio_cpp_voice_task_kind_name(audio_cpp_voice_task_kind kind);
AUDIO_CPP_API audio_cpp_status audio_cpp_voice_task_kind_parse(
    const char *value,
    audio_cpp_voice_task_kind *out);
AUDIO_CPP_API const char *audio_cpp_run_mode_name(audio_cpp_run_mode mode);
AUDIO_CPP_API audio_cpp_status audio_cpp_run_mode_parse(const char *value, audio_cpp_run_mode *out);
AUDIO_CPP_API const char *audio_cpp_backend_type_name(audio_cpp_backend_type type);
AUDIO_CPP_API audio_cpp_status audio_cpp_backend_type_parse(
    const char *value,
    audio_cpp_backend_type *out);
AUDIO_CPP_API const char *audio_cpp_artifact_kind_name(audio_cpp_artifact_kind kind);
AUDIO_CPP_API audio_cpp_status audio_cpp_artifact_kind_parse(
    const char *value,
    audio_cpp_artifact_kind *out);

AUDIO_CPP_API audio_cpp_status audio_cpp_list_backend_devices(
    audio_cpp_backend_device **out,
    size_t *count);
AUDIO_CPP_API void audio_cpp_backend_devices_free(audio_cpp_backend_device *devices, size_t count);
AUDIO_CPP_API void audio_cpp_string_list_free(char **items, size_t count);

AUDIO_CPP_API audio_cpp_status audio_cpp_registry_create_default(
    const char *config_path,
    audio_cpp_registry **out);
AUDIO_CPP_API void audio_cpp_registry_destroy(audio_cpp_registry *registry);
AUDIO_CPP_API size_t audio_cpp_registry_size(const audio_cpp_registry *registry);
AUDIO_CPP_API uint8_t audio_cpp_registry_empty(const audio_cpp_registry *registry);
AUDIO_CPP_API uint8_t audio_cpp_registry_supports_family(
    const audio_cpp_registry *registry,
    const char *family);
AUDIO_CPP_API audio_cpp_status audio_cpp_registry_families(
    const audio_cpp_registry *registry,
    char ***out,
    size_t *count);
AUDIO_CPP_API audio_cpp_status audio_cpp_registry_inspect(
    const audio_cpp_registry *registry,
    const audio_cpp_model_load_request *request,
    audio_cpp_inspection *out);
AUDIO_CPP_API audio_cpp_status audio_cpp_registry_load(
    const audio_cpp_registry *registry,
    const audio_cpp_model_load_request *request,
    audio_cpp_model **out);
AUDIO_CPP_API void audio_cpp_inspection_free(audio_cpp_inspection *inspection);

AUDIO_CPP_API void audio_cpp_model_destroy(audio_cpp_model *model);
AUDIO_CPP_API audio_cpp_status audio_cpp_model_get_metadata(
    const audio_cpp_model *model,
    audio_cpp_model_metadata *out);
AUDIO_CPP_API void audio_cpp_model_metadata_free(audio_cpp_model_metadata *metadata);
AUDIO_CPP_API audio_cpp_status audio_cpp_model_capabilities(
    const audio_cpp_model *model,
    audio_cpp_capability_set *out);
AUDIO_CPP_API void audio_cpp_capability_set_free(audio_cpp_capability_set *capabilities);
AUDIO_CPP_API audio_cpp_status audio_cpp_model_create_session(
    const audio_cpp_model *model,
    audio_cpp_task_spec spec,
    audio_cpp_session_options options,
    audio_cpp_session **out);

AUDIO_CPP_API void audio_cpp_session_destroy(audio_cpp_session *session);
AUDIO_CPP_API audio_cpp_status audio_cpp_session_family(
    const audio_cpp_session *session,
    const char **out);
AUDIO_CPP_API audio_cpp_status audio_cpp_session_task_kind(
    const audio_cpp_session *session,
    audio_cpp_voice_task_kind *out);
AUDIO_CPP_API audio_cpp_status audio_cpp_session_run_mode(
    const audio_cpp_session *session,
    audio_cpp_run_mode *out);
AUDIO_CPP_API uint8_t audio_cpp_session_supports_offline(const audio_cpp_session *session);
AUDIO_CPP_API uint8_t audio_cpp_session_supports_streaming(const audio_cpp_session *session);
AUDIO_CPP_API audio_cpp_status audio_cpp_build_prep_from_request(
    const audio_cpp_task_request *request,
    audio_cpp_session_prep_request *out);
AUDIO_CPP_API audio_cpp_status audio_cpp_session_prepare(
    audio_cpp_session *session,
    const audio_cpp_session_prep_request *request);
AUDIO_CPP_API audio_cpp_status audio_cpp_session_run(
    audio_cpp_session *session,
    const audio_cpp_task_request *request,
    audio_cpp_task_result *out);
AUDIO_CPP_API audio_cpp_status audio_cpp_session_streaming_policy(
    const audio_cpp_session *session,
    audio_cpp_streaming_policy *out);
AUDIO_CPP_API audio_cpp_status audio_cpp_session_set_stream_callback(
    audio_cpp_session *session,
    audio_cpp_stream_event_fn callback,
    void *user_data);
AUDIO_CPP_API audio_cpp_status audio_cpp_session_start_stream(
    audio_cpp_session *session,
    const audio_cpp_task_request *request);
AUDIO_CPP_API audio_cpp_status audio_cpp_session_process_audio_chunk(
    audio_cpp_session *session,
    const audio_cpp_audio_chunk *chunk,
    audio_cpp_stream_event *out);
AUDIO_CPP_API audio_cpp_status audio_cpp_session_next_stream_event(
    audio_cpp_session *session,
    audio_cpp_stream_event *out,
    uint8_t *has_event);
AUDIO_CPP_API audio_cpp_status audio_cpp_session_finish_stream(
    audio_cpp_session *session,
    audio_cpp_task_result *out);
AUDIO_CPP_API audio_cpp_status audio_cpp_session_reset(audio_cpp_session *session);

AUDIO_CPP_API void audio_cpp_task_result_free(audio_cpp_task_result *result);
AUDIO_CPP_API void audio_cpp_stream_event_free(audio_cpp_stream_event *event);

#ifdef __cplusplus
}
#endif

#endif
