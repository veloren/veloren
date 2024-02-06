pub mod faction;
pub mod nature;
pub mod npc;
pub mod report;
pub mod sentiment;
pub mod site;

pub use self::{
    faction::{Faction, FactionId, Factions},
    nature::Nature,
    npc::{Npc, NpcId, Npcs},
    report::{Report, ReportId, ReportKind, Reports},
    sentiment::{Sentiment, Sentiments},
    site::{Site, SiteId, Sites},
};

use common::resources::TimeOfDay;
use enum_map::{enum_map, EnumArray, EnumMap};
use serde::{de, ser, Deserialize, Serialize};
use std::{
    cmp::PartialEq,
    fmt,
    io::{Read, Write},
    marker::PhantomData,
};

/// The current version of rtsim data.
///
/// Note that this number does *not* need incrementing on every change: most
/// field removals/additions are fine. This number should only be incremented
/// when we wish to perform a *hard purge* of rtsim data.
pub const CURRENT_VERSION: u32 = 5;

#[derive(Clone, Serialize, Deserialize)]
pub struct Data {
    // Absence of field just implied version = 0
    #[serde(default)]
    pub version: u32,

    pub nature: Nature,
    #[serde(default)]
    pub npcs: Npcs,
    #[serde(default)]
    pub sites: Sites,
    #[serde(default)]
    pub factions: Factions,
    #[serde(default)]
    pub reports: Reports,

    #[serde(default)]
    pub tick: u64,
    #[serde(default)]
    pub time_of_day: TimeOfDay,

    // If true, rtsim data will be ignored (and, hence, overwritten on next save) on load.
    #[serde(default)]
    pub should_purge: bool,
}

pub enum ReadError {
    Load(rmp_serde::decode::Error),
    // Preserve old data
    VersionMismatch(Box<Data>),
}

impl fmt::Debug for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Load(err) => err.fmt(f),
            Self::VersionMismatch(_) => write!(f, "VersionMismatch"),
        }
    }
}

pub type WriteError = rmp_serde::encode::Error;

impl Data {
    pub fn spawn_npc(&mut self, npc: Npc) -> NpcId {
        let home = npc.home;
        let id = self.npcs.create_npc(npc);
        if let Some(home) = home.and_then(|home| self.sites.get_mut(home)) {
            home.population.insert(id);
        }
        id
    }

    pub fn from_reader<R: Read>(reader: R) -> Result<Box<Self>, ReadError> {
        rmp_serde::decode::from_read(reader)
            .map_err(ReadError::Load)
            .and_then(|data: Data| {
                if data.version == CURRENT_VERSION {
                    Ok(Box::new(data))
                } else {
                    Err(ReadError::VersionMismatch(Box::new(data)))
                }
            })
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
    ser.collect_map(map.iter().filter(|(_, v)| v != &&V::from(DEFAULT)))
}

fn rugged_de_enum_map<
    'a,
    K: EnumArray<V> + EnumArray<Option<V>> + Deserialize<'a>,
    V: From<i16> + Deserialize<'a>,
    D: de::Deserializer<'a>,
    const DEFAULT: i16,
>(
    de: D,
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
