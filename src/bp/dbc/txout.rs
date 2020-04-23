// LNP/BP Rust Library
// Written in 2019 by
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

use super::scriptpubkey::{Error, ScriptPubkeyCommitment, ScriptPubkeyContainer};
use crate::primitives::commit_verify::CommitEmbedVerify;

#[derive(Clone, PartialEq, Eq, Debug, Display)]
#[display_from(Debug)]
pub struct TxoutContainer {
    pub value: u64,
    pub script_container: ScriptPubkeyContainer,
}

#[derive(Clone, PartialEq, Eq, Debug, Display)]
#[display_from(Debug)]
pub struct TxoutCommitment {
    pub value: u64,
    pub script_commitment: ScriptPubkeyCommitment,
}

impl<MSG> CommitEmbedVerify<MSG> for TxoutCommitment
where
    MSG: AsRef<[u8]>,
{
    type Container = TxoutContainer;
    type Error = Error;

    #[inline]
    fn container(&self) -> Self::Container {
        TxoutContainer {
            value: self.value,
            script_container: CommitEmbedVerify::<MSG>::container(&self.script_commitment),
        }
    }

    fn commit_embed(container: Self::Container, msg: &MSG) -> Result<Self, Self::Error> {
        Ok(Self {
            value: container.value,
            script_commitment: ScriptPubkeyCommitment::commit_embed(
                container.script_container,
                msg,
            )?,
        })
    }
}
