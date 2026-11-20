//! Serde for arrays longer than serde's own 32 (the level's boxes): as a
//! sequence.

use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub fn serialize<S: Serializer, T: Serialize, const N: usize>(a: &[T; N], s: S) -> Result<S::Ok, S::Error> {
    s.collect_seq(a.iter())
}

pub fn deserialize<'de, D: Deserializer<'de>, T: Deserialize<'de>, const N: usize>(d: D) -> Result<[T; N], D::Error> {
    let v = Vec::<T>::deserialize(d)?;
    let len = v.len();
    v.try_into().map_err(|_| D::Error::invalid_length(len, &"the array's length"))
}
