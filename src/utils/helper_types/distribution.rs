use std::fmt::Display;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, Ord, PartialOrd)]
pub enum Distribution
{
  #[serde(rename = "static")]
  Static,
  #[serde(rename = "shared")]
  Shared,
  #[serde(rename = "sources")]
  Sources,
  #[serde(rename = "executable")]
  Executable,
  #[serde(rename = "???")]
  Unknown
}

impl Default for Distribution
{
  fn default() -> Self
  {
    Self::Unknown
  }
}

impl From<&str> for Distribution
{
  fn from(value: &str) -> Self
  {
    match value
    {
      "static" | "static-lib" => Self::Static,
      "shared" | "dynamic" | "dynamic-lib" => Self::Shared,
      "sources" | "src" | "source" => Self::Sources,
      "executable" | "bin" | "binaries" => Self::Executable,
      _ => Self::Unknown
    }
  }
}

impl From<String> for Distribution
{
  fn from(value: String) -> Self
  {
    Self::from(value.as_str())
  }
}

impl Display for Distribution
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
  {
    write!(f, "{}", match self
    {
      Self::Static => "static",
      Self::Shared => "shared",
      Self::Sources => "sources",
      Self::Executable => "executable",
      _ => "unknown"
    })
  }
}