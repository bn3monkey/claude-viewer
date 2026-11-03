use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FsKind {
    Dir,
    File,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FsNode {
    pub path: String,
    pub name: String,
    pub kind: FsKind,
    /// 디렉터리면 Some, 파일이면 None.
    #[ts(optional = nullable)]
    pub children: Option<Vec<FsNode>>,
}
