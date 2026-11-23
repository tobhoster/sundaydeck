use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::IdParseError;

macro_rules! define_id {
    ($name:ident, $kind:literal) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        #[repr(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Creates a new time-ordered UUIDv7 identifier.
            pub fn new_v7() -> Self {
                Self(Uuid::now_v7())
            }

            /// Wraps an existing UUID explicitly so distinct ID types cannot be swapped.
            pub const fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Creates a deterministic UUIDv5 identifier inside `namespace`.
            pub fn from_v5(namespace: &Uuid, name: &[u8]) -> Self {
                Self(Uuid::new_v5(namespace, name))
            }

            /// Parses a canonical UUID string into this strongly-typed identifier.
            pub fn parse_str(raw: &str) -> Result<Self, IdParseError> {
                Uuid::parse_str(raw)
                    .map(Self)
                    .map_err(|source| IdParseError {
                        kind: $kind,
                        raw: raw.to_owned(),
                        source,
                    })
            }

            /// Returns the underlying UUID value.
            pub const fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }
    };
}

define_id!(SongId, "song");
define_id!(SectionId, "song_section");
define_id!(ArrangementId, "song_arrangement");
define_id!(ThemeId, "theme");
define_id!(MediaAssetId, "media_asset");
define_id!(ServiceId, "service");
define_id!(ServiceItemId, "service_item");
define_id!(SlideId, "slide");
define_id!(AlertId, "alert");
define_id!(TimerId, "timer");
define_id!(BibleTranslationId, "bible_translation");

/// Fixed UUIDv5 namespace for deterministic built-in seeds and reflowed slide IDs.
pub const SUNDAYDECK_NAMESPACE: Uuid = Uuid::from_bytes([
    0x73, 0x75, 0x6e, 0x64, 0x61, 0x79, 0x64, 0x65, 0x63, 0x6b, 0x2d, 0x76, 0x31, 0x2d, 0x6e, 0x73,
]);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_roundtrip_and_invalid_error() {
        let id = SongId::new_v7();
        let text = id.to_string();
        let parsed = SongId::parse_str(&text).expect("valid UUID string must parse");
        assert_eq!(parsed, id);

        let err = ThemeId::parse_str("not-a-uuid").expect_err("invalid UUID must fail");
        assert_eq!(err.kind, "theme");
        assert_eq!(err.raw, "not-a-uuid");
    }

    #[test]
    fn deterministic_v5_is_stable() {
        let a = SlideId::from_v5(&SUNDAYDECK_NAMESPACE, b"song:1:slide:0");
        let b = SlideId::from_v5(&SUNDAYDECK_NAMESPACE, b"song:1:slide:0");
        let c = SlideId::from_v5(&SUNDAYDECK_NAMESPACE, b"song:1:slide:1");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
