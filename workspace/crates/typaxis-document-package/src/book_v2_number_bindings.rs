use super::*;

/// A number reference borrows the explicitly selected part of displayed source
/// text. The syntax owner validates both nodes, their ancestry and UTF-8 range.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireBookV2NumberBinding {
    pub anchor_id: String,
    pub owner_node_id: u32,
    pub label_node_id: u32,
    pub text_span: WireStagingTextSpan,
}

pub(super) fn deserialize_bindings<'de, K: WireSemanticKind, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Vec<WireBookV2NumberBinding>>, D::Error> {
    if !K::NUMBER_BINDINGS {
        return Err(serde::de::Error::custom(
            "number bindings require contract 1.5",
        ));
    }
    // A present null or empty collection cannot masquerade as an absent field.
    let values = Vec::<WireBookV2NumberBinding>::deserialize(deserializer)?;
    if values.is_empty() {
        return Err(serde::de::Error::custom("number bindings must be nonempty"));
    }
    Ok(Some(values))
}

pub(super) fn serialize_bindings<K: WireSemanticKind, S: serde::Serializer>(
    values: &Option<Vec<WireBookV2NumberBinding>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    if !K::NUMBER_BINDINGS || values.as_ref().is_none_or(Vec::is_empty) {
        return Err(serde::ser::Error::custom(
            "number bindings require nonempty contract 1.5 data",
        ));
    }
    values.serialize(serializer)
}
