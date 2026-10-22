//! Turns the shell's output bytes into text without breaking characters. A read can end in the middle of a
//! character (an emoji is four bytes); those bytes wait for the next read instead of becoming "?".

#[derive(Default)]
pub struct Utf8Chunker {
    pending: Vec<u8>,
}

impl Utf8Chunker {
    pub fn new() -> Self { Self::default() }

    /// The text in `bytes` (after anything held back from last time). A cut-off character at the end is held
    /// back; bytes that can never be valid UTF-8 become U+FFFD, as a terminal shows them.
    pub fn push(&mut self, bytes: &[u8]) -> String {
        self.pending.extend_from_slice(bytes);
        let mut out = String::with_capacity(self.pending.len());
        let mut rest: &[u8] = &self.pending;
        loop {
            match std::str::from_utf8(rest) {
                Ok(s) => { out.push_str(s); rest = &[]; break; }
                Err(e) => {
                    let good = e.valid_up_to();
                    // Safe: from_utf8 said these bytes are valid.
                    out.push_str(unsafe { std::str::from_utf8_unchecked(&rest[..good]) });
                    match e.error_len() {
                        Some(bad) => { out.push('\u{FFFD}'); rest = &rest[good + bad..]; }
                        None => { rest = &rest[good..]; break; }   // cut off: keep for the next read
                    }
                }
            }
        }
        self.pending = rest.to_vec();
        out
    }

    /// Whatever is still held back, at the end of the output.
    pub fn finish(&mut self) -> String {
        let s = String::from_utf8_lossy(&self.pending).into_owned();
        self.pending.clear();
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_text_passes_through() {
        assert_eq!(Utf8Chunker::new().push("héllo ✓".as_bytes()), "héllo ✓");
    }

    #[test]
    fn a_character_split_across_reads_is_kept_whole() {
        let fire = "🔥".as_bytes();
        let mut c = Utf8Chunker::new();
        assert_eq!(c.push(&[b'a', fire[0], fire[1]]), "a");
        assert_eq!(c.push(&[fire[2]]), "");
        assert_eq!(c.push(&[fire[3], b'b']), "🔥b");
    }

    #[test]
    fn bad_bytes_become_replacement_characters() {
        let mut c = Utf8Chunker::new();
        assert_eq!(c.push(&[b'a', 0xff, b'b']), "a\u{FFFD}b");
        assert_eq!(c.push(&[0xe2, 0x82]), "");
        assert_eq!(c.finish(), "\u{FFFD}");
    }
}
