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

//! LNPBP-2
//! =======
//!
//! Module implementing LNPBP-2 standard:
//! Deterministic embedding of LNPBP1-type commitments into `scriptPubkey` of a
//! transaction output
//! [LNPBP-2](https://github.com/LNP-BP/lnpbps/blob/master/lnpbp-0002.md)
//!
//! The standard defines an algorithm for deterministic embedding and
//! verification of cryptographic commitments based on elliptic-curve public and
//! private key modifications (tweaks) inside all the existing types of Bitcoin
//! transaction output and arbitrary complex Bitcoin scripts.

use bitcoin::hash_types::PubkeyHash;
use bitcoin::secp256k1;
use core::cell::RefCell;

use super::pubkey::PubkeyCommitment;
use super::Error;
use crate::bp::scripts::LockScript;
use crate::primitives::commit_verify::CommitEmbedVerify;

#[derive(Clone, PartialEq, Eq, Hash, Debug, Display)]
#[display_from(Debug)]
pub struct LockscriptContainer {
    pub script: LockScript,
    pub pubkey: secp256k1::PublicKey,
}

wrapper!(
    LockscriptCommitment,
    LockScript,
    doc = "LockScript contanining public keys which sum is commit to some message according to LNPBP-2",
    derive = [PartialEq, Eq, Hash]
);

impl<MSG> CommitEmbedVerify<MSG> for LockscriptCommitment
where
    MSG: AsRef<[u8]>,
{
    type Container = LockscriptContainer;
    type Error = Error;

    /// Function implements commitment procedure according to LNPBP-2.
    ///
    /// LNPBP-2 Specification extract:
    /// ------------------------------
    ///
    /// 1. The provided script MUST be parsed with Miniscript parser; if the parser fails the procedure MUST fail.
    /// 2. Iterate over all branches of the abstract syntax tree generated by the Miniscript parser, running the following
    ///    algorithm for each node:
    ///    - if a public key hash is met (`pk_h` Miniscript command) and it can't be resolved against known public keys or other public keys extracted from the script, fail the procedure;
    ///    - if a public key is found (`pk`) add it to the list of the collected public keys;
    ///    - for all other types of Miniscript commands iterate over their branches.
    /// 3. Select unique public keys (i.e. if some public key is repeated in different parts of the script/in different
    ///    script branches, pick a single instance of it). Compressed and uncompressed versions of the same public key must be treaded as the same public key under this procedure.
    /// 4. If no public keys were found fail the procedure; return the collected keys otherwise.
    ///
    /// **NB: SUBJECT TO CHANGE UPON RELEASE**
    /// By "miniscript" we mean usage of `rust-miniscript` library at commit `a5ba1219feb8b5a289c8f12176d632635eb8a959`
    /// which may be found on <https://github.com/LNP-BP/rust-miniscript/commit/a5ba1219feb8b5a289c8f12176d632635eb8a959>
    // #[consensus_critical]
    // #[standard_critical("LNPBP-1")]
    fn commit_embed(container: Self::Container, msg: &MSG) -> Result<Self, Self::Error> {
        let original_hash = bitcoin::PublicKey {
            compressed: true,
            key: container.pubkey,
        }
        .pubkey_hash();
        let tweaked_pk = PubkeyCommitment::commit_embed(container.pubkey, msg)?;
        let tweaked_hash = bitcoin::PublicKey {
            compressed: true,
            key: *tweaked_pk,
        }
        .pubkey_hash();

        let keys_count = RefCell::new(0);
        let hashes_count = RefCell::new(0);
        let found = RefCell::new(0);

        // ! [CONSENSUS-CRITICAL]:
        // ! [STANDARD-CRITICAL]: Iterate over all branches of the abstract
        //                        syntax tree generated by the Miniscript parser,
        //                        running the following algorithm for each node:
        let lockscript = container.script.replace_pubkeys_and_hashes(
            |pubkey: secp256k1::PublicKey| {
                *keys_count.borrow_mut() += 1;
                match pubkey == container.pubkey {
                    true => {
                        *found.borrow_mut() += 1;
                        Some(*tweaked_pk)
                    }
                    false => Some(pubkey),
                }
            },
            |hash: PubkeyHash| {
                *hashes_count.borrow_mut() += 1;
                match hash == original_hash {
                    true => {
                        *found.borrow_mut() += 1;
                        Some(tweaked_hash)
                    }
                    false => Some(hash),
                }
            },
        )?;

        // ! [CONSENSUS-CRITICAL]:
        // ! [STANDARD-CRITICAL]: If no public keys were found fail the
        //                        procedure; return the collected keys otherwise
        match (
            found.into_inner(),
            keys_count.into_inner(),
            hashes_count.into_inner(),
        ) {
            (0, 0, 0) => Err(Error::LockscriptContainsNoKeysOrHashes),
            (0, 0, _) => Err(Error::LockscriptContainsUnknownHashesOnly),
            (0, _, _) => Err(Error::LockscriptKeyNotFound),
            (_, _, _) => Ok(Self(lockscript)),
        }
    }
}

#[cfg(test)]
mod test {
    use bitcoin::hashes::{hash160, sha256, Hash};
    use bitcoin::secp256k1;
    use miniscript::Miniscript;
    use std::str::FromStr;

    use super::super::Error;
    use super::*;

    macro_rules! ms_str {
        ($($arg:tt)*) => (Miniscript::<bitcoin::PublicKey>::from_str(&format!($($arg)*)).unwrap())
    }

    macro_rules! policy_str {
    ($($arg:tt)*) => (miniscript::policy::Concrete::<bitcoin::PublicKey>::from_str(&format!($($arg)*)).unwrap())
}

    fn pubkeys(n: usize) -> Vec<bitcoin::PublicKey> {
        let mut ret = Vec::with_capacity(n);
        let secp = secp256k1::Secp256k1::new();
        let mut sk = [0; 32];
        for i in 1..n + 1 {
            sk[0] = i as u8;
            sk[1] = (i >> 8) as u8;
            sk[2] = (i >> 16) as u8;

            let pk = bitcoin::PublicKey {
                key: secp256k1::PublicKey::from_secret_key(
                    &secp,
                    &secp256k1::SecretKey::from_slice(&sk[..]).expect("secret key"),
                ),
                compressed: true,
            };
            ret.push(pk);
        }
        ret
    }

    fn gen_test_data() -> (Vec<bitcoin::PublicKey>, Vec<PubkeyHash>, Vec<hash160::Hash>) {
        let keys = pubkeys(13);
        let key_hashes = keys.iter().map(bitcoin::PublicKey::pubkey_hash).collect();
        let dummy_hashes = (1..13)
            .map(|i| hash160::Hash::from_inner([i; 20]))
            .collect();
        (keys, key_hashes, dummy_hashes)
    }

    #[test]
    fn test_no_keys_and_hashes() {
        let (keys, key_hashes, dummy_hashes) = gen_test_data();
        let sha_hash = sha256::Hash::hash(&"(nearly)random string".as_bytes());

        let ms = vec![
            ms_str!("older(921)"),
            ms_str!("sha256({})", sha_hash),
            ms_str!("hash256({})", sha_hash),
            ms_str!("hash160({})", dummy_hashes[0]),
            ms_str!("ripemd160({})", dummy_hashes[1]),
            ms_str!("hash160({})", dummy_hashes[2]),
        ];

        ms.into_iter()
            .map(|ms: Miniscript<bitcoin::PublicKey>| LockScript::from(ms.encode()))
            .for_each(|ls| {
                assert_eq!(
                    LockscriptCommitment::commit_embed(
                        LockscriptContainer {
                            script: ls,
                            pubkey: keys[0].key
                        },
                        &"Test message"
                    )
                    .err(),
                    Some(Error::LockscriptContainsNoKeysOrHashes)
                );
            });
    }

    #[test]
    fn test_unknown_key() {
        let (keys, key_hashes, dummy_hashes) = gen_test_data();
        let sha_hash = sha256::Hash::hash(&"(nearly)random string".as_bytes());

        let mut uncompressed = keys[5];
        uncompressed.compressed = false;
        let ms = vec![
            ms_str!("c:pk({})", keys[1]),
            ms_str!("c:pk({})", keys[2]),
            ms_str!("c:pk({})", keys[3]),
            ms_str!("c:pk({})", keys[4]),
            //ms_str!("c:pk({})", uncompressed),
        ];

        ms.into_iter()
            .map(|ms: Miniscript<bitcoin::PublicKey>| LockScript::from(ms.encode()))
            .for_each(|ls| {
                assert_eq!(
                    LockscriptCommitment::commit_embed(
                        LockscriptContainer {
                            script: ls,
                            pubkey: keys[0].key
                        },
                        &"Test message"
                    )
                    .err(),
                    Some(Error::LockscriptKeyNotFound)
                );
            });
    }

    #[test]
    fn test_unknown_hash() {
        let (keys, key_hashes, dummy_hashes) = gen_test_data();
        let sha_hash = sha256::Hash::hash(&"(nearly)random string".as_bytes());

        let ms = vec![
            ms_str!("c:pk_h({})", keys[1].pubkey_hash()),
            ms_str!("c:pk_h({})", keys[2].pubkey_hash()),
            ms_str!("c:pk_h({})", keys[3].pubkey_hash()),
            ms_str!("c:pk_h({})", keys[4].pubkey_hash()),
        ];

        ms.into_iter()
            .map(|ms: Miniscript<bitcoin::PublicKey>| LockScript::from(ms.encode()))
            .for_each(|ls| {
                assert_eq!(
                    LockscriptCommitment::commit_embed(
                        LockscriptContainer {
                            script: ls,
                            pubkey: keys[0].key
                        },
                        &"Test message"
                    )
                    .err(),
                    Some(Error::LockscriptContainsUnknownHashesOnly)
                );
            });
    }

    #[test]
    fn test_known_key() {
        let (keys, key_hashes, dummy_hashes) = gen_test_data();
        let sha_hash = sha256::Hash::hash(&"(nearly)random string".as_bytes());

        let mut uncompressed = keys[5];
        uncompressed.compressed = false;
        let ms = vec![
            ms_str!("c:pk({})", keys[0]),
            ms_str!("c:pk({})", keys[1]),
            ms_str!("c:pk({})", keys[2]),
            ms_str!("c:pk({})", keys[3]),
            //ms_str!("c:pk({})", uncompressed),
        ];

        ms.into_iter()
            .map(|ms: Miniscript<bitcoin::PublicKey>| LockScript::from(ms.encode()))
            .enumerate()
            .for_each(|(idx, ls)| {
                let container = LockscriptContainer {
                    script: ls,
                    pubkey: keys[idx].key,
                };
                let msg = "Test message";
                let commitment =
                    LockscriptCommitment::commit_embed(container.clone(), &msg).unwrap();
                assert!(commitment.verify(container, &msg))
            });
    }

    #[test]
    fn test_known_hash() {
        let (keys, key_hashes, dummy_hashes) = gen_test_data();
        let sha_hash = sha256::Hash::hash(&"(nearly)random string".as_bytes());

        let ms = vec![
            ms_str!("c:pk_h({})", keys[0].pubkey_hash()),
            ms_str!("c:pk_h({})", keys[1].pubkey_hash()),
            ms_str!("c:pk_h({})", keys[2].pubkey_hash()),
            ms_str!("c:pk_h({})", keys[3].pubkey_hash()),
        ];

        ms.into_iter()
            .map(|ms: Miniscript<bitcoin::PublicKey>| LockScript::from(ms.encode()))
            .enumerate()
            .for_each(|(idx, ls)| {
                let container = LockscriptContainer {
                    script: ls,
                    pubkey: keys[idx].key,
                };
                let msg = "Test message";
                let commitment =
                    LockscriptCommitment::commit_embed(container.clone(), &msg).unwrap();
                assert!(commitment.verify(container, &msg))
            });
    }

    #[test]
    fn test_multisig() {
        let (keys, key_hashes, dummy_hashes) = gen_test_data();
        let sha_hash = sha256::Hash::hash(&"(nearly)random string".as_bytes());

        let ms: Vec<Miniscript<bitcoin::PublicKey>> = vec![
            policy_str!("thresh(2,pk({}),pk({}))", keys[0], keys[1],),
            policy_str!(
                "thresh(3,pk({}),pk({}),pk({}),pk({}),pk({}))",
                keys[0],
                keys[1],
                keys[2],
                keys[3],
                keys[4]
            ),
        ]
        .into_iter()
        .map(|p| p.compile().unwrap())
        .collect();

        ms.into_iter()
            .map(|ms: Miniscript<bitcoin::PublicKey>| LockScript::from(ms.encode()))
            .enumerate()
            .for_each(|(idx, ls)| {
                let container = LockscriptContainer {
                    script: ls,
                    pubkey: keys[1].key,
                };
                let msg = "Test message";
                let commitment =
                    LockscriptCommitment::commit_embed(container.clone(), &msg).unwrap();
                assert!(commitment.verify(container, &msg))
            });
    }

    #[test]
    fn test_complex_scripts_unique_key() {
        let (keys, key_hashes, dummy_hashes) = gen_test_data();
        let sha_hash = sha256::Hash::hash(&"(nearly)random string".as_bytes());

        let ms = policy_str!(
            "or(thresh(3,pk({}),pk({}),pk({})),and(thresh(2,pk({}),pk({})),older(10000)))",
            keys[0],
            keys[1],
            keys[2],
            keys[3],
            keys[4],
        )
        .compile()
        .unwrap();

        let container = LockscriptContainer {
            script: LockScript::from(ms.encode()),
            pubkey: keys[1].key,
        };
        let msg = "Test message";
        let commitment = LockscriptCommitment::commit_embed(container.clone(), &msg).unwrap();
        assert!(commitment.verify(container, &msg))
    }
}
