//! Build `engine_runtime` from the vendored `audio.cpp` tree and generate FFI bindings.

#![allow(missing_docs)]

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use cmake::Config;
use walkdir::WalkDir;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=include/audio_cpp.h");
    println!("cargo:rerun-if-changed=src/audio_cpp.cpp");
    println!("cargo:rerun-if-changed=audio.cpp/CMakeLists.txt");
    println!("cargo:rerun-if-env-changed=AUDIOCPP_LIB_PROFILE");
    println!("cargo:rerun-if-env-changed=AUDIOCPP_MODEL_SET");
    println!("cargo:rerun-if-env-changed=AUDIOCPP_MODELS");
    println!("cargo:rerun-if-env-changed=AUDIOCPP_GGML_SOURCE_DIR");
    println!("cargo:rerun-if-env-changed=AUDIOCPP_USE_SYSTEM_GGML");
    println!("cargo:rerun-if-env-changed=AUDIOCPP_GGML_INCLUDE_DIR");
    println!("cargo:rerun-if-env-changed=ggml_DIR");
    println!("cargo:rerun-if-env-changed=CMAKE_PREFIX_PATH");
    println!("cargo:rerun-if-env-changed=CMAKE_BUILD_PARALLEL_LEVEL");

    if cfg!(feature = "cuda") && cfg!(feature = "hip") {
        panic!("features `cuda` and `hip` are mutually exclusive");
    }
    if cfg!(feature = "full-models") && cfg!(feature = "core-models") {
        panic!("features `full-models` and `core-models` are mutually exclusive");
    }
    if cfg!(feature = "native-cpu") && cfg!(feature = "portable-cpu") {
        panic!("features `native-cpu` and `portable-cpu` are mutually exclusive");
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let target = env::var("TARGET").expect("TARGET");
    let source_dir = manifest_dir.join("audio.cpp");

    if !source_dir.join("CMakeLists.txt").is_file() {
        panic!(
            "audio.cpp submodule is missing at {}. Run `git submodule update --init --recursive`.",
            source_dir.display()
        );
    }

    generate_bindings(&manifest_dir, &out_dir);

    if env::var_os("DOCS_RS").is_some() {
        return;
    }

    let enable_metal = should_enable_metal(&target);
    let enable_openmp = should_enable_openmp(&target);
    let model_set = if cfg!(feature = "core-models") {
        "core"
    } else {
        "full"
    };

    let use_system_ggml = cfg!(feature = "external-ggml") || env_truthy("AUDIOCPP_USE_SYSTEM_GGML");
    let ggml_include = if use_system_ggml {
        resolve_system_ggml_include()
    } else {
        resolve_ggml_source(&source_dir).join("include")
    };

    let mut config = Config::new(&source_dir);
    let profile = env::var("AUDIOCPP_LIB_PROFILE").unwrap_or_else(|_| "RelWithDebInfo".into());
    config
        .profile(&profile)
        .build_target("engine_runtime")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("ENGINE_BUILD_EXAMPLES", "OFF")
        .define("ENGINE_BUILD_TESTS", "OFF")
        .define("ENGINE_BUILD_WARMBENCH", "OFF")
        .define("SPM_ENABLE_SHARED", "OFF")
        .define("SPM_BUILD_TEST", "OFF")
        .define("AUDIOCPP_MODEL_SET", model_set)
        .define("AUDIOCPP_DEPLOYMENT_BUILD", "ON")
        .define("AUDIOCPP_USE_SYSTEM_GGML", bool_flag(use_system_ggml))
        .define("ENGINE_ENABLE_CUDA", bool_flag(cfg!(feature = "cuda")))
        .define("ENGINE_ENABLE_HIP", bool_flag(cfg!(feature = "hip")))
        .define("ENGINE_ENABLE_VULKAN", bool_flag(cfg!(feature = "vulkan")))
        .define("ENGINE_ENABLE_METAL", bool_flag(enable_metal))
        .define("ENGINE_ENABLE_OPENMP", bool_flag(enable_openmp))
        .pic(true);

    if use_system_ggml {
        println!("cargo:warning=using shared/system ggml");
    } else {
        let ggml_source = ggml_include
            .parent()
            .map_or_else(|| source_dir.join("external/ggml"), Path::to_path_buf);
        config.define("AUDIOCPP_GGML_SOURCE_DIR", ggml_source.to_string_lossy());
        config.define("GGML_BACKEND_DL", "OFF");
        config.define("GGML_BUILD_EXAMPLES", "OFF");
        config.define("GGML_BUILD_TESTS", "OFF");
        config.define("GGML_OPENMP", bool_flag(enable_openmp));
        if enable_metal {
            config.define("GGML_METAL_EMBED_LIBRARY", "ON");
            config.define("GGML_METAL_NDEBUG", "ON");
        }
    }

    if cfg!(feature = "native-cpu") {
        config.define("ENGINE_ENABLE_NATIVE_CPU", "ON");
    } else if cfg!(feature = "portable-cpu") {
        config.define("ENGINE_ENABLE_NATIVE_CPU", "OFF");
    }

    if let Ok(models) = env::var("AUDIOCPP_MODELS") {
        if !models.is_empty() {
            config.define("AUDIOCPP_MODEL_SET", "custom");
            config.define("AUDIOCPP_MODELS", models);
        }
    }

    if target.contains("msvc") {
        config.cxxflag("/utf-8");
    }

    if enable_openmp && target.contains("apple") {
        if let Some(prefix) = brew_prefix("libomp") {
            config.define("OpenMP_ROOT", prefix);
        }
    }

    for (key, value) in env::vars() {
        let useful = key.starts_with("CMAKE_")
            || key.starts_with("ENGINE_")
            || key.starts_with("GGML_")
            || key.starts_with("AUDIOCPP_");
        if useful
            && !value.is_empty()
            && key != "AUDIOCPP_LIB_PROFILE"
            && key != "AUDIOCPP_GGML_SOURCE_DIR"
            && key != "AUDIOCPP_USE_SYSTEM_GGML"
        {
            config.define(&key, &value);
        }
    }

    let destination = config.build();
    add_link_search_path(&out_dir);
    add_link_search_path(&destination);
    add_link_search_path(&out_dir.join("build"));

    compile_shim(&manifest_dir, &source_dir, &ggml_include, &target);

    println!("cargo:rustc-link-lib=static=audio_cpp_c_api");
    println!("cargo:rustc-link-lib=static=engine_runtime");
    if use_system_ggml {
        link_shared_ggml(&target, enable_metal);
    } else {
        if enable_metal {
            println!("cargo:rustc-link-lib=static=ggml-metal");
        }
        if cfg!(feature = "cuda") {
            println!("cargo:rustc-link-lib=static=ggml-cuda");
        }
        if cfg!(feature = "hip") {
            println!("cargo:rustc-link-lib=static=ggml-hip");
        }
        if cfg!(feature = "vulkan") {
            println!("cargo:rustc-link-lib=static=ggml-vulkan");
        }
        link_static_if_present(&out_dir, "ggml-blas");
        println!("cargo:rustc-link-lib=static=ggml");
        println!("cargo:rustc-link-lib=static=ggml-base");
        println!("cargo:rustc-link-lib=static=ggml-cpu");
    }
    println!("cargo:rustc-link-lib=static=sentencepiece");
    println!("cargo:rustc-link-lib=static=cjson_vendor");
    println!("cargo:rustc-link-lib=static=yaml_vendor");

    link_system_libs(&target, enable_metal, enable_openmp);
}

fn generate_bindings(manifest_dir: &Path, out_dir: &Path) {
    let bindings = bindgen::Builder::default()
        .header(manifest_dir.join("wrapper.h").to_string_lossy())
        .clang_arg(format!("-I{}", manifest_dir.join("include").display()))
        .allowlist_function("audio_cpp_.*")
        .allowlist_type("audio_cpp_.*")
        .allowlist_var("AUDIO_CPP_.*")
        .rustified_enum("audio_cpp_status")
        .rustified_enum("audio_cpp_voice_task_kind")
        .rustified_enum("audio_cpp_run_mode")
        .rustified_enum("audio_cpp_backend_type")
        .rustified_enum("audio_cpp_artifact_kind")
        .rustified_enum("audio_cpp_streaming_input_kind")
        .rustified_enum("audio_cpp_streaming_output_kind")
        .rustified_enum("audio_cpp_vad_event_kind")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .derive_default(true)
        .derive_debug(true)
        .generate()
        .expect("failed to generate audio.cpp bindings");
    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("failed to write bindings.rs");
}

fn resolve_ggml_source(audio_cpp_source: &Path) -> PathBuf {
    let requested = env::var_os("AUDIOCPP_GGML_SOURCE_DIR").map(PathBuf::from);
    let custom = requested.is_some();
    let raw = requested.unwrap_or_else(|| audio_cpp_source.join("external/ggml"));
    let resolved = locate_ggml_source(&raw).unwrap_or_else(|error| panic!("{error}"));
    if custom {
        println!("cargo:warning=building ggml from {}", resolved.display());
    }
    println!("cargo:rerun-if-changed={}", resolved.display());
    resolved
}

fn resolve_system_ggml_include() -> PathBuf {
    if let Some(path) = env::var_os("AUDIOCPP_GGML_INCLUDE_DIR").map(PathBuf::from) {
        if path.join("ggml.h").is_file() {
            return canonicalize_or_clone(&path);
        }
    }
    if let Some(source) = env::var_os("AUDIOCPP_GGML_SOURCE_DIR").map(PathBuf::from) {
        if let Ok(resolved) = locate_ggml_source(&source) {
            return resolved.join("include");
        }
    }
    for prefix in cmake_prefixes() {
        let include = prefix.join("include");
        if include.join("ggml.h").is_file() {
            return include;
        }
    }
    panic!(
        "feature `external-ggml` / AUDIOCPP_USE_SYSTEM_GGML requires an installed ggml. \
         Set ggml_DIR or CMAKE_PREFIX_PATH to the ggml prefix, or AUDIOCPP_GGML_INCLUDE_DIR \
         / AUDIOCPP_GGML_SOURCE_DIR for headers"
    );
}

fn cmake_prefixes() -> Vec<PathBuf> {
    let mut prefixes = Vec::new();
    if let Some(dir) = env::var_os("ggml_DIR").map(PathBuf::from) {
        if let Some(prefix) = dir
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .map(Path::to_path_buf)
        {
            prefixes.push(prefix);
        }
        prefixes.push(dir);
    }
    if let Ok(value) = env::var("CMAKE_PREFIX_PATH") {
        for part in value.split([':', ';']) {
            if !part.is_empty() {
                prefixes.push(PathBuf::from(part));
            }
        }
    }
    prefixes
}

fn env_truthy(key: &str) -> bool {
    env::var(key).is_ok_and(|value| {
        matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "on" | "yes"
        )
    })
}

fn link_shared_ggml(target: &str, enable_metal: bool) {
    let lib_dirs = cmake_prefixes()
        .into_iter()
        .flat_map(|prefix| [prefix.join("lib"), prefix.join("lib64"), prefix])
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    if lib_dirs.is_empty() {
        panic!("could not find installed ggml libraries; set ggml_DIR or CMAKE_PREFIX_PATH");
    }
    for dir in &lib_dirs {
        println!("cargo:rustc-link-search=native={}", dir.display());
    }
    let kind = if lib_dirs.iter().any(|dir| ggml_lib_is_static(dir, target)) {
        "static"
    } else {
        "dylib"
    };
    let mut names = vec!["ggml", "ggml-base", "ggml-cpu"];
    if enable_metal {
        names.push("ggml-metal");
    }
    if cfg!(feature = "cuda") {
        names.push("ggml-cuda");
    }
    if cfg!(feature = "hip") {
        names.push("ggml-hip");
    }
    if cfg!(feature = "vulkan") {
        names.push("ggml-vulkan");
    }
    names.push("ggml-blas");
    for name in names {
        if kind == "static" {
            if name == "ggml-blas" {
                for dir in &lib_dirs {
                    link_static_if_present(dir, name);
                }
            } else {
                println!("cargo:rustc-link-lib=static={name}");
            }
        } else if name != "ggml-blas"
            || lib_dirs.iter().any(|dir| library_exists(dir, name, target))
        {
            println!("cargo:rustc-link-lib=dylib={name}");
        }
    }
}

fn ggml_lib_is_static(dir: &Path, target: &str) -> bool {
    dir.join("libggml.a").is_file()
        || (target.contains("windows")
            && (dir.join("ggml.lib").is_file() || dir.join("libggml.lib").is_file()))
}

fn library_exists(dir: &Path, name: &str, target: &str) -> bool {
    [
        format!("lib{name}.dylib"),
        format!("lib{name}.so"),
        format!("lib{name}.a"),
        format!("{name}.lib"),
        format!("lib{name}.lib"),
        format!("{name}.dll"),
    ]
    .into_iter()
    .any(|file_name| dir.join(file_name).is_file())
        || (target.contains("windows") && dir.join(format!("{name}.dll")).is_file())
}

fn locate_ggml_source(path: &Path) -> Result<PathBuf, String> {
    for candidate in expand_source_path(path) {
        if is_ggml_source(&candidate) {
            return Ok(canonicalize_or_clone(&candidate));
        }
    }
    Err(format!(
        "ggml source not found at {} (expected CMakeLists.txt and include/ggml.h)",
        path.display()
    ))
}

fn expand_source_path(path: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if path.is_absolute() {
        out.push(path.to_path_buf());
        return out;
    }
    out.push(env::current_dir().map_or_else(|_| path.to_path_buf(), |cwd| cwd.join(path)));
    if let Ok(manifest) = env::var("CARGO_MANIFEST_DIR") {
        out.push(PathBuf::from(manifest).join(path));
    }
    out
}

fn is_ggml_source(path: &Path) -> bool {
    path.join("CMakeLists.txt").is_file() && path.join("include/ggml.h").is_file()
}

fn canonicalize_or_clone(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn compile_shim(manifest_dir: &Path, source_dir: &Path, ggml_include: &Path, target: &str) {
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file(manifest_dir.join("src/audio_cpp.cpp"))
        .include(manifest_dir.join("include"))
        .include(source_dir.join("include"))
        .include(ggml_include)
        .flag_if_supported("-std=c++17")
        .flag_if_supported("-fexceptions")
        .warnings(false)
        .pic(true);
    if target.contains("msvc") {
        build.flag("/std:c++17");
        build.flag("/EHsc");
        build.flag("/utf-8");
    }
    build.compile("audio_cpp_c_api");
}

fn should_enable_metal(target: &str) -> bool {
    if cfg!(feature = "metal") {
        if !target.contains("apple") {
            panic!("feature `metal` is only supported on Apple targets");
        }
        return true;
    }
    target.contains("apple") && !cfg!(feature = "portable-cpu")
}

fn should_enable_openmp(target: &str) -> bool {
    if !cfg!(feature = "openmp") {
        return false;
    }
    if !target.contains("apple") {
        return true;
    }
    if let Some(prefix) = brew_prefix("libomp") {
        println!("cargo:warning=enabling OpenMP via {}", prefix.display());
        return true;
    }
    println!("cargo:warning=OpenMP requested but libomp was not found; building without OpenMP");
    false
}

fn brew_prefix(formula: &str) -> Option<PathBuf> {
    let output = Command::new("brew")
        .args(["--prefix", formula])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let prefix = String::from_utf8(output.stdout).ok()?;
    let path = PathBuf::from(prefix.trim());
    path.is_dir().then_some(path)
}

fn bool_flag(value: bool) -> &'static str {
    if value {
        "ON"
    } else {
        "OFF"
    }
}

fn add_link_search_path(dir: &Path) {
    if !dir.is_dir() {
        return;
    }
    println!("cargo:rustc-link-search=native={}", dir.display());
    for entry in WalkDir::new(dir).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_dir() {
            println!("cargo:rustc-link-search=native={}", entry.path().display());
        }
    }
}

fn link_static_if_present(root: &Path, name: &str) {
    let file_names = [
        format!("lib{name}.a"),
        format!("{name}.lib"),
        format!("lib{name}.lib"),
    ];
    let found = WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .any(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|file_name| file_names.iter().any(|expected| file_name == expected))
        });
    if found {
        println!("cargo:rustc-link-lib=static={name}");
    }
}

fn link_system_libs(target: &str, enable_metal: bool, enable_openmp: bool) {
    if let Some(stdlib) = cpp_stdlib(target) {
        println!("cargo:rustc-link-lib=dylib={stdlib}");
    }

    if target.contains("apple") {
        println!("cargo:rustc-link-lib=framework=Accelerate");
        println!("cargo:rustc-link-lib=framework=Foundation");
        if enable_metal {
            println!("cargo:rustc-link-lib=framework=Metal");
            println!("cargo:rustc-link-lib=framework=MetalKit");
        }
    }

    if enable_openmp && target.contains("gnu") {
        println!("cargo:rustc-link-lib=gomp");
    }
    if enable_openmp && target.contains("apple") {
        if let Some(prefix) = brew_prefix("libomp") {
            println!(
                "cargo:rustc-link-search=native={}",
                prefix.join("lib").display()
            );
            println!("cargo:rustc-link-lib=omp");
        }
    }

    if cfg!(feature = "cuda") {
        println!("cargo:rerun-if-env-changed=CUDA_PATH");
        println!("cargo:rerun-if-env-changed=CUDAToolkit_ROOT");
        if target.contains("windows") {
            println!("cargo:rustc-link-lib=cudart");
            println!("cargo:rustc-link-lib=cublas");
            println!("cargo:rustc-link-lib=cublasLt");
            println!("cargo:rustc-link-lib=cufft");
            println!("cargo:rustc-link-lib=cuda");
        } else {
            println!("cargo:rustc-link-lib=static=cudart_static");
            println!("cargo:rustc-link-lib=static=cublas_static");
            println!("cargo:rustc-link-lib=static=cublasLt_static");
            println!("cargo:rustc-link-lib=cufft");
            println!("cargo:rustc-link-lib=cuda");
            println!("cargo:rustc-link-lib=static=culibos");
        }
    }

    if cfg!(feature = "hip") {
        println!("cargo:rerun-if-env-changed=ROCM_PATH");
        println!("cargo:rerun-if-env-changed=HIP_PATH");
        let hip = env::var("ROCM_PATH")
            .or_else(|_| env::var("HIP_PATH"))
            .unwrap_or_else(|_| "/opt/rocm".into());
        println!("cargo:rustc-link-search=native={hip}/lib");
        println!("cargo:rustc-link-lib=amdhip64");
        println!("cargo:rustc-link-lib=rocblas");
        println!("cargo:rustc-link-lib=hipblas");
    }

    if cfg!(feature = "vulkan") {
        println!("cargo:rerun-if-env-changed=VULKAN_SDK");
        if target.contains("windows") {
            println!("cargo:rustc-link-lib=vulkan-1");
            if let Ok(sdk) = env::var("VULKAN_SDK") {
                println!("cargo:rustc-link-search=native={sdk}/Lib");
            }
        } else {
            println!("cargo:rustc-link-lib=vulkan");
            if let Ok(sdk) = env::var("VULKAN_SDK") {
                println!("cargo:rustc-link-search=native={sdk}/lib");
            }
        }
    }
}

fn cpp_stdlib(target: &str) -> Option<&'static str> {
    if target.contains("msvc") {
        None
    } else if target.contains("apple") || target.contains("freebsd") || target.contains("openbsd") {
        Some("c++")
    } else if target.contains("android") {
        Some("c++_shared")
    } else {
        Some("stdc++")
    }
}
