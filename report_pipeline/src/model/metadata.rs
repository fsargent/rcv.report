use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Represents a body that oversees elections for one or more constituancies.
pub struct Jurisdiction {
    /// Name of the district.
    pub name: String,
    /// Path to the district, e.g. <country>/<state>/<city>.
    pub path: String,
    /// Kind of electoral commission, e.g. municipal, state.
    pub kind: String,
    /// A mapping from id of elected positions to their display names.
    pub offices: BTreeMap<String, Office>,
    /// A list of elections under this commission.
    pub elections: BTreeMap<String, ElectionMetadata>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Represents an elected office in this constituancy.
pub struct Office {
    /// Name of the office.
    pub name: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElectionMetadata {
    /// Name of election.
    pub name: String,

    /// Date of election:
    pub date: String,

    pub data_format: String,

    pub tabulation_options: Option<TabulationOptions>,

    pub normalization: String,

    pub contests: Vec<Contest>,

    pub files: BTreeMap<String, String>,

    pub website: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TabulationOptions {
    pub eager: Option<bool>,
}

impl Default for TabulationOptions {
    fn default() -> Self {
        TabulationOptions { eager: Some(true) }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contest {
    pub office: String,
    pub loader_params: Option<BTreeMap<String, String>>,
}
