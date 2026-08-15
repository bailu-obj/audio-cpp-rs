//! Inspect a model and run one offline task without assuming a family.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use audio_cpp::{
    BackendConfig, BackendType, ModelLoadRequest, Registry, SessionOptions, TaskRequest, TaskSpec,
    VoiceTaskKind,
};

fn main() -> audio_cpp::Result<()> {
    let path = std::env::var("AUDIO_CPP_TEST_MODEL").unwrap_or_else(|_| {
        eprintln!("Set AUDIO_CPP_TEST_MODEL to a model file or directory.");
        std::process::exit(2);
    });
    let family = std::env::var("AUDIO_CPP_TEST_FAMILY").ok();
    let text = std::env::var("AUDIO_CPP_TEST_TEXT").unwrap_or_else(|_| {
        "(A calm studio narrator with clear articulation) Hello from audio-cpp.".into()
    });

    let registry = Registry::new()?;
    println!("families: {}", registry.families()?.join(", "));
    println!("devices:");
    for device in audio_cpp::backend_devices()? {
        println!(
            "  {}[{}] {} ({})",
            device.backend, device.index, device.name, device.kind
        );
    }

    let mut request = ModelLoadRequest::new(&path);
    if let Some(family) = family {
        request = request.family_hint(family);
    }
    if let Some(spec) = std::env::var_os("AUDIO_CPP_TEST_MODEL_SPEC") {
        request = request.model_spec_override(spec);
    } else if let Some(family) = request.family_hint.as_deref() {
        if let Some(spec) = Registry::bundled_model_spec(family) {
            request = request.model_spec_override(spec);
        }
    }

    let inspection = registry.inspect(&request)?;
    println!(
        "family={} variant={} tasks={}",
        inspection.metadata.family,
        inspection.metadata.variant,
        inspection.capabilities.tasks.len()
    );

    let model = registry.load(&request)?;
    let task = inspection
        .capabilities
        .tasks
        .iter()
        .find(|task| task.task == VoiceTaskKind::Tts)
        .or_else(|| inspection.capabilities.tasks.first())
        .expect("model advertises no tasks");

    let options = SessionOptions::new().backend(select_backend()?);
    let mut session = model.create_session(TaskSpec::offline(task.task), &options)?;
    println!(
        "session family={} task={} mode={}",
        session.family()?,
        session.task_kind()?,
        session.run_mode()?
    );

    let task_request = TaskRequest::new().text(text);
    let result = session.prepare_and_run(&task_request)?;
    if let Some(audio) = result.audio {
        println!(
            "audio {} Hz, {} ch, {} samples ({:.2}s)",
            audio.sample_rate,
            audio.channels,
            audio.samples.len(),
            f64::from(u32::try_from(audio.samples.len()).unwrap_or(u32::MAX))
                / f64::from(audio.sample_rate.max(1) * audio.channels.max(1))
        );
        let output = match std::env::var("AUDIO_CPP_TEST_OUTPUT") {
            Ok(path) => PathBuf::from(path),
            Err(_) => PathBuf::from(format!("{}.wav", inspection.metadata.family)),
        };
        write_wav(&output, &audio.samples, audio.sample_rate, audio.channels)
            .map_err(|error| audio_cpp::Error::Unsupported(error.to_string()))?;
        println!("wrote {}", output.display());
    } else {
        eprintln!("no audio output");
        std::process::exit(1);
    }
    if let Some(text) = result.text {
        println!("text: {}", text.text);
    }
    Ok(())
}

fn select_backend() -> audio_cpp::Result<BackendConfig> {
    let threads =
        i32::try_from(std::thread::available_parallelism().map_or(4, std::num::NonZero::get))
            .unwrap_or(4);
    if let Ok(name) = std::env::var("AUDIO_CPP_TEST_BACKEND") {
        return Ok(BackendConfig {
            backend: name.parse()?,
            device: 0,
            threads,
        });
    }
    let has_metal = audio_cpp::backend_devices()?
        .iter()
        .any(|device| device.backend.eq_ignore_ascii_case("METAL"));
    Ok(BackendConfig {
        backend: if has_metal {
            BackendType::Metal
        } else {
            BackendType::BestAvailable
        },
        device: 0,
        threads,
    })
}

fn write_wav(
    path: &std::path::Path,
    samples: &[f32],
    sample_rate: i32,
    channels: i32,
) -> std::io::Result<()> {
    let mut pcm = Vec::with_capacity(samples.len() * 2);
    for sample in samples {
        let clipped = sample.clamp(-1.0, 1.0);
        #[allow(clippy::cast_possible_truncation)]
        let value = (clipped * 32767.0).round() as i16;
        pcm.extend_from_slice(&value.to_le_bytes());
    }

    let data_len = u32::try_from(pcm.len()).expect("wav data too large");
    let byte_rate = u32::try_from(sample_rate * channels * 2).expect("invalid wav rate");
    let block_align = u16::try_from(channels * 2).expect("invalid wav channels");
    let mut file = BufWriter::new(File::create(path)?);
    file.write_all(b"RIFF")?;
    file.write_all(&(36 + data_len).to_le_bytes())?;
    file.write_all(b"WAVEfmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(
        &u16::try_from(channels)
            .expect("invalid channels")
            .to_le_bytes(),
    )?;
    file.write_all(
        &u32::try_from(sample_rate)
            .expect("invalid sample rate")
            .to_le_bytes(),
    )?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&16u16.to_le_bytes())?;
    file.write_all(b"data")?;
    file.write_all(&data_len.to_le_bytes())?;
    file.write_all(&pcm)?;
    file.flush()
}
