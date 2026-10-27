use serde::Serialize;

/// Sort the results by their field value. The default sort is ASC, DESC if the field is
/// preceded by a `-`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum SourceOrdering {
    #[serde(rename = "last_scan_date")]
    LastScanDate,
    #[serde(rename = "-last_scan_date")]
    LastScanDateDesc,
}
