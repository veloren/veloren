pub mod faction;
pub mod nature;
pub mod npc;
pub mod site;

pub use self::{
    faction::{Faction, FactionId, Factions},
    nature::Nature,
    npc::{Npc, NpcId, Npcs},
    site::{Site, SiteId, Sites},
};

use common::resources::TimeOfDay;
use enum_map::{enum_map, EnumArray, EnumMap};
use serde::{
    de::{self, Error as _},
    ser, Deserialize, Serialize,
};
use std::{
    cmp::PartialEq,
    fmt,
    io::{Read, Write},
    marker::PhantomData,
};

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum Actor {
    Npc(NpcId),
    Character(common::character::CharacterId),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Data {
    pub nature: Nature,
    pub npcs: Npcs,
    pub sites: Sites,
    pub factions: Factions,

    pub time_of_day: TimeOfDay,
}

pub type ReadError = rmp_serde::decode::Error;
pub type WriteError = rmp_serde::encode::Error;

impl Data {
    pub fn from_reader<R: Read>(reader: R) -> Result<Self, ReadError> {
        rmp_serde::decode::from_read(reader)
    }

    pub fn write_to<W: Write>(&self, mut writer: W) -> Result<(), WriteError> {
        rmp_serde::encode::write_named(&mut writer, self)
    }
}

fn rugged_ser_enum_map<
    K: EnumArray<V> + Serialize,
    V: From<i16> + PartialEq + Serialize,
    S: ser::Serializer,
    const DEFAULT: i16,
>(
    map: &EnumMap<K, V>,
    ser: S,
) -> Result<S::Ok, S::Error> {
    ser.collect_map(
        map.iter()
            .filter(|(k, v)| v != &&V::from(DEFAULT))
            .map(|(k, v)| (k, v)),
    )
}

fn rugged_de_enum_map<
    'a,
    K: EnumArray<V> + EnumArray<Option<V>> + Deserialize<'a>,
    V: From<i16> + Deserialize<'a>,
    D: de::Deserializer<'a>,
    const DEFAULT: i16,
>(
    mut de: D,
) -> Result<EnumMap<K, V>, D::Error> {
    struct Visitor<K, V, const DEFAULT: i16>(PhantomData<(K, V)>);

    impl<'de, K, V, const DEFAULT: i16> de::Visitor<'de> for Visitor<K, V, DEFAULT>
    where
        K: EnumArray<V> + EnumArray<Option<V>> + Deserialize<'de>,
        V: From<i16> + Deserialize<'de>,
    {
        type Value = EnumMap<K, V>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(formatter, "a map")
        }

        fn visit_map<M: de::MapAccess<'de>>(self, mut access: M) -> Result<Self::Value, M::Error> {
            let mut entries = EnumMap::default();
            while let Some((key, value)) = access.next_entry()? {
                entries[key] = Some(value);
            }
            Ok(enum_map! { key => entries[key].take().unwrap_or_else(|| V::from(DEFAULT)) })
        }
    }

    de.deserialize_map(Visitor::<_, _, DEFAULT>(PhantomData))
}
