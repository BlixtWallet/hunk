use std::ffi::c_void;
use std::sync::Arc;

use hunk_browser::BrowserFrame;

type BrowserFrameCleanup = unsafe extern "C" fn(*mut c_void);

unsafe extern "C" {
    fn hunk_register_browser_frame_item();
    fn hunk_browser_frame_publish(
        bgra: *const u8,
        len: usize,
        width: u32,
        height: u32,
        epoch: u64,
        owner: *mut c_void,
        cleanup: BrowserFrameCleanup,
    );
    fn hunk_browser_frame_clear();
}

pub fn register_browser_frame_item() {
    // SAFETY: The function has no arguments and only registers a Qt type once on the UI thread.
    unsafe { hunk_register_browser_frame_item() };
}

pub fn publish_browser_frame(frame: &BrowserFrame, presentation_epoch: u64) {
    let metadata = frame.metadata();
    let bgra = frame.shared_bgra();
    let data = bgra.as_ptr();
    let len = bgra.len();
    let owner = Box::into_raw(Box::new(bgra)).cast::<c_void>();
    // SAFETY: `owner` keeps `data` alive until Qt invokes the supplied cleanup callback. The
    // dimensions and buffer length were validated when BrowserFrame was constructed.
    unsafe {
        hunk_browser_frame_publish(
            data,
            len,
            metadata.width,
            metadata.height,
            presentation_epoch,
            owner,
            release_browser_frame,
        )
    };
}

unsafe extern "C" fn release_browser_frame(owner: *mut c_void) {
    if !owner.is_null() {
        // SAFETY: `owner` is produced exactly once by `Box::into_raw` above and Qt invokes this
        // callback exactly once after the final QImage reference releases the shared frame.
        unsafe { drop(Box::from_raw(owner.cast::<Arc<[u8]>>())) };
    }
}

pub fn clear_browser_frame() {
    // SAFETY: The function clears UI-thread-owned image state and retains no Rust pointer.
    unsafe { hunk_browser_frame_clear() };
}
