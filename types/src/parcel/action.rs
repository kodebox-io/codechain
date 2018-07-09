// Copyright 2018 Kodebox, Inc.
// This file is part of CodeChain.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use ckey::{Address, Public};
use primitives::{H256, U256};
use rlp::{Decodable, DecoderError, Encodable, RlpStream, UntrustedRlp};

use super::super::transaction::Transaction;

const CHANGE_SHARD_STATE: u8 = 1;
const PAYMENT: u8 = 2;
const SET_REGULAR_KEY: u8 = 3;
const CREATE_SHARD: u8 = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, RlpDecodable, RlpEncodable)]
#[serde(rename_all = "camelCase")]
pub struct ChangeShard {
    pub shard_id: u32,
    pub pre_root: H256,
    pub post_root: H256,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "action")]
pub enum Action {
    ChangeShardState {
        /// Transaction, can be either asset mint or asset transfer
        transactions: Vec<Transaction>,
        changes: Vec<ChangeShard>,
    },
    Payment {
        receiver: Address,
        /// Transferred amount.
        amount: U256,
    },
    SetRegularKey {
        key: Public,
    },
    CreateShard,
}

impl Encodable for Action {
    fn rlp_append(&self, s: &mut RlpStream) {
        match self {
            Action::ChangeShardState {
                transactions,
                changes,
            } => {
                s.begin_list(3);
                s.append(&CHANGE_SHARD_STATE);
                s.append_list(transactions);
                s.append_list(changes);
            }
            Action::Payment {
                receiver,
                amount,
            } => {
                s.begin_list(3);
                s.append(&PAYMENT);
                s.append(receiver);
                s.append(amount);
            }
            Action::SetRegularKey {
                key,
            } => {
                s.begin_list(2);
                s.append(&SET_REGULAR_KEY);
                s.append(key);
            }
            Action::CreateShard => {
                s.begin_list(1);
                s.append(&CREATE_SHARD);
            }
        }
    }
}

impl Decodable for Action {
    fn decode(rlp: &UntrustedRlp) -> Result<Self, DecoderError> {
        match rlp.val_at(0)? {
            CHANGE_SHARD_STATE => {
                if rlp.item_count()? != 3 {
                    return Err(DecoderError::RlpIncorrectListLen)
                }
                Ok(Action::ChangeShardState {
                    transactions: rlp.list_at(1)?,
                    changes: rlp.list_at(2)?,
                })
            }
            PAYMENT => {
                if rlp.item_count()? != 3 {
                    return Err(DecoderError::RlpIncorrectListLen)
                }
                Ok(Action::Payment {
                    receiver: rlp.val_at(1)?,
                    amount: rlp.val_at(2)?,
                })
            }
            SET_REGULAR_KEY => {
                if rlp.item_count()? != 2 {
                    return Err(DecoderError::RlpIncorrectListLen)
                }
                Ok(Action::SetRegularKey {
                    key: rlp.val_at(1)?,
                })
            }
            CREATE_SHARD => {
                if rlp.item_count()? != 1 {
                    return Err(DecoderError::RlpIncorrectListLen)
                }
                Ok(Action::CreateShard)
            }
            _ => Err(DecoderError::Custom("Unexpected action prefix")),
        }
    }
}
