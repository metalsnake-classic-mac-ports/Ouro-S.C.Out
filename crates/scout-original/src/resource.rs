//! Classic Mac resource fork parser.

use crate::bytes::{i16_at, mac_roman, slice, u8_at, u16_at, u32_at};
use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct Resource {
    pub kind: [u8; 4],
    pub id: i16,
    pub name: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct ResourceFork {
    resources: Vec<Resource>,
}

impl ResourceFork {
    pub fn parse(fork: &[u8]) -> Result<Self> {
        let data_offset = u32_at(fork, 0)? as usize;
        let map_offset = u32_at(fork, 4)? as usize;
        let map_len = u32_at(fork, 12)? as usize;
        let map = slice(fork, map_offset, map_len)?;
        let type_list = u16_at(map, 24)? as usize;
        let name_list = u16_at(map, 26)? as usize;
        let type_count = u16_at(map, type_list)?.wrapping_add(1) as usize;

        let mut resources = Vec::new();
        for t in 0..type_count {
            let entry = type_list + 2 + t * 8;
            let kind: [u8; 4] = slice(map, entry, 4)?.try_into().unwrap();
            let count = u16_at(map, entry + 4)? as usize + 1;
            let refs = type_list + u16_at(map, entry + 6)? as usize;
            for r in 0..count {
                let reference = refs + r * 12;
                let id = i16_at(map, reference)?;
                let name_offset = u16_at(map, reference + 2)?;
                let offset = data_offset + (u32_at(map, reference + 4)? & 0x00FF_FFFF) as usize;
                let len = u32_at(fork, offset)? as usize;
                let name = if name_offset == 0xFFFF {
                    String::new()
                } else {
                    let at = name_list + name_offset as usize;
                    mac_roman(slice(map, at + 1, u8_at(map, at)? as usize)?)
                };
                resources.push(Resource {
                    kind,
                    id,
                    name,
                    data: slice(fork, offset + 4, len)?.to_vec(),
                });
            }
        }
        Ok(Self { resources })
    }

    pub fn get(&self, kind: &[u8; 4], id: i16) -> Result<&Resource> {
        self.resources
            .iter()
            .find(|r| &r.kind == kind && r.id == id)
            .ok_or_else(|| Error::MissingResource {
                kind: String::from_utf8_lossy(kind).into_owned(),
                id,
            })
    }

    pub fn of_kind<'s>(&'s self, kind: &'s [u8; 4]) -> impl Iterator<Item = &'s Resource> + 's {
        self.resources.iter().filter(move |r| &r.kind == kind)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Resource> {
        self.resources.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a resource fork with one 'TEST' resource (id 128, name "hi").
    fn sample_fork() -> Vec<u8> {
        let payload = b"abc";
        let mut data = Vec::new();
        data.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        data.extend_from_slice(payload);

        let mut map = vec![0u8; 28];
        let type_list = map.len() as u16;
        map.extend_from_slice(&0u16.to_be_bytes()); // one type
        map.extend_from_slice(b"TEST");
        map.extend_from_slice(&0u16.to_be_bytes()); // one resource
        map.extend_from_slice(&10u16.to_be_bytes()); // refs at type_list + 10
        map.extend_from_slice(&128i16.to_be_bytes());
        map.extend_from_slice(&0u16.to_be_bytes()); // name offset
        map.extend_from_slice(&0u32.to_be_bytes()); // data offset 0
        map.extend_from_slice(&0u32.to_be_bytes());
        let name_list = map.len() as u16;
        map.extend_from_slice(b"\x02hi");
        map[24..26].copy_from_slice(&type_list.to_be_bytes());
        map[26..28].copy_from_slice(&name_list.to_be_bytes());

        let mut fork = vec![0u8; 16];
        fork[0..4].copy_from_slice(&16u32.to_be_bytes());
        fork[4..8].copy_from_slice(&((16 + data.len()) as u32).to_be_bytes());
        fork[8..12].copy_from_slice(&(data.len() as u32).to_be_bytes());
        fork[12..16].copy_from_slice(&(map.len() as u32).to_be_bytes());
        fork.extend_from_slice(&data);
        fork.extend_from_slice(&map);
        fork
    }

    #[test]
    fn parses_a_resource() {
        let fork = ResourceFork::parse(&sample_fork()).unwrap();
        let r = fork.get(b"TEST", 128).unwrap();
        assert_eq!(r.data, b"abc");
        assert_eq!(r.name, "hi");
        assert!(fork.get(b"TEST", 129).is_err());
    }

    #[test]
    fn truncated_fork_is_an_error() {
        let fork = sample_fork();
        assert!(ResourceFork::parse(&fork[..fork.len() - 4]).is_err());
    }
}
