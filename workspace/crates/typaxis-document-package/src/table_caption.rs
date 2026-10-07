//! Version-bound table captions. The retained tree always owns the original
//! caption blocks before its header/body rows; compatibility views are temporary.
use super::*;

pub(super) fn deserialize<'de, D: Deserializer<'de>, K: WireSemanticKind>(
    deserializer: D,
) -> Result<Option<Vec<WireSemanticBlock<K>>>, D::Error> {
    if !K::TABLE_CAPTIONS {
        return Err(de::Error::custom("table caption requires contract 1.5"));
    }
    // Deserialize a Vec, not Option: explicit null is not omission.
    let blocks = Vec::deserialize(deserializer)?;
    if blocks.is_empty() {
        return Err(de::Error::custom("table caption must not be empty"));
    }
    Ok(Some(blocks))
}

pub(super) fn serialize<S: Serializer, K: WireSemanticKind>(
    caption: &Option<Vec<WireSemanticBlock<K>>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    validate::<K>(caption).map_err(serde::ser::Error::custom)?;
    caption.serialize(serializer)
}

pub(super) fn validate<K: WireSemanticKind>(
    caption: &Option<Vec<WireSemanticBlock<K>>>,
) -> Result<(), StagingSemanticDecodeError> {
    if let Some(blocks) = caption {
        if !K::TABLE_CAPTIONS || blocks.is_empty() {
            return Err(StagingSemanticDecodeError::Shape(
                "table caption requires contract 1.5 and nonempty blocks",
            ));
        }
    }
    Ok(())
}
