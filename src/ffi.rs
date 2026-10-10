//! Versioned C ABI. Caller-owned byte buffers avoid cross-allocator ownership.
//! Handles are monotonic integers, not pointers into Rust memory.
use crate::{
    RedactionError,
    engine::{self, Redactor},
};
use std::{
    collections::HashMap,
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ok = 0,
    InvalidArgument = 1,
    InvalidHandle = 2,
    Disabled = 3,
    UnsupportedInput = 4,
    ArtifactError = 5,
    RuntimeError = 6,
    InferenceError = 7,
    BufferTooSmall = 8,
    InternalError = 9,
}
impl From<RedactionError> for Status {
    fn from(error: RedactionError) -> Self {
        match error {
            RedactionError::Artifact => Self::ArtifactError,
            RedactionError::Runtime => Self::RuntimeError,
            RedactionError::Inference => Self::InferenceError,
            RedactionError::Input => Self::UnsupportedInput,
        }
    }
}
struct Entry {
    redactor: Redactor,
    closed: bool,
}
struct Registry {
    next: u64,
    entries: HashMap<u64, Arc<Mutex<Entry>>>,
}
fn registry() -> &'static Mutex<Registry> {
    static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        Mutex::new(Registry {
            next: 1,
            entries: HashMap::new(),
        })
    })
}
fn entry(handle: u64) -> Result<Arc<Mutex<Entry>>, Status> {
    registry()
        .lock()
        .map_err(|_| Status::InternalError)?
        .entries
        .get(&handle)
        .cloned()
        .ok_or(Status::InvalidHandle)
}
fn boundary(f: impl FnOnce() -> Result<(), Status>) -> i32 {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => Status::Ok as i32,
        Ok(Err(status)) => status as i32,
        Err(_) => Status::InternalError as i32,
    }
}
// Pointer validity is the foreign caller's obligation. Bounds precede all reads.
unsafe fn utf8<'a>(pointer: *const u8, length: usize, maximum: usize) -> Result<&'a str, Status> {
    if length > maximum || pointer.is_null() {
        return Err(Status::InvalidArgument);
    }
    let bytes = unsafe { std::slice::from_raw_parts(pointer, length) };
    std::str::from_utf8(bytes).map_err(|_| Status::UnsupportedInput)
}

#[unsafe(no_mangle)]
pub extern "C" fn defingerprinter_abi_version() -> u32 {
    1
}

/// Create a disabled context without examining any diagnostic or model file.
/// # Safety
/// Path pointers must be readable for the stated lengths, or both NULL/0 for Nix defaults.
/// `out_handle` must be valid writable u64 memory disjoint from the path slices.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn defingerprinter_create(
    model_dir: *const u8,
    model_dir_len: usize,
    runtime: *const u8,
    runtime_len: usize,
    out_handle: *mut u64,
) -> i32 {
    if out_handle.is_null() {
        return Status::InvalidArgument as i32;
    }
    unsafe {
        out_handle.write(0);
    }
    boundary(|| {
        let (directory, runtime) =
            if model_dir.is_null() && model_dir_len == 0 && runtime.is_null() && runtime_len == 0 {
                engine::compiled_paths().ok_or(Status::ArtifactError)?
            } else {
                let directory = unsafe { utf8(model_dir, model_dir_len, 4096)? };
                let runtime = unsafe { utf8(runtime, runtime_len, 4096)? };
                if directory.contains('\0') || runtime.contains('\0') {
                    return Err(Status::InvalidArgument);
                }
                (PathBuf::from(directory), PathBuf::from(runtime))
            };
        if !directory.is_absolute() || !runtime.is_absolute() {
            return Err(Status::InvalidArgument);
        }
        let mut registry = registry().lock().map_err(|_| Status::InternalError)?;
        let handle = registry.next;
        registry.next = handle.checked_add(1).ok_or(Status::InternalError)?;
        registry.entries.insert(
            handle,
            Arc::new(Mutex::new(Entry {
                redactor: Redactor::new(directory, runtime),
                closed: false,
            })),
        );
        unsafe {
            out_handle.write(handle);
        }
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn defingerprinter_set_consent(handle: u64, enabled: i32) -> i32 {
    boundary(|| {
        if enabled != 0 && enabled != 1 {
            return Err(Status::InvalidArgument);
        }
        let entry = entry(handle)?;
        let mut entry = entry.lock().map_err(|_| Status::InternalError)?;
        if entry.closed {
            return Err(Status::InvalidHandle);
        }
        entry.redactor.set_consent(enabled == 1);
        Ok(())
    })
}

/// Redact into a caller-owned buffer. No source text is returned on error.
/// # Safety
/// `written` must be writable and disjoint from all slices. On an enabled context,
/// input/language must be readable and output writable for the stated lengths.
/// Each concurrent call must own distinct output and `written` storage.
/// Pointers are not dereferenced when a disabled/stale handle rejects the call,
/// except for `written`, which is always set to zero before processing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn defingerprinter_protect(
    handle: u64,
    language: *const u8,
    language_len: usize,
    input: *const u8,
    input_len: usize,
    output: *mut u8,
    output_capacity: usize,
    written: *mut usize,
) -> i32 {
    if written.is_null() {
        return Status::InvalidArgument as i32;
    }
    unsafe {
        written.write(0);
    }
    boundary(|| {
        let entry = entry(handle)?;
        let mut entry = entry.lock().map_err(|_| Status::InternalError)?;
        if entry.closed {
            return Err(Status::InvalidHandle);
        }
        if !entry.redactor.enabled() {
            return Err(Status::Disabled);
        }
        if language_len != 2 || input_len > engine::MAX_INPUT_BYTES {
            return Err(Status::UnsupportedInput);
        }
        if output_capacity < engine::OUTPUT_CAPACITY {
            return Err(Status::BufferTooSmall);
        }
        if output.is_null() {
            return Err(Status::InvalidArgument);
        }
        let language = unsafe { utf8(language, language_len, 2)? };
        let input = if input_len == 0 {
            ""
        } else {
            unsafe { utf8(input, input_len, engine::MAX_INPUT_BYTES)? }
        };
        let text = entry
            .redactor
            .protect(language, input)
            .map_err(Status::from)?;
        // The lock covers publication too: after revocation returns, no earlier call can publish text.
        unsafe {
            std::ptr::copy_nonoverlapping(text.as_ptr(), output, text.len());
            written.write(text.len());
        }
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn defingerprinter_destroy(handle: u64) -> i32 {
    boundary(|| {
        let entry = registry()
            .lock()
            .map_err(|_| Status::InternalError)?
            .entries
            .remove(&handle)
            .ok_or(Status::InvalidHandle)?;
        let mut entry = entry.lock().map_err(|_| Status::InternalError)?;
        entry.closed = true;
        entry.redactor.set_consent(false);
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn create() -> u64 {
        let model = b"/missing/defingerprinter-model";
        let runtime = b"/missing/defingerprinter-runtime";
        let mut handle = 0;
        assert_eq!(
            unsafe {
                defingerprinter_create(
                    model.as_ptr(),
                    model.len(),
                    runtime.as_ptr(),
                    runtime.len(),
                    &mut handle,
                )
            },
            0
        );
        handle
    }
    #[test]
    fn disabled_context_never_reads_payload_or_output() {
        let handle = create();
        let mut written = 9;
        assert_eq!(
            unsafe {
                defingerprinter_protect(
                    handle,
                    std::ptr::dangling::<u8>(),
                    2,
                    std::ptr::dangling::<u8>(),
                    100,
                    std::ptr::dangling_mut::<u8>(),
                    4096,
                    &mut written,
                )
            },
            Status::Disabled as i32
        );
        assert_eq!(written, 0);
        assert_eq!(defingerprinter_destroy(handle), 0);
        assert_eq!(
            defingerprinter_destroy(handle),
            Status::InvalidHandle as i32
        );
    }
    #[test]
    fn admission_errors_clear_length_and_do_not_load_model() {
        let handle = create();
        assert_eq!(defingerprinter_set_consent(handle, 1), 0);
        let mut output = [0u8; 4096];
        let mut written = 99;
        for input in ["Αlice", "١٢٣٤٥٦٧٨٩", "Alice\0Johnson"] {
            assert_eq!(
                unsafe {
                    defingerprinter_protect(
                        handle,
                        b"en".as_ptr(),
                        2,
                        input.as_ptr(),
                        input.len(),
                        output.as_mut_ptr(),
                        output.len(),
                        &mut written,
                    )
                },
                Status::UnsupportedInput as i32
            );
            assert_eq!(written, 0);
        }
        assert_eq!(
            unsafe {
                defingerprinter_protect(
                    handle,
                    b"en".as_ptr(),
                    2,
                    b"Alice".as_ptr(),
                    5,
                    output.as_mut_ptr(),
                    output.len(),
                    &mut written,
                )
            },
            Status::ArtifactError as i32
        );
        assert_eq!(written, 0);
        assert_eq!(defingerprinter_set_consent(handle, 0), 0);
        assert_eq!(defingerprinter_destroy(handle), 0);
    }
    #[test]
    fn panic_cannot_cross_c_abi() {
        assert_eq!(
            boundary(|| panic!("synthetic boundary test")),
            Status::InternalError as i32
        );
    }
    #[test]
    fn handles_are_unique_under_concurrent_creation() {
        let threads: Vec<_> = (0..8)
            .map(|_| {
                std::thread::spawn(|| {
                    let h = create();
                    assert_eq!(defingerprinter_destroy(h), 0);
                    h
                })
            })
            .collect();
        let mut handles: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        handles.sort();
        handles.dedup();
        assert_eq!(handles.len(), 8);
    }
}
