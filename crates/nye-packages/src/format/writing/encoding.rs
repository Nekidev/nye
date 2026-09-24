//! Encoding of package file components.

use crate::format::{NyeFileEntry, NyeFileEntryKind, NyeFileSignature, Segment, Segments};

pub trait Encodeable {
    fn encode(&self) -> Vec<u8>;
}

impl Encodeable for NyeFileSignature {
    fn encode(&self) -> Vec<u8> {
        Vec::from([110, 121, 101, self.version])
    }
}

impl Encodeable for NyeFileEntry {
    fn encode(&self) -> Vec<u8> {
        let mut bytes = self.size.to_be_bytes().to_vec();

        bytes.extend(self.kind.encode());
        bytes.extend(self.name.encode());

        bytes
    }
}

impl Encodeable for NyeFileEntryKind {
    fn encode(&self) -> Vec<u8> {
        let byte = match self {
            Self::Bin => 0,
            Self::Lib => 1,
            Self::Etc => 2,
            Self::Var => 3,
        };

        vec![byte]
    }
}

impl Encodeable for Segments {
    fn encode(&self) -> Vec<u8> {
        let size = (self.segments().len() - 1) as u8;
        let mut bytes = vec![size];

        for segment in self.segments() {
            bytes.extend(segment.encode());
        }

        bytes
    }
}

impl Encodeable for Segment {
    fn encode(&self) -> Vec<u8> {
        let size = (self.0.len() - 1) as u8;
        let mut bytes = Vec::with_capacity(1 + self.0.len());

        bytes.push(size);
        bytes.extend(self.0.clone());

        bytes
    }
}
