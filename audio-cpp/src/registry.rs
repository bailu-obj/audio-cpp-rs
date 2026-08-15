//! Model registry and backend discovery.

use std::path::PathBuf;
use std::ptr;

use audio_cpp_sys::{
    audio_cpp_registry, audio_cpp_registry_create_default, audio_cpp_registry_destroy,
    audio_cpp_registry_empty, audio_cpp_registry_families, audio_cpp_registry_inspect,
    audio_cpp_registry_load, audio_cpp_registry_size, audio_cpp_registry_supports_family,
};

use crate::convert::{
    check, list_backend_devices, path_cstring, take_inspection, take_string_list,
    EncodedLoadRequest,
};
use crate::error::Result;
use crate::model::Model;
use crate::types::{BackendDevice, ModelInspection, ModelLoadRequest};

/// Catalog of compiled-in model loaders.
#[derive(Debug)]
pub struct Registry {
    ptr: *mut audio_cpp_registry,
}

unsafe impl Send for Registry {}
unsafe impl Sync for Registry {}

impl Registry {
    /// Create the default registry, optionally filtered by a config file.
    pub fn new() -> Result<Self> {
        Self::with_config_path(None::<&str>)
    }

    /// Create the default registry using an optional loader-config path.
    pub fn with_config_path(path: Option<impl AsRef<std::path::Path>>) -> Result<Self> {
        let owned = path.map(|path| path_cstring(path.as_ref())).transpose()?;
        let mut ptr = ptr::null_mut();
        check(unsafe {
            audio_cpp_registry_create_default(
                owned.as_ref().map_or(ptr::null(), |value| value.as_ptr()),
                &mut ptr,
            )
        })?;
        Ok(Self { ptr })
    }

    /// Number of registered loaders.
    #[must_use]
    pub fn len(&self) -> usize {
        unsafe { audio_cpp_registry_size(self.ptr) }
    }

    /// Whether the registry has no loaders.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        unsafe { audio_cpp_registry_empty(self.ptr) != 0 }
    }

    /// Compiled-in family names.
    pub fn families(&self) -> Result<Vec<String>> {
        let mut items = ptr::null_mut();
        let mut count = 0usize;
        check(unsafe { audio_cpp_registry_families(self.ptr, &mut items, &mut count) })?;
        take_string_list(items, count)
    }

    /// Path to a vendored `model_specs/{family}.json` when the source tree is present.
    ///
    /// Returns `None` if `family` is not a single path segment or the file is missing.
    /// Runtime discovery does not need this helper when specs are compiled in.
    #[must_use]
    pub fn bundled_model_spec(family: &str) -> Option<PathBuf> {
        if family.is_empty()
            || family.contains('/')
            || family.contains('\\')
            || family.contains("..")
        {
            return None;
        }
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../audio-cpp-sys/audio.cpp/model_specs")
            .join(format!("{family}.json"));
        path.is_file().then_some(path)
    }

    /// Whether `family` has a registered loader.
    #[must_use]
    pub fn supports_family(&self, family: &str) -> bool {
        let Ok(family) = std::ffi::CString::new(family) else {
            return false;
        };
        unsafe { audio_cpp_registry_supports_family(self.ptr, family.as_ptr()) != 0 }
    }

    /// Inspect a model path without loading weights.
    pub fn inspect(&self, request: &ModelLoadRequest) -> Result<ModelInspection> {
        let encoded = EncodedLoadRequest::new(request)?;
        let mut inspection = audio_cpp_sys::audio_cpp_inspection::default();
        check(unsafe { audio_cpp_registry_inspect(self.ptr, &encoded.request, &mut inspection) })?;
        take_inspection(inspection)
    }

    /// Load a model. The registry may be dropped after this returns.
    pub fn load(&self, request: &ModelLoadRequest) -> Result<Model> {
        let encoded = EncodedLoadRequest::new(request)?;
        let mut model = ptr::null_mut();
        check(unsafe { audio_cpp_registry_load(self.ptr, &encoded.request, &mut model) })?;
        Ok(unsafe { Model::from_ptr(model) })
    }
}

impl Drop for Registry {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { audio_cpp_registry_destroy(self.ptr) };
            self.ptr = ptr::null_mut();
        }
    }
}

/// Enumerate ggml backend devices currently available to the process.
pub fn backend_devices() -> Result<Vec<BackendDevice>> {
    list_backend_devices()
}
