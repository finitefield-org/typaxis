//! Resource preflight facade. The source owner issues the sealed policy view so
//! downstream layout/shaping can consume it without depending on this registry.
pub use typaxis_syntax::book_v2::{
    prepare_book_v2_resource_policy, BookV2ResourcePolicy, BookV2ResourcePolicyError,
    BOOK_V2_RESOURCE_POLICY_ALGORITHM, BOOK_V2_RESOURCE_SET,
};
