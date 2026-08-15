//! Raw FFI bindings to the `audio.cpp` C ABI.
//!
//! Prefer the safe [`audio-cpp`](https://docs.rs/audio-cpp) crate unless you need
//! direct access to the generated declarations.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(clippy::all)]
#![allow(rustdoc::broken_intra_doc_links)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn parses_task_kind_names() {
        unsafe {
            let mut kind = audio_cpp_voice_task_kind::AUDIO_CPP_TASK_VAD;
            assert_eq!(
                audio_cpp_voice_task_kind_parse(c"tts".as_ptr(), &mut kind),
                audio_cpp_status::AUDIO_CPP_OK
            );
            assert_eq!(kind, audio_cpp_voice_task_kind::AUDIO_CPP_TASK_TTS);
            assert_eq!(
                std::ffi::CStr::from_ptr(audio_cpp_voice_task_kind_name(kind))
                    .to_str()
                    .unwrap(),
                "tts"
            );
            assert_eq!(
                audio_cpp_voice_task_kind_parse(c"not-a-task".as_ptr(), &mut kind),
                audio_cpp_status::AUDIO_CPP_ERR_UNSUPPORTED
            );
            assert!(!audio_cpp_last_error().is_null());
        }
    }

    #[test]
    fn lists_backend_devices() {
        unsafe {
            let mut devices = ptr::null_mut();
            let mut count = 0usize;
            assert_eq!(
                audio_cpp_list_backend_devices(&mut devices, &mut count),
                audio_cpp_status::AUDIO_CPP_OK
            );
            assert!(count >= 1);
            assert!(!devices.is_null());
            audio_cpp_backend_devices_free(devices, count);
        }
    }

    #[test]
    fn creates_default_registry() {
        unsafe {
            let mut registry = ptr::null_mut();
            assert_eq!(
                audio_cpp_registry_create_default(ptr::null(), &mut registry),
                audio_cpp_status::AUDIO_CPP_OK
            );
            assert!(!registry.is_null());
            assert_eq!(audio_cpp_registry_empty(registry), 0);
            assert!(audio_cpp_registry_size(registry) > 0);

            let mut families = ptr::null_mut();
            let mut count = 0usize;
            assert_eq!(
                audio_cpp_registry_families(registry, &mut families, &mut count),
                audio_cpp_status::AUDIO_CPP_OK
            );
            assert!(count > 0);
            audio_cpp_string_list_free(families, count);
            audio_cpp_registry_destroy(registry);
        }
    }
}
