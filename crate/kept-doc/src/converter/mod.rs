//! Converter Traits and Registry
//!
//! Defines the traits for converting between platforms and Universal IR,
//! plus a registry for dynamic platform discovery.

use crate::ir::{Platform, UniversalDocument};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// NOTE-001: รวม Error ทั้งหมดที่เกิดจากการแปลงข้อมูลไว้ในซองเดียว (Unified Error Envelope)
#[derive(Error, Debug, Serialize, Deserialize)]
pub enum ConvertError {
    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Invalid data: {0}")]
    InvalidData(String),
    #[error("Conversion failed: {0}")]
    ConversionFailed(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Platform error from {platform}: {message}")]
    PlatformError { platform: String, message: String },
}

// NOTE-001: M2 - Reader/Writer Traits (Synchronous, Total functions over bytes)
// ช่วยให้ Registry จัดการ Crate ได้อย่างปลอดภัยโดยไม่ต้องใช้ Box<dyn Any>

/// Trait สำหรับการอ่านไฟล์จาก bytes เข้าสู่ Universal IR
pub trait Reader: Send + Sync {
    fn read(&self, input: &[u8]) -> Result<UniversalDocument, ConvertError>;
}

/// Trait สำหรับการเขียน Universal IR ออกเป็น bytes
pub trait Writer: Send + Sync {
    fn write(&self, doc: &UniversalDocument) -> Result<Vec<u8>, ConvertError>;
}

// NOTE-001: M4 - Source/Sink Traits (Asynchronous, for live platforms)

/// Trait สำหรับการดึงข้อมูลจาก API ภายนอกเข้าสู่ Universal IR
#[async_trait]
pub trait Source: Send + Sync {
    async fn fetch(
        &self,
        client: &crate::client::NotionClient,
        id: &str,
    ) -> Result<UniversalDocument, ConvertError>;
}

/// Trait สำหรับการส่งข้อมูลจาก Universal IR ไปยัง API ภายนอก
#[async_trait]
pub trait Sink: Send + Sync {
    async fn apply(
        &self,
        client: &crate::client::NotionClient,
        plan: &crate::sync::ChangeSet,
    ) -> Result<(), ConvertError>;
}

// --- Legacy Traits (Backward Compatibility) ---

/// Legacy Error type for conversion operations
#[derive(Error, Debug, Serialize, Deserialize)]
pub enum ConverterError {
    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Invalid data: {0}")]
    InvalidData(String),
    #[error("Conversion failed: {0}")]
    ConversionFailed(String),
    #[error("IO error: {0}")]
    IoError(String),
}

impl From<ConverterError> for ConvertError {
    fn from(err: ConverterError) -> Self {
        match err {
            ConverterError::UnsupportedPlatform(s) => ConvertError::UnsupportedPlatform(s),
            ConverterError::MissingField(s) => ConvertError::MissingField(s),
            ConverterError::InvalidData(s) => ConvertError::InvalidData(s),
            ConverterError::ConversionFailed(s) => ConvertError::ConversionFailed(s),
            ConverterError::IoError(s) => ConvertError::IoError(s),
        }
    }
}

/// Trait for converting FROM a platform TO Universal IR
pub trait FromPlatform {
    const PLATFORM: Platform;
    type Input;
    fn from_platform(input: Self::Input) -> Result<UniversalDocument, ConverterError>;
}

/// Trait for converting FROM Universal IR TO a platform
pub trait ToPlatform {
    const PLATFORM: Platform;
    type Output;
    fn to_platform(doc: &UniversalDocument) -> Result<Self::Output, ConverterError>;
}

/// Converter registry for dynamic platform discovery
/// NOTE-001: M2 - ปรับปรุง Registry ให้ใช้ Reader/Writer ที่เป็น Uniform Traits
/// กำจัดความจำเป็นในการใช้ Box<dyn Any> และ Downcasting ในการแปลงไฟล์ทั่วไป
#[derive(Default)]
pub struct ConverterRegistry {
    readers: HashMap<Platform, Box<dyn Reader>>,
    writers: HashMap<Platform, Box<dyn Writer>>,
    // NOTE-001: เก็บ Source/Sink สำหรับ live platforms
    sources: HashMap<Platform, Box<dyn Source>>,
    sinks: HashMap<Platform, Box<dyn Sink>>,
}

impl ConverterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_reader(&mut self, platform: Platform, reader: Box<dyn Reader>) {
        self.readers.insert(platform, reader);
    }

    pub fn register_writer(&mut self, platform: Platform, writer: Box<dyn Writer>) {
        self.writers.insert(platform, writer);
    }

    pub fn register_source(&mut self, platform: Platform, source: Box<dyn Source>) {
        self.sources.insert(platform, source);
    }

    pub fn register_sink(&mut self, platform: Platform, sink: Box<dyn Sink>) {
        self.sinks.insert(platform, sink);
    }

    pub fn get_reader(&self, platform: Platform) -> Option<&dyn Reader> {
        self.readers.get(&platform).map(|r| r.as_ref())
    }

    pub fn get_writer(&self, platform: Platform) -> Option<&dyn Writer> {
        self.writers.get(&platform).map(|w| w.as_ref())
    }

    pub fn get_source(&self, platform: Platform) -> Option<&dyn Source> {
        self.sources.get(&platform).map(|s| s.as_ref())
    }

    pub fn get_sink(&self, platform: Platform) -> Option<&dyn Sink> {
        self.sinks.get(&platform).map(|s| s.as_ref())
    }

    pub fn available_platforms(&self) -> Vec<Platform> {
        let mut platforms: Vec<Platform> = self.readers.keys().cloned().collect();
        platforms.extend(self.writers.keys().cloned());
        platforms.sort_by_key(|p| format!("{:?}", p));
        platforms.dedup();
        platforms
    }
}

pub mod docx;
pub mod filter;
pub mod github_markdown;
pub mod lark_sheets;
pub mod markdown;
pub mod markdown_frontmatter;
pub mod notion;
pub mod obsidian_base;
pub mod pdf;

pub use docx::DocxAdapter;
pub use filter::{Filter, FilterPipeline, MarkdownAlertFilter, ThaiSanitizationFilter};
pub use obsidian_base::ObsidianBaseAdapter;
pub use pdf::PdfAdapter;

// --- Blanket Implementations for M2 ---

/// NOTE-001: M2 - Blanket implementation สำหรับ Reader
/// ช่วยให้ Adapter ที่เป็น String-based (เช่น Markdown) ย้ายมาใช้ Reader ได้ทันที
impl<T> Reader for T
where
    T: FromPlatform<Input = String> + Send + Sync,
{
    fn read(&self, input: &[u8]) -> Result<UniversalDocument, ConvertError> {
        let s = std::str::from_utf8(input)
            .map_err(|e| ConvertError::InvalidData(format!("UTF-8 error: {}", e)))?;
        T::from_platform(s.to_string()).map_err(ConvertError::from)
    }
}

/// NOTE-001: M2 - Blanket implementation สำหรับ Writer
/// ช่วยให้ Adapter ที่เป็น String-based ย้ายมาใช้ Writer ได้ทันที
impl<T> Writer for T
where
    T: ToPlatform<Output = String> + Send + Sync,
{
    fn write(&self, doc: &UniversalDocument) -> Result<Vec<u8>, ConvertError> {
        let s = T::to_platform(doc).map_err(ConvertError::from)?;
        Ok(s.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::{ConverterRegistry, Platform};
    use crate::converter::markdown::MarkdownConverter;

    #[test]
    fn typed_reader_writer_registration_discovers_markdown_without_legacy_dispatch() {
        let mut registry = ConverterRegistry::new();
        registry.register_reader(Platform::Markdown, Box::new(MarkdownConverter));
        registry.register_writer(Platform::Markdown, Box::new(MarkdownConverter));

        assert!(registry.get_reader(Platform::Markdown).is_some());
        assert!(registry.get_writer(Platform::Markdown).is_some());
        assert_eq!(registry.available_platforms(), vec![Platform::Markdown]);
    }
}
