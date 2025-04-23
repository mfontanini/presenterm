use serde::{Deserializer, Serializer};
use std::{
    fmt::{self, Display},
    marker::PhantomData,
    str::FromStr,
};

macro_rules! impl_deserialize_from_str {
    ($ty:ty) => {
        impl<'de> serde::de::Deserialize<'de> for $ty {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::de::Deserializer<'de>,
            {
                $crate::utils::deserialize_from_str(deserializer)
            }
        }
    };
}

macro_rules! impl_serialize_from_display {
    ($ty:ty) => {
        impl serde::Serialize for $ty {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                $crate::utils::serialize_display(self, serializer)
            }
        }
    };
}

pub(crate) use impl_deserialize_from_str;
pub(crate) use impl_serialize_from_display;

// Same behavior as serde_with::DeserializeFromStr
pub(crate) fn deserialize_from_str<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: Display,
{
    struct Visitor<S>(PhantomData<S>);

    impl<S> serde::de::Visitor<'_> for Visitor<S>
    where
        S: FromStr,
        <S as FromStr>::Err: Display,
    {
        type Value = S;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(formatter, "a string")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            value.parse::<S>().map_err(serde::de::Error::custom)
        }
    }

    deserializer.deserialize_str(Visitor(PhantomData))
}

// Same behavior as serde_with::SerializeDisplay
pub(crate) fn serialize_display<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Display,
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}
