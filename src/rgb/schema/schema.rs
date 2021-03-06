// LNP/BP Rust Library
// Written in 2020 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License
// along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use bitcoin::hashes::{sha256t, Hash};
use std::{collections::BTreeMap, io};

use super::{
    script, AssignmentsType, DataFormat, GenesisSchema, SimplicityScript, StateFormat,
    TransitionSchema,
};
use crate::client_side_validation::{commit_strategy, CommitEncodeWithStrategy, ConsensusCommit};

pub type FieldType = usize; // Here we can use usize since encoding/decoding makes sure that it's u16
pub type TransitionType = usize; // Here we can use usize since encoding/decoding makes sure that it's u16

static MIDSTATE_SHEMA_ID: [u8; 32] = [
    25, 205, 224, 91, 171, 217, 131, 31, 140, 104, 5, 155, 127, 82, 14, 81, 58, 245, 79, 165, 114,
    243, 110, 60, 133, 174, 103, 187, 103, 230, 9, 106,
];

tagged_hash!(
    SchemaId,
    SchemaIdTag,
    MIDSTATE_SHEMA_ID,
    doc = "Commitment-based schema identifier used for committing to the schema type"
);

#[derive(Clone, Debug, Display)]
#[display_from(Debug)]
pub struct Schema {
    pub field_types: BTreeMap<FieldType, DataFormat>,
    pub assignment_types: BTreeMap<AssignmentsType, StateFormat>,
    pub genesis: GenesisSchema,
    pub transitions: BTreeMap<TransitionType, TransitionSchema>,
    pub script_library: SimplicityScript,
    pub script_extensions: script::Extensions,
}

impl Schema {
    #[inline]
    pub fn schema_id(&self) -> SchemaId {
        self.clone().consensus_commit()
    }
}

impl ConsensusCommit for Schema {
    type Commitment = SchemaId;
}
impl CommitEncodeWithStrategy for Schema {
    type Strategy = commit_strategy::UsingStrict;
}

mod strict_encoding {
    use super::*;
    use crate::strict_encoding::{Error, StrictDecode, StrictEncode};

    impl StrictEncode for SchemaId {
        type Error = Error;

        #[inline]
        fn strict_encode<E: io::Write>(&self, e: E) -> Result<usize, Self::Error> {
            self.into_inner().to_vec().strict_encode(e)
        }
    }

    impl StrictDecode for SchemaId {
        type Error = Error;

        #[inline]
        fn strict_decode<D: io::Read>(d: D) -> Result<Self, Self::Error> {
            Ok(
                Self::from_slice(&Vec::<u8>::strict_decode(d)?).map_err(|_| {
                    Error::DataIntegrityError("Wrong RIPEMD-160 hash data size".to_string())
                })?,
            )
        }
    }

    impl StrictEncode for Schema {
        type Error = Error;

        fn strict_encode<E: io::Write>(&self, mut e: E) -> Result<usize, Self::Error> {
            Ok(strict_encode_list!(e;
                self.field_types,
                self.assignment_types,
                self.genesis,
                self.transitions,
                self.script_library,
                self.script_extensions
            ))
        }
    }

    impl StrictDecode for Schema {
        type Error = Error;

        fn strict_decode<D: io::Read>(mut d: D) -> Result<Self, Self::Error> {
            Ok(Self {
                field_types: BTreeMap::strict_decode(&mut d)?,
                assignment_types: BTreeMap::strict_decode(&mut d)?,
                genesis: GenesisSchema::strict_decode(&mut d)?,
                transitions: BTreeMap::strict_decode(&mut d)?,
                script_library: Vec::strict_decode(&mut d)?,
                script_extensions: script::Extensions::strict_decode(&mut d)?,
            })
        }
    }
}
