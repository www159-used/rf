use crate::core::{ClipboardPort, Error, Result};
use std::path::Path;

#[cfg(target_os = "macos")]
fn lock_pasteboard() -> Result<std::fs::File> {
    use std::{
        fs::{self, OpenOptions},
        os::unix::fs::OpenOptionsExt,
    };
    let store = crate::core::ArchiveStore::system()?;
    let directory = store.root().parent().expect("system store has a parent");
    fs::create_dir_all(directory)
        .map_err(|e| Error::Message(format!("Cannot create the clipboard lock directory: {e}")))?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(directory.join("clipboard.lock"))
        .map_err(|e| Error::Message(format!("Cannot open the clipboard lock: {e}")))?;
    if !lock
        .metadata()
        .map_err(|e| Error::Message(e.to_string()))?
        .is_file()
    {
        return Err(Error::Message(
            "Clipboard lock is not a regular file".into(),
        ));
    }
    fs2::FileExt::lock_exclusive(&lock)
        .map_err(|e| Error::Message(format!("Cannot acquire the clipboard lock: {e}")))?;
    Ok(lock)
}

#[derive(Default)]
pub struct SystemClipboard;

#[cfg(target_os = "macos")]
impl ClipboardPort for SystemClipboard {
    fn set_text(&mut self, text: &str) -> Result<()> {
        let _lock = lock_pasteboard()?;
        use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
        use objc2_foundation::NSString;
        objc2::rc::autoreleasepool(|_| {
            let pb = NSPasteboard::generalPasteboard();
            let value = NSString::from_str(text);
            pb.clearContents();
            // Apple's exported immutable NSString constant lives for the process lifetime.
            if pb.setString_forType(&value, unsafe { NSPasteboardTypeString }) {
                Ok(())
            } else {
                Err(Error::Message(
                    "macOS could not write path text to the clipboard".into(),
                ))
            }
        })
    }
    fn set_file(&mut self, path: &Path) -> Result<()> {
        let _lock = lock_pasteboard()?;
        use objc2::runtime::ProtocolObject;
        use objc2_app_kit::{NSPasteboard, NSPasteboardTypeFileURL, NSPasteboardWriting};
        use objc2_foundation::{NSArray, NSString, NSURL};
        let value = path.to_str().ok_or_else(|| {
            Error::Message("File path cannot be represented as valid Unicode".into())
        })?;
        objc2::rc::autoreleasepool(|_| {
            // Initialize pasteboard support before constructing the file URL.
            let pb = NSPasteboard::generalPasteboard();
            let url = NSURL::fileURLWithPath_isDirectory(&NSString::from_str(value), false);
            let object: &ProtocolObject<dyn NSPasteboardWriting> = ProtocolObject::from_ref(&*url);
            let objects = NSArray::from_slice(&[object]);
            pb.clearContents();
            // A write alone can return before the file representation is available
            // to the next process. Read it back while the URL is still alive;
            // this also rejects an empty or mismatched successful write.
            if pb.writeObjects(&objects)
                && pb.stringForType(unsafe { NSPasteboardTypeFileURL }) == url.absoluteString()
            {
                Ok(())
            } else {
                Err(Error::Message(
                    "macOS could not write or verify the clipboard file reference".into(),
                ))
            }
        })
    }
}

#[cfg(not(target_os = "macos"))]
impl ClipboardPort for SystemClipboard {
    fn set_text(&mut self, _: &str) -> Result<()> {
        Err(Error::Message(
            "This version supports the macOS clipboard only".into(),
        ))
    }
    fn set_file(&mut self, _: &Path) -> Result<()> {
        Err(Error::Message(
            "This version supports the macOS clipboard only".into(),
        ))
    }
}
