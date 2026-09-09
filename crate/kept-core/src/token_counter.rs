//! LLM token counter — ported from SQZ token_counter.rs
//! Source: D:\01work\Active\references\campbellr\sqz\sqz_engine\src\token_counter.rs
//!
//! Uses tiktoken BPE singletons for exact counting.
//! Supports cl100k_base (GPT-4/Claude) and o200k_base (GPT-4o/o1/o3).
//! Fast fallback: ceil(chars / 4) for unknown models.

/// Encoding model family — determines which BPE vocabulary to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFamily {
    /// OpenAI GPT-4/4o, o1/o3 — uses o200k_base
    OpenAi,
    /// Anthropic Claude — approximate via cl100k_base (~5% variance)
    Claude,
    /// Google Gemini — approximate via cl100k_base
    Gemini,
    /// Local/unknown model — fast heuristic fallback
    Local,
}

/// BPE token counter using tiktoken-rs singletons.
///
/// Construction is essentially free — BPE data is lazily loaded once.
pub struct TokenCounter;

impl TokenCounter {
    pub fn new() -> Self {
        Self
    }

    /// Count tokens for `text` using the tokenizer appropriate for `model`.
    pub fn count(&self, text: &str, model: ModelFamily) -> u32 {
        match model {
            ModelFamily::OpenAi => self.count_o200k(text),
            ModelFamily::Claude | ModelFamily::Gemini => self.count_cl100k(text),
            ModelFamily::Local => Self::count_fast(text),
        }
    }

    /// Fast character-based approximation: `ceil(chars / 4)`.
    pub fn count_fast(text: &str) -> u32 {
        ((text.len() as f64) / 4.0).ceil() as u32
    }

    // NOTE-001: tiktoken-rs 0.12 singleton คืน &CoreBPE ที่ sync ภายในแล้ว จึงเรียกใช้ตรง ๆ ได้โดยไม่ต้อง lock
    fn count_cl100k(&self, text: &str) -> u32 {
        let bpe = tiktoken_rs::cl100k_base_singleton();
        bpe.encode_with_special_tokens(text).len() as u32
    }

    fn count_o200k(&self, text: &str) -> u32 {
        let bpe = tiktoken_rs::o200k_base_singleton();
        bpe.encode_with_special_tokens(text).len() as u32
    }
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_empty() {
        let tc = TokenCounter::new();
        assert_eq!(tc.count("", ModelFamily::OpenAi), 0);
        assert_eq!(tc.count("", ModelFamily::Claude), 0);
        assert_eq!(TokenCounter::count_fast(""), 0);
    }

    #[test]
    fn count_hello_world() {
        let tc = TokenCounter::new();
        let openai = tc.count("Hello, world!", ModelFamily::OpenAi);
        let claude = tc.count("Hello, world!", ModelFamily::Claude);
        assert!(openai > 0 && openai < 10);
        assert!(claude > 0 && claude < 10);
    }

    #[test]
    fn count_fast_basic() {
        assert_eq!(TokenCounter::count_fast("abcd"), 1);
        assert_eq!(TokenCounter::count_fast("abcde"), 2);
    }

    #[test]
    fn local_model_uses_fallback() {
        let tc = TokenCounter::new();
        assert_eq!(tc.count("abcdefgh", ModelFamily::Local), 2);
    }
}
