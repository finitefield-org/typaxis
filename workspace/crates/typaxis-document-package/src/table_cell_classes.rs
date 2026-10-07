//! Optional, version-bound style classes on original table-cell owners.
use super::*;

pub(super) fn deserialize<'de, K: WireSemanticKind, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Vec<String>>, D::Error> {
    if !K::TABLE_CELL_STYLES {
        return Err(de::Error::custom("table cell classes require contract 1.5"));
    }
    // Vec rejects explicit null; only an absent member means omission.
    Vec::<String>::deserialize(deserializer).map(Some)
}

pub(super) fn serialize<K: WireSemanticKind, S: Serializer>(
    classes: &Option<Vec<String>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    validate::<K>(classes).map_err(serde::ser::Error::custom)?;
    classes.serialize(serializer)
}

pub(super) fn validate<K: WireSemanticKind>(
    classes: &Option<Vec<String>>,
) -> Result<(), StagingSemanticDecodeError> {
    if classes.is_some() && !K::TABLE_CELL_STYLES {
        return Err(StagingSemanticDecodeError::Shape(
            "table cell classes require contract 1.5",
        ));
    }
    Ok(())
}
