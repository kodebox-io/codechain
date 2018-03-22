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

extern crate codechain_bytes as cbytes;
extern crate codechain_crypto as ccrypto;
extern crate codechain_io as cio;
extern crate codechain_json as cjson;
extern crate codechain_keys as ckeys;
extern crate codechain_types as ctypes;
extern crate hashdb;
extern crate heapsize;
extern crate kvdb;
extern crate kvdb_rocksdb;
extern crate kvdb_memorydb;
extern crate memorydb;
extern crate num_cpus;
extern crate patricia_trie as trie;
extern crate rlp;
extern crate rlp_compress;
#[macro_use]
extern crate rlp_derive;
extern crate parking_lot;
extern crate time;
extern crate triehash;
extern crate unexpected;
extern crate util_error;

#[macro_use]
extern crate log;

mod block;
mod blockchain;
mod blockchain_info;
mod client;
mod codechain_machine;
mod consensus;
mod db;
mod encoded;
mod error;
mod header;
mod machine;
mod service;
mod spec;
mod transaction;
mod types;
mod verification;
mod views;

#[cfg(test)]
mod tests;

pub use client::Client;
pub use error::Error;
pub use service::ClientService;
pub use spec::Spec;
