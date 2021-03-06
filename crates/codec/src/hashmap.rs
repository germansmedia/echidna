// Echidna - Codec

use {
    crate::*,
    std::{
        collections::HashMap,
        hash::Hash,
    },
};

impl<K: Codec + Eq + Hash,V: Codec> Codec for HashMap<K,V> {
    fn decode(buffer: &[u8]) -> Option<(usize,Self)> {
        if let Some((_,len)) = u32::decode(buffer) {
            let mut r = HashMap::<K,V>::new();
            let mut ofs = 4usize;
            for _ in 0..len {
                let key = if let Some((l,key)) = K::decode(&buffer[ofs..]) {
                    ofs += l;
                    key
                }
                else {
                    return None;
                };
                let value = if let Some((l,value)) = V::decode(&buffer[ofs..]) {
                    ofs += l;
                    value
                }
                else {
                    return None;
                };
                r.insert(key,value);
            }
            Some((ofs,r))
        }
        else {
            None
        }
    }

    fn encode(&self,buffer: &mut Vec<u8>) -> usize {
        (self.len() as u32).encode(buffer);
        let mut len = 4;
        for (key,value) in self {
            len += key.encode(buffer);
            len += value.encode(buffer);
        }
        len
    }

    fn size(&self) -> usize {
        let mut len = 4;
        for (key,value) in self {
            len += key.size();
            len += value.size();
        }
        len
    }
}

#[cfg(test)]
mod tests {

    use crate::Codec;
    use std::collections::HashMap;

    #[test]
    fn test_hashmap() {
        let mut source: HashMap<u16,i32> = HashMap::new();
        source.insert(0,-3);
        source.insert(1,-2);
        source.insert(2,-1);
        let mut buffer = Vec::<u8>::new();
        source.encode(&mut buffer);
        if let Some((_,target)) = HashMap::<u16,i32>::decode(&buffer) {
            assert_eq!(source,target)
        }
        else {
            assert!(false)
        }
    }
}
