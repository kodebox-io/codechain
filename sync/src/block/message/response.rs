// Copyright 2018-2019 Kodebox, Inc.
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

use super::MessageID;
use ccore::UnverifiedTransaction;
use ctypes::Header;
use rlp::{DecoderError, Encodable, Rlp, RlpStream};
use snap;

#[derive(Debug)]
pub enum ResponseMessage {
    Headers(Vec<Header>),
    Bodies(Vec<Vec<UnverifiedTransaction>>),
    StateHead(Vec<u8>),
    StateChunk(Vec<u8>),
}

impl Encodable for ResponseMessage {
    fn rlp_append(&self, s: &mut RlpStream) {
        match self {
            ResponseMessage::Headers(headers) => {
                s.append_list(headers);
            }
            ResponseMessage::Bodies(bodies) => {
                s.begin_list(1);

                let uncompressed = {
                    let mut inner_list = RlpStream::new_list(bodies.len());
                    bodies.iter().for_each(|body| {
                        inner_list.append_list(body);
                    });
                    inner_list.out()
                };

                let compressed = {
                    // TODO: Cache the Encoder object
                    let mut snappy_encoder = snap::Encoder::new();
                    snappy_encoder.compress_vec(&uncompressed).expect("Compression always succeed")
                };

                s.append(&compressed);
            }
            ResponseMessage::StateHead(bytes) => {
                s.begin_list(1);
                s.append(bytes);
            }
            ResponseMessage::StateChunk(bytes) => {
                s.begin_list(1);
                s.append(bytes);
            }
        };
    }
}

impl ResponseMessage {
    pub fn message_id(&self) -> MessageID {
        match self {
            ResponseMessage::Headers {
                ..
            } => MessageID::Headers,
            ResponseMessage::Bodies(..) => MessageID::Bodies,
            ResponseMessage::StateHead(..) => MessageID::StateHead,
            ResponseMessage::StateChunk {
                ..
            } => MessageID::StateChunk,
        }
    }

    pub fn decode(id: MessageID, rlp: &Rlp) -> Result<Self, DecoderError> {
        let message = match id {
            MessageID::Headers => ResponseMessage::Headers(rlp.as_list()?),
            MessageID::Bodies => {
                let item_count = rlp.item_count()?;
                if item_count != 1 {
                    return Err(DecoderError::RlpIncorrectListLen {
                        got: item_count,
                        expected: 1,
                    })
                }

                let compressed: Vec<u8> = rlp.val_at(0)?;
                let uncompressed = {
                    // TODO: Cache the Decoder object
                    let mut snappy_decoder = snap::Decoder::new();
                    snappy_decoder.decompress_vec(&compressed).map_err(|err| {
                        cwarn!(SYNC, "Decompression failed while decoding a body response: {}", err);
                        DecoderError::Custom("Invalid compression format")
                    })?
                };

                let uncompressed_rlp = Rlp::new(&uncompressed);

                let mut bodies = Vec::new();
                for item in uncompressed_rlp.into_iter() {
                    bodies.push(item.as_list()?);
                }
                ResponseMessage::Bodies(bodies)
            }
            MessageID::StateHead => {
                let item_count = rlp.item_count()?;
                if item_count != 1 {
                    return Err(DecoderError::RlpIncorrectListLen {
                        got: item_count,
                        expected: 1,
                    })
                }
                ResponseMessage::StateHead(rlp.val_at(0)?)
            }
            MessageID::StateChunk => {
                let item_count = rlp.item_count()?;
                if item_count != 1 {
                    return Err(DecoderError::RlpIncorrectListLen {
                        got: item_count,
                        expected: 1,
                    })
                }
                ResponseMessage::StateChunk(rlp.val_at(0)?)
            }
            _ => return Err(DecoderError::Custom("Unknown message id detected")),
        };

        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use rlp::{Encodable, Rlp};

    use ccore::UnverifiedTransaction;
    use ckey::{Address, Signature};
    use ctypes::transaction::{Action, Transaction};
    use ctypes::Header;

    use super::{MessageID, ResponseMessage};

    pub fn decode_bytes(id: MessageID, bytes: &[u8]) -> ResponseMessage {
        let rlp = Rlp::new(bytes);
        ResponseMessage::decode(id, &rlp).unwrap()
    }

    /// For a type that does not have PartialEq, uses Debug instead.
    fn assert_eq_by_debug<T: std::fmt::Debug>(a: &T, b: &T) {
        assert_eq!(format!("{:?}", a), format!("{:?}", b));
    }

    #[test]
    fn headers_message_rlp() {
        let headers = vec![Header::default()];
        headers.iter().for_each(|header| {
            header.hash();
        });

        let message = ResponseMessage::Headers(headers);
        assert_eq_by_debug(&message, &decode_bytes(message.message_id(), message.rlp_bytes().as_ref()))
    }

    #[test]
    fn bodies_message_rlp() {
        let message = ResponseMessage::Bodies(vec![vec![]]);
        assert_eq_by_debug(&message, &decode_bytes(message.message_id(), message.rlp_bytes().as_ref()));

        let tx = UnverifiedTransaction::new(
            Transaction {
                seq: 0,
                fee: 10,
                action: Action::CreateShard {
                    users: vec![Address::random(), Address::random()],
                },
                network_id: "tc".into(),
            },
            Signature::default(),
        );

        let message = ResponseMessage::Bodies(vec![vec![tx]]);
        assert_eq_by_debug(&message, &decode_bytes(message.message_id(), message.rlp_bytes().as_ref()));
    }

    #[test]
    fn state_head_message_rlp() {
        let message = ResponseMessage::StateHead(vec![]);
        assert_eq_by_debug(&message, &decode_bytes(message.message_id(), message.rlp_bytes().as_ref()));
    }

    #[test]
    fn state_chunk_message_rlp() {
        let message = ResponseMessage::StateChunk(vec![]);
        assert_eq_by_debug(&message, &decode_bytes(message.message_id(), message.rlp_bytes().as_ref()));
    }
}
