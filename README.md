# audio-cpp-rs

Rust bindings for [audio.cpp](https://github.com/bailu-obj/audio.cpp): a raw FFI crate
(`audio-cpp-sys`) and a safe wrapper (`audio-cpp`) around the generic voice
runtime.

The workspace vendors `audio.cpp` as a git submodule and links the static
`engine_runtime` library. There is no upstream C ABI, so `audio-cpp-sys` ships a
small C++ shim that exposes registry, model, and session calls without leaking
exceptions or STL types.

## Crates

| Crate | Role |
| --- | --- |
| `audio-cpp-sys` | CMake build, bindgen, raw `audio_cpp_*` declarations |
| `audio-cpp` | RAII `Registry` / `Model` / `Session` types and owned value objects |

## Checkout

```bash
git clone --recurse-submodules https://github.com/bailu-obj/audio-cpp-rs.git
cd audio-cpp-rs
```

If the clone was not recursive:

```bash
git submodule update --init --recursive
```

## Toolchain

- Rust 1.80+
- CMake 3.20+
- A C++17 compiler
- On macOS, Xcode CLT. Metal is enabled automatically unless you select
  `portable-cpu`.
- Optional: CUDA 12+, Vulkan SDK, or ROCm when those features are enabled
- Optional: `libomp` if you want the `openmp` feature on macOS (`brew install libomp`)

A first build compiles the full upstream model set and can take a long time.

## Features

Both crates share these Cargo features:

| Feature | Default | Effect |
| --- | --- | --- |
| `full-models` | yes | `-DAUDIOCPP_MODEL_SET=full` |
| `core-models` | no | Framework plus bundled VAD models only |
| `openmp` | yes | Enable OpenMP when the toolchain provides it |
| `metal` | no | Force Metal (Apple only; also auto-enabled on macOS) |
| `cuda` | no | CUDA backend |
| `hip` | no | HIP/ROCm backend (exclusive with `cuda`) |
| `vulkan` | no | Vulkan backend |
| `native-cpu` | no | Native host CPU kernels |
| `portable-cpu` | no | Portable CPU kernels; disables automatic Metal |
| `external-ggml` | no | Use a shared/installed ggml (`find_package(ggml)`), like llama.cpp |

`full-models` and `core-models` cannot be combined. `cuda` and `hip` cannot be
combined.

ggml is consumed the same way llama.cpp does:

1. Reuse a `ggml` / `ggml::ggml` target if a parent CMake project already added it
2. Otherwise `find_package(ggml)` when `AUDIOCPP_USE_SYSTEM_GGML=ON` or the
   `external-ggml` feature is enabled
3. Otherwise build from `AUDIOCPP_GGML_SOURCE_DIR` (default:
   `audio-cpp-sys/audio.cpp/external/ggml`)

Share an installed ggml with llama.cpp:

```bash
# after cmake --install of ggml (or llama.cpp's ggml)
CMAKE_PREFIX_PATH=/path/to/ggml-prefix \
  cargo build -p audio-cpp --features external-ggml
```

Or point CMake at the package dir:

```bash
ggml_DIR=/path/to/prefix/lib/cmake/ggml \
  cargo build -p audio-cpp --features external-ggml
```

To build ggml from another source tree instead of sharing a prebuilt library:

```bash
AUDIOCPP_GGML_SOURCE_DIR=/path/to/llama.cpp/ggml cargo build -p audio-cpp
```

Faster local checks:

```bash
cargo test -p audio-cpp --no-default-features --features core-models
```

Environment overrides passed through to CMake include `CMAKE_*`, `ENGINE_*`,
`GGML_*`, `AUDIOCPP_*` (except `AUDIOCPP_GGML_SOURCE_DIR` and
`AUDIOCPP_USE_SYSTEM_GGML`, which are resolved first), and
`AUDIOCPP_LIB_PROFILE` (default `RelWithDebInfo`).

## Example

```rust
use audio_cpp::{
    ModelLoadRequest, Registry, SessionOptions, TaskOptions, TaskRequest, TaskSpec,
    VoiceTaskKind,
};

fn main() -> audio_cpp::Result<()> {
    let registry = Registry::new()?;
    let model = registry.load(
        &ModelLoadRequest::new("/path/to/model").family_hint("qwen3_tts"),
    )?;
    let mut session = model.create_session(
        TaskSpec::offline(VoiceTaskKind::Tts),
        &SessionOptions::new(),
    )?;
    let result = session.prepare_and_run(
        &TaskRequest::new()
            .text("Hello from Rust")
            .options(TaskOptions {
                seed: Some(1234),
                temperature: Some(0.7),
                ..TaskOptions::default()
            }),
    )?;
    let _samples = result.audio.map(|audio| audio.samples);
    Ok(())
}
```

`TaskRequest` options are typed. Shared knobs live on `TaskOptions`; families with extra
keys have dedicated structs such as `VoxCpm2Options` that convert into `TaskOptions`.
Unset fields are omitted so schema-backed models are not sent unknown keys.

Sessions must not outlive the `Model` that created them. A session is prepared
explicitly, or with `prepare_and_run` / `prepare_from_request`. Streaming
sessions expose `start_stream`, `process_audio_chunk`, `next_stream_event`,
`finish_stream`, `reset`, and a scoped `with_event_callback` helper.

An ignored integration test and `audio-cpp/examples/run.rs` use
`AUDIO_CPP_TEST_MODEL` when you have local weights. The example inspects the
model instead of assuming a family. Optional overrides:

```bash
AUDIO_CPP_TEST_MODEL=/path/to/model \
AUDIO_CPP_TEST_FAMILY=qwen3_tts \
AUDIO_CPP_TEST_BACKEND=metal \
cargo run -p audio-cpp --example run
```

## License

Apache-2.0. Upstream `audio.cpp` is also Apache-2.0; vendored third-party trees
keep their own licenses.
