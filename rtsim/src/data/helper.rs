use serde::{
    de::{DeserializeOwned, Error},
    Deserialize, Deserializer, Serialize, Serializer,
};

#[derive(Copy, Clone, Default, PartialEq, Eq, Hash)]
pub struct V<T>(pub T);

impl<T: Serialize> Serialize for V<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, T: Version> Deserialize<'de> for V<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::try_from_value_compat(ron::Value::deserialize(deserializer)?)
            .map(Self)
            .map_err(|e| D::Error::custom(e))
    }
}

impl<U, T: Latest<U>> Latest<U> for V<T> {
    fn to_unversioned(self) -> U { self.0.to_unversioned() }

    fn from_unversioned(x: &U) -> Self { Self(T::from_unversioned(x)) }
}

pub trait Latest<T> {
    fn to_unversioned(self) -> T;
    fn from_unversioned(x: &T) -> Self;
}

pub trait Version: Sized + DeserializeOwned {
    type Prev: Version;

    fn migrate(prev: Self::Prev) -> Self;

    fn try_from_value_compat(value: ron::Value) -> Result<Self, ron::Error> {
        value.clone().into_rust().or_else(|e| {
            Ok(Self::migrate(
                <Self as Version>::Prev::try_from_value_compat(value).map_err(|_| e)?,
            ))
        })
    }
}

#[derive(Deserialize)]
pub enum Bottom {}

impl Version for Bottom {
    type Prev = Self;

    fn migrate(prev: Self::Prev) -> Self { prev }
}
