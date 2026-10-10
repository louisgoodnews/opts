//! System clipboard access for the [`TextArea`](super::TextArea).

/// Clipboard handle with an in-process fallback.
///
/// `arboard::Clipboard` is created lazily and is not `Send` on every platform, so this lives in
/// the world as a non-send resource. When the system clipboard is unavailable (headless CI,
/// missing display server) copy and paste still work inside the application through the fallback
/// string, and the failure is logged once at `WARN`.
#[derive(Default)]
pub struct TextClipboard {
    backend: Option<arboard::Clipboard>,
    backend_failed: bool,
    fallback: String,
}

impl TextClipboard {
    /// A clipboard that never touches the system clipboard (tests, headless runs).
    #[must_use]
    pub fn in_memory() -> Self {
        Self {
            backend: None,
            backend_failed: true,
            fallback: String::new(),
        }
    }

    fn backend(&mut self) -> Option<&mut arboard::Clipboard> {
        if self.backend.is_none() && !self.backend_failed {
            match arboard::Clipboard::new() {
                Ok(c) => self.backend = Some(c),
                Err(e) => {
                    self.backend_failed = true;
                    tracing::warn!("system clipboard unavailable, using the in-app fallback: {e}");
                }
            }
        }
        self.backend.as_mut()
    }

    /// Stores `text` on the clipboard.
    pub fn set(&mut self, text: &str) {
        self.fallback = text.to_owned();
        if let Some(c) = self.backend() {
            if let Err(e) = c.set_text(text.to_owned()) {
                tracing::warn!("clipboard write failed: {e}");
            }
        }
    }

    /// Returns the clipboard text, falling back to the last in-app copy.
    pub fn get(&mut self) -> String {
        if let Some(c) = self.backend() {
            match c.get_text() {
                Ok(t) => return t,
                Err(e) => tracing::debug!("clipboard read failed, using the in-app copy: {e}"),
            }
        }
        self.fallback.clone()
    }
}
