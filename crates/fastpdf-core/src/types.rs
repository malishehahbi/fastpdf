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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSpan {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMatch {
    pub page_index: u32,
    pub start: usize,
    pub end: usize,
    pub snippet: String,
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
