use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineItem {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_index: Option<u32>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<OutlineItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    pub index: u32,
    pub width: f32,
    pub height: f32,
    pub rotation: u32,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentInfo {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub modification_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationDto {
    pub subtype: String,
    pub rect: Rect,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<String>,
    pub flags: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_index: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkDto {
    pub rect: Rect,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_index: Option<u32>,
}

/// `span_index` value in [`CharMapEntry`] for synthetic space/newline characters in [`PagePlain`].
pub const SYNTHETIC_CHAR_MAP: usize = usize::MAX;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CharMapEntry {
    pub span_index: usize,
    pub offset_in_span: u32,
}

impl CharMapEntry {
    pub const fn synthetic() -> Self {
        Self {
            span_index: SYNTHETIC_CHAR_MAP,
            offset_in_span: 0,
        }
    }

    pub fn is_synthetic(self) -> bool {
        self.span_index == SYNTHETIC_CHAR_MAP
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextLine {
    pub span_indices: Vec<usize>,
    pub rtl: bool,
    pub column: u32,
    pub y_top: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagePlain {
    pub text: String,
    pub char_map: Vec<CharMapEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSpan {
    pub text: String,
    /// PDF user space (origin bottom-left); links / legacy highlights.
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// Hayro view space: distance from page top in points (matches raster).
    #[serde(default)]
    pub y_top: f32,
    /// Typographic size in view points (em height).
    #[serde(default)]
    pub font_size: f32,
    #[serde(default)]
    pub space_before: bool,
    #[serde(default)]
    pub line_break_before: bool,
    #[serde(default)]
    pub rtl: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMatch {
    pub page_index: u32, // serialized as pageIndex
    pub start: usize,
    pub end: usize,
    pub snippet: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rects: Vec<Rect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormFieldDto {
    pub name: String,
    pub field_type: String,
  #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    pub flags: u32,
    pub read_only: bool,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionsDto {
    pub print: bool,
    pub modify: bool,
    pub copy: bool,
    pub annotate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderOutput {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}
