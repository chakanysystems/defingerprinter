//! Version-2 C ABI with caller-owned UTF-8 byte buffers.
//!
//! Use the declarations in `include/defingerprinter.h` and check
//! [`defingerprinter_abi_version`] before creating a context. Handles are opaque,
//! monotonically allocated `u64` integers, not pointers. Create a handle, protect
//! text, then destroy the handle. No artifact loading occurs until an admitted
//! request. Collection and privacy policy are the caller's responsibility.
//!
//! Every operation other than the version query returns an `i32` [`Status`].
//! Only status zero permits consumption of output. On failure, `written` is
//! zero and the output buffer must be ignored: it may contain an earlier result.
//! Output is UTF-8 with an explicit byte count, without a NUL terminator. Allocate
//! at least [`engine::OUTPUT_CAPACITY`] bytes even for short or empty inputs.
//! There is no size-query or retained-result retry protocol.
//!
//! # Ownership and concurrency
//!
//! Rust copies path strings during creation and borrows diagnostic input only
//! for the duration of a call. Allocate and release byte buffers in the calling
//! language; there is no Rust-allocated string to free across the ABI.
//! Calls serialize per handle, including output publication. Different handles
//! may run concurrently. Destruction waits for earlier in-flight work on that
//! handle; after successful destruction returns, earlier calls cannot publish
//! more output. Supply separate writable storage to concurrent
//! calls. Never unload the library with live handles or calls in flight.
//! The ONNX Runtime is process-global; use one compatible runtime across handles.
//!
//! Invalid pointer addresses cannot be validated by Rust. Catching Rust panics
//! at the ABI boundary does not make invalid foreign memory safe.
//!
//! ```
//! use defingerprinter::ffi::{defingerprinter_abi_version, Status};
//! assert_eq!(defingerprinter_abi_version(), 2);
//! assert_eq!(Status::Ok as i32, 0);
//! ```
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

/// Numeric result codes for ABI version 2. Code 3 is reserved.
///
/// Treat every nonzero value, including unknown future values, as failure.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Success; output and its reported byte count may be consumed.
    Ok = 0,
    /// Invalid path, pointer/length combination, or output pointer.
    InvalidArgument = 1,
    /// Unknown, destroyed, or already-closed handle.
    InvalidHandle = 2,
    /// Unsupported tag, invalid UTF-8 diagnostic/path text, or rejected input policy.
    UnsupportedInput = 4,
    /// Missing compile-time paths or invalid pinned model artifacts.
    ArtifactError = 5,
    /// Native runtime loading or model session creation failed.
    RuntimeError = 6,
    /// Tensor creation, model execution, or output validation failed.
    InferenceError = 7,
    /// Output capacity is below [`engine::OUTPUT_CAPACITY`]; no size query is supported.
    BufferTooSmall = 8,
    /// A caught Rust panic, poisoned lock, or internal registry failure.
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
    redactor: Option<Redactor>,
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

/// Return the supported C ABI version, currently `2`.
///
/// This query does not create a context or load any artifacts.
#[unsafe(no_mangle)]
pub extern "C" fn defingerprinter_abi_version() -> u32 {
    2
}

/// Create a context without examining any diagnostic or model file.
///
/// Supply two absolute UTF-8 paths, each at most 4,096 bytes and without NUL
/// characters: the model directory and native runtime shared library. Lengths
/// exclude any terminator. Pass NULL/0 for **both** paths to select
/// [`engine::compiled_paths`]; missing defaults return [`Status::ArtifactError`].
/// Paths are copied but files are not checked until protection is attempted.
/// `out_handle` is set to zero before processing and receives a nonzero handle
/// only on success. Destroy that handle with [`defingerprinter_destroy`].
///
/// # Safety
///
/// Non-default path pointers must reference initialized, readable byte slices
/// of the stated lengths, valid for this call and within one allocation.
/// `out_handle` must be aligned, valid writable `u64` memory, disjoint from both
/// path slices. Do not concurrently access that memory while this call writes it.
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
                redactor: Some(Redactor::new(directory, runtime)),
                closed: false,
            })),
        );
        unsafe {
            out_handle.write(handle);
        }
        Ok(())
    })
}

/// Redact UTF-8 text into a caller-owned buffer, returning a numeric [`Status`].
///
/// `language` is an exact two-byte tag (`en`, `es`, `fr`, `de`, `it`, `pt`, `nl`).
/// Input must satisfy [`engine::supported`]. Empty input may use NULL/0.
/// Output capacity must be at least [`engine::OUTPUT_CAPACITY`] bytes. On success,
/// the first `*written` bytes contain UTF-8, without a NUL terminator; the count
/// is a byte count, not a character count. Output may be truncated to
/// [`engine::MAX_OUTPUT_CHARS`] Unicode scalar values.
///
/// `written` is zeroed before handle checks. Stale handles reject the call
/// before dereferencing language, input, or output.
/// On every failure, ignore the output buffer, including any previous contents.
/// No source text or result is retained for retries. Rust panics caught at the
/// boundary return [`Status::InternalError`].
///
/// # Safety
///
/// `written` must be aligned, valid writable `usize` memory, disjoint from all
/// byte slices for the entire call. Language and nonempty
/// input must reference initialized readable memory for their stated lengths;
/// output must reference writable memory for its stated capacity. Each slice
/// must lie within one allocation and satisfy Rust's slice size requirements.
/// Keep input and language immutable during the call and use separate output
/// and `written` storage for each concurrent call. Do not access output while
/// the call writes it. The library neither takes ownership nor frees buffers.
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
            .as_mut()
            .ok_or(Status::InvalidHandle)?
            .protect(language, input)
            .map_err(Status::from)?;
        // The lock covers publication: destruction waits until output is complete.
        unsafe {
            std::ptr::copy_nonoverlapping(text.as_ptr(), output, text.len());
            written.write(text.len());
        }
        Ok(())
    })
}

/// Remove a handle, wait for outstanding work, and drop its model session.
///
/// Safe for concurrent callers. Unknown, stale, and duplicate handles return
/// [`Status::InvalidHandle`]. Destruction does not reset the process-global
/// runtime. Do not unload the library until all handles have
/// been destroyed and all calls have returned.
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
        drop(entry.redactor.take());
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
    fn destruction_closes_existing_entry_references() {
        let handle = create();
        let retained = entry(handle).unwrap();
        assert_eq!(defingerprinter_destroy(handle), Status::Ok as i32);
        let entry = retained.lock().unwrap();
        assert!(entry.closed);
        assert!(entry.redactor.is_none());
        assert_eq!(
            defingerprinter_destroy(handle),
            Status::InvalidHandle as i32
        );
        let mut written = 9;
        assert_eq!(
            unsafe {
                defingerprinter_protect(
                    handle,
                    std::ptr::null(),
                    2,
                    std::ptr::null(),
                    100,
                    std::ptr::null_mut(),
                    0,
                    &mut written,
                )
            },
            Status::InvalidHandle as i32
        );
        assert_eq!(written, 0);
    }
    #[test]
    fn destruction_waits_for_locked_entry() {
        let handle = create();
        let retained = entry(handle).unwrap();
        let guard = retained.lock().unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        let destroyer = std::thread::spawn(move || {
            sender.send(defingerprinter_destroy(handle)).unwrap();
        });
        // Removal precedes locking: wait until destruction reaches the entry lock.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while entry(handle).is_ok() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(matches!(
            receiver.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
        drop(guard);
        assert_eq!(
            receiver
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap(),
            Status::Ok as i32
        );
        destroyer.join().unwrap();
        assert!(retained.lock().unwrap().redactor.is_none());
    }
    #[test]
    fn null_written_is_rejected() {
        let handle = create();
        assert_eq!(
            unsafe {
                defingerprinter_protect(
                    handle,
                    std::ptr::null(),
                    2,
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                )
            },
            Status::InvalidArgument as i32
        );
        assert_eq!(defingerprinter_destroy(handle), Status::Ok as i32);
    }
    #[test]
    fn admission_errors_clear_length_and_do_not_load_model() {
        let handle = create();
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
