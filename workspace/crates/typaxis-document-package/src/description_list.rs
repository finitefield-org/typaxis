//! Version-bound authored term/definition carrier. It never synthesizes a
//! bullet, number, paragraph owner or label in the retained document.
use super::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireDescriptionTerm {
    pub node_id: u32,
    pub span: WireStagingSourceSpan,
    pub classes: Vec<String>,
    pub children: Vec<WireStagingM4Inline>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, bound = "K: WireSemanticKind")]
pub struct WireSemanticDescriptionItem<K> {
    pub node_id: u32,
    pub span: WireStagingSourceSpan,
    pub term: WireDescriptionTerm,
    pub blocks: Vec<WireSemanticBlock<K>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

pub(super) fn deserialize_items<'de, D: Deserializer<'de>, K: WireSemanticKind>(
    deserializer: D,
) -> Result<Vec<WireSemanticDescriptionItem<K>>, D::Error> {
    if !K::DESCRIPTION_LISTS {
        return Err(de::Error::custom("description_list requires contract 1.5"));
    }
    Vec::deserialize(deserializer)
}

pub(super) fn serialize_items<S: Serializer, K: WireSemanticKind>(
    items: &[WireSemanticDescriptionItem<K>],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    if !K::DESCRIPTION_LISTS {
        return Err(serde::ser::Error::custom(
            "description_list requires contract 1.5",
        ));
    }
    items.serialize(serializer)
}

pub(super) fn validate_shape<K: WireSemanticKind>(
    items: &[WireSemanticDescriptionItem<K>],
) -> Result<(), StagingSemanticDecodeError> {
    if !K::DESCRIPTION_LISTS || items.is_empty() {
        return Err(StagingSemanticDecodeError::Shape(
            "description_list requires contract 1.5 and nonempty items",
        ));
    }
    if items
        .iter()
        .any(|item| item.term.children.is_empty() || item.blocks.is_empty())
    {
        return Err(StagingSemanticDecodeError::Shape(
            "description item requires nonempty term children and definition blocks",
        ));
    }
    Ok(())
}

/// The old carrier checks unchanged inline/resource/style fields. Its temporary
/// view includes the term so those checks cannot be bypassed. The returned
/// successor wire, source tree and canonical bytes never use this projection.
#[cfg(any(test, feature = "book-v2-staging"))]
pub(super) fn flatten_for_carrier_validation(
    object: &mut Map<String, Value>,
) -> Result<(), StagingSemanticDecodeError> {
    if object.contains_key("ordered") || object.contains_key("start") {
        return Err(StagingSemanticDecodeError::Shape(
            "description_list forbids ordered and start",
        ));
    }
    let items = object
        .get_mut("items")
        .and_then(Value::as_array_mut)
        .ok_or(StagingSemanticDecodeError::Shape(
            "description items are required",
        ))?;
    for item in items {
        let item = item
            .as_object_mut()
            .ok_or(StagingSemanticDecodeError::Shape(
                "description item must be an object",
            ))?;
        let mut term = item
            .remove("term")
            .ok_or(StagingSemanticDecodeError::Shape(
                "description term is required",
            ))?;
        let term_object = term
            .as_object_mut()
            .ok_or(StagingSemanticDecodeError::Shape(
                "description term must be an object",
            ))?;
        if term_object.contains_key("kind") {
            return Err(StagingSemanticDecodeError::Shape(
                "description term forbids kind",
            ));
        }
        term_object.insert("kind".into(), Value::String("paragraph".into()));
        let blocks = item.get_mut("blocks").and_then(Value::as_array_mut).ok_or(
            StagingSemanticDecodeError::Shape("description blocks are required"),
        )?;
        blocks
            .try_reserve_exact(1)
            .map_err(|_| StagingSemanticDecodeError::Limit)?;
        blocks.insert(0, term);
    }
    object.insert("kind".into(), Value::String("list".into()));
    object.insert("ordered".into(), Value::Bool(false));
    object.insert("start".into(), Value::Null);
    Ok(())
}
