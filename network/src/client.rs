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

use std::collections::HashMap;
use std::sync::{Arc, Weak};

use cio::IoChannel;
use ctimer::{TimeoutHandler, TimerApi, TimerLoop, TimerToken};
use parking_lot::RwLock;
use time::Duration;

use crate::p2p::Message as P2pMessage;
use crate::{Api, IntoSocketAddr, NetworkExtension, NetworkExtensionResult, NodeId};

struct ClientApi {
    extension: RwLock<Option<Weak<NetworkExtension>>>,
    p2p_channel: IoChannel<P2pMessage>,
    timer: TimerApi,
}

impl Api for ClientApi {
    fn send(&self, id: &NodeId, message: &[u8]) {
        let extension_guard = self.extension.read();
        let some_extension = extension_guard.as_ref().expect("Extension should be initialized");
        if let Some(extension) = some_extension.upgrade() {
            let need_encryption = extension.need_encryption();
            let extension_name = extension.name();
            let node_id = *id;
            let data = message.to_vec();
            let bytes = data.len();
            if let Err(err) = self.p2p_channel.send(P2pMessage::SendExtensionMessage {
                node_id,
                extension_name,
                need_encryption,
                data,
            }) {
                cerror!(
                    NETAPI,
                    "`{}` cannot send {} bytes message to {} : {:?}",
                    extension.name(),
                    bytes,
                    id.into_addr(),
                    err
                );
            } else {
                cdebug!(NETAPI, "`{}` sends {} bytes to {}", extension.name(), bytes, id.into_addr());
            }
        } else {
            cwarn!(NETAPI, "The extension already dropped");
        }
    }

    fn set_timer(&self, token: TimerToken, duration: Duration) -> NetworkExtensionResult<()> {
        let duration = duration.to_std().expect("Cannot convert to standard duratino type");
        self.timer.schedule_repeat(duration, token)?;
        Ok(())
    }

    fn set_timer_once(&self, token: TimerToken, duration: Duration) -> NetworkExtensionResult<()> {
        let duration = duration.to_std().expect("Cannot convert to standard duratino type");
        self.timer.schedule_once(duration, token)?;
        Ok(())
    }

    fn clear_timer(&self, token: TimerToken) -> NetworkExtensionResult<()> {
        self.timer.cancel(token)?;
        Ok(())
    }
}

impl TimeoutHandler for ClientApi {
    fn on_timeout(&self, token: TimerToken) {
        let extension_guard = self.extension.read();
        let some_extension = extension_guard.as_ref().expect("Extension should be initialized");
        if let Some(extension) = some_extension.upgrade() {
            extension.on_timeout(token);
        }
    }
}

pub struct Client {
    extensions: RwLock<HashMap<&'static str, Arc<NetworkExtension>>>,
    p2p_channel: IoChannel<P2pMessage>,
    timer_loop: TimerLoop,
}

impl Client {
    pub fn new_extension<T, F>(&self, factory: F) -> Arc<T>
    where
        T: 'static + Sized + NetworkExtension,
        F: FnOnce(Arc<Api>) -> T, {
        let mut extensions = self.extensions.write();
        let timer = self.timer_loop.new_timer();
        let api = {
            let p2p_channel = self.p2p_channel.clone();
            Arc::new(ClientApi {
                extension: RwLock::new(None),
                p2p_channel,
                timer,
            })
        };
        let extension = Arc::new(factory(Arc::clone(&api) as Arc<Api>));
        let name = extension.name();
        *api.extension.write() = Some(Arc::downgrade(&extension) as Weak<NetworkExtension>);
        api.timer.set_name(name);
        api.timer.set_handler(Arc::downgrade(&api));
        extension.on_initialize();
        let trait_extension = Arc::clone(&extension) as Arc<NetworkExtension>;
        if extensions.insert(name, trait_extension).is_some() {
            unreachable!("Duplicated extension name : {}", name)
        }
        extension
    }

    #[cfg_attr(feature = "cargo-clippy", allow(clippy::new_ret_no_self))]
    pub fn new(p2p_channel: IoChannel<P2pMessage>, timer_loop: TimerLoop) -> Arc<Self> {
        Arc::new(Self {
            extensions: RwLock::new(HashMap::new()),
            p2p_channel,
            timer_loop,
        })
    }

    pub fn extension_versions(&self) -> Vec<(String, Vec<u64>)> {
        let extensions = self.extensions.read();
        extensions.iter().map(|(name, extension)| (name.to_string(), extension.versions().to_vec())).collect()
    }

    pub fn on_node_removed(&self, id: &NodeId) {
        let extensions = self.extensions.read();
        for (_, extension) in extensions.iter() {
            extension.on_node_removed(id);
        }
    }

    pub fn on_node_added(&self, name: &str, id: &NodeId, version: u64) {
        let extensions = self.extensions.read();
        if let Some(extension) = extensions.get(name) {
            extension.on_node_added(id, version);
        } else {
            cdebug!(NETAPI, "{} doesn't exist.", name);
        }
    }

    pub fn on_message(&self, name: &str, id: &NodeId, data: &[u8]) {
        let extensions = self.extensions.read();
        if let Some(extension) = extensions.get(name) {
            cdebug!(NETAPI, "`{}` receives {} bytes from {}", name, data.len(), id.into_addr());
            extension.on_message(id, data);
        } else {
            cwarn!(NETAPI, "{} doesn't exist.", name);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Deref;
    use std::vec::Vec;

    use cio::IoService;
    use ctimer::TimerLoop;
    use parking_lot::Mutex;
    use time::Duration;

    use super::{Api, Client, NetworkExtension, NetworkExtensionResult, NodeId};
    use crate::SocketAddr;

    #[allow(dead_code)]
    struct TestApi;

    impl Api for TestApi {
        fn send(&self, _id: &NodeId, _message: &[u8]) {
            unimplemented!()
        }

        fn set_timer(&self, _timer_id: usize, _duration: Duration) -> NetworkExtensionResult<()> {
            unimplemented!()
        }

        fn set_timer_once(&self, _timer_id: usize, _duration: Duration) -> NetworkExtensionResult<()> {
            unimplemented!()
        }

        fn clear_timer(&self, _timer_id: usize) -> NetworkExtensionResult<()> {
            unimplemented!()
        }
    }

    #[derive(Debug, Eq, PartialEq)]
    enum Callback {
        Initialize,
        NodeAdded,
        NodeRemoved,
        Message,
        Timeout,
    }

    struct TestExtension {
        name: &'static str,
        callbacks: Mutex<Vec<Callback>>,
    }

    impl TestExtension {
        fn new(name: &'static str) -> Self {
            Self {
                name,
                callbacks: Mutex::new(vec![]),
            }
        }
    }

    impl NetworkExtension for TestExtension {
        fn name(&self) -> &'static str {
            self.name
        }

        fn need_encryption(&self) -> bool {
            false
        }

        fn versions(&self) -> &[u64] {
            const VERSIONS: &[u64] = &[0];
            &VERSIONS
        }

        fn on_initialize(&self) {
            let mut callbacks = self.callbacks.lock();
            callbacks.push(Callback::Initialize);
        }

        fn on_node_added(&self, _id: &NodeId, _version: u64) {
            let mut callbacks = self.callbacks.lock();
            callbacks.push(Callback::NodeAdded);
        }

        fn on_node_removed(&self, _id: &NodeId) {
            let mut callbacks = self.callbacks.lock();
            callbacks.push(Callback::NodeRemoved);
        }

        fn on_message(&self, _id: &NodeId, _message: &[u8]) {
            let mut callbacks = self.callbacks.lock();
            callbacks.push(Callback::Message);
        }

        fn on_timeout(&self, _timer_id: usize) {
            let mut callbacks = self.callbacks.lock();
            callbacks.push(Callback::Timeout);
        }
    }

    #[test]
    fn message_only_to_target() {
        let p2p_service = IoService::start("P2P").unwrap();
        let timer_loop = TimerLoop::new(2);

        let client = Client::new(p2p_service.channel(), timer_loop);

        let node_id1 = SocketAddr::v4(127, 0, 0, 1, 8081).into();
        let node_id5 = SocketAddr::v4(127, 0, 0, 1, 8085).into();

        let e1 = client.new_extension(|_| TestExtension::new("e1"));
        let e2 = client.new_extension(|_| TestExtension::new("e2"));

        client.on_message(&"e1".to_string(), &node_id1, &[]);
        {
            let callbacks = e1.callbacks.lock();
            assert_eq!(callbacks.deref(), &vec![Callback::Initialize, Callback::Message]);
            let callbacks = e2.callbacks.lock();
            assert_eq!(callbacks.deref(), &vec![Callback::Initialize]);
        }

        client.on_message(&"e2".to_string(), &node_id1, &[]);
        {
            let callbacks = e1.callbacks.lock();
            assert_eq!(callbacks.deref(), &vec![Callback::Initialize, Callback::Message]);
            let callbacks = e2.callbacks.lock();
            assert_eq!(callbacks.deref(), &vec![Callback::Initialize, Callback::Message]);
        }

        client.on_message(&"e2".to_string(), &node_id5, &[]);
        client.on_message(&"e2".to_string(), &node_id1, &[]);
        {
            let callbacks = e1.callbacks.lock();
            assert_eq!(callbacks.deref(), &vec![Callback::Initialize, Callback::Message]);
            let callbacks = e2.callbacks.lock();
            assert_eq!(
                callbacks.deref(),
                &vec![Callback::Initialize, Callback::Message, Callback::Message, Callback::Message]
            );
        }
    }
}
