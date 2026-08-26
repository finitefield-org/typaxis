use core::fmt;

macro_rules! sha256_type {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; 32]);

        impl $name {
            pub(crate) const fn new(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            pub const fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }

            pub const fn into_bytes(self) -> [u8; 32] {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                for byte in self.0 {
                    write!(formatter, "{byte:02x}")?;
                }
                Ok(())
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_tuple(stringify!($name))
                    .field(&self.to_string())
                    .finish()
            }
        }
    };
}

sha256_type!(
    RawDocumentPackageSha256,
    "SHA-256 of the exact admitted JSON bytes, including whitespace and member order."
);
sha256_type!(
    CanonicalDocumentPackageJcsSha256,
    "SHA-256 of the bounded canonical JCS encoding of a typed DocumentPackage."
);

pub type RawDocumentPackageHash = RawDocumentPackageSha256;
pub type CanonicalDocumentPackageHash = CanonicalDocumentPackageJcsSha256;
