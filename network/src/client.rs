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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{Builder, JoinHandle};

use cio::IoChannel;
use crossbeam_channel as crossbeam;
use ctimer::{TimeoutHandler, TimerApi, TimerLoop, TimerToken};
use parking_lot::{Mutex, RwLock};
use time::Duration;

use crate::p2p::Message as P2pMessage;
use crate::{Api, IntoSocketAddr, NetworkExtension, NetworkExtensionResult, NodeId};

struct ClientApi {
    p2p_channel: IoChannel<P2pMessage>,
    timer: TimerApi,
    channel: Mutex<crossbeam::Sender<ExtensionMessage>>,
    name: Mutex<Option<&'static str>>,
    need_encryption: AtomicBool,
}

impl Api for ClientApi {
    fn send(&self, id: &NodeId, message: &[u8]) {
        let need_encryption = self.need_encryption.load(Ordering::SeqCst);
        let extension_name = self.name.lock().expect("send must be called after initialized");
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
                extension_name,
                bytes,
                id.into_addr(),
                err
            );
        } else {
            cdebug!(NETAPI, "`{}` sends {} bytes to {}", extension_name, bytes, id.into_addr());
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
        let channel = self.channel.lock();
        if let Err(err) = channel.send(ExtensionMessage::Timeout(token)) {
            cwarn!(
                NETAPI,
                "{} cannot timeout {}: {:?}",
                self.name.lock().expect("send must be called after initialized"),
                token,
                err
            );
        }
    }
}

struct Extension {
    versions: Vec<u64>,
    sender: Mutex<crossbeam::Sender<ExtensionMessage>>,
    quit: Mutex<crossbeam::Sender<()>>,
    join: Mutex<Option<JoinHandle<()>>>,
}

impl Drop for Extension {
    fn drop(&mut self) {
        let _ = self.quit.lock().send(());
        if let Some(join) = self.join.lock().take() {
            join.join().unwrap();
        }
    }
}

pub struct Client {
    extensions: RwLock<HashMap<&'static str, Extension>>,
    p2p_channel: IoChannel<P2pMessage>,
    timer_loop: TimerLoop,
}

impl Client {
    pub fn register_extension<T, E, F>(&self, factory: F) -> (crossbeam::Sender<E>, Arc<T>)
    where
        T: 'static + Sized + NetworkExtension<E>,
        E: 'static + Sized + Send,
        F: FnOnce(Arc<Api>) -> T, {
        let mut extensions = self.extensions.write();
        let timer = self.timer_loop.new_timer();
        let (api, channel, rx) = {
            let p2p_channel = self.p2p_channel.clone();
            let (channel, rx) = crossbeam::unbounded();
            (
                Arc::new(ClientApi {
                    name: Default::default(),
                    need_encryption: Default::default(),
                    p2p_channel,
                    timer,
                    channel: channel.clone().into(),
                }),
                channel,
                rx,
            )
        };
        let extension = Arc::new(factory(Arc::clone(&api) as Arc<Api>));
        let name = extension.name();
        let versions = extension.versions().to_vec();
        let need_encryption = extension.need_encryption();

        let (quit_sender, quit_receiver) = crossbeam::bounded(1);
        let (init_sender, init_receiver) = crossbeam::bounded(1);
        let (event_sender, event_receiver) = crossbeam::unbounded();

        let cloned_extension = Arc::clone(&extension);

        let join = Some(
            Builder::new()
                .name(format!("{}.ext", name))
                .spawn(move || {
                    *api.name.lock() = Some(name);
                    if need_encryption {
                        api.need_encryption.store(true, Ordering::SeqCst);
                    }
                    api.timer.set_name(name);
                    api.timer.set_handler(Arc::downgrade(&api));

                    init_receiver.recv().expect("The main thread must send one message");
                    extension.on_initialize();
                    let mut s = crossbeam::Select::new();
                    let rx_index = s.recv(&rx);
                    let quit_index = s.recv(&quit_receiver);
                    let mut event_closed = false;
                    loop {
                        let mut s = s.clone();
                        // Not all extension uses event channel, so closing the event channel is natural thing.
                        // TODO: Please make this dynamic selection simply.
                        let event_index = if event_closed {
                            // It's a trick using that the index increases sequentially form 0.
                            // TODO: Please remove this magic number.
                            ::std::usize::MAX
                        } else {
                            s.recv(&event_receiver)
                        };
                        match s.ready() {
                            index if index == rx_index => match rx.try_recv() {
                                Ok(ExtensionMessage::NodeAdded(id, version)) => {
                                    extension.on_node_added(&id, version);
                                }
                                Ok(ExtensionMessage::NodeRemoved(id)) => {
                                    extension.on_node_removed(&id);
                                }
                                Ok(ExtensionMessage::Timeout(token)) => {
                                    extension.on_timeout(token);
                                }
                                Ok(ExtensionMessage::Message(id, message)) => {
                                    extension.on_message(&id, &message);
                                }
                                Err(crossbeam::TryRecvError::Empty) => continue, // Handle a spuriously wake-up
                                Err(crossbeam::TryRecvError::Disconnected) => {
                                    cinfo!(NETAPI, "The channel for {} had been disconnected", name);
                                    break
                                }
                            },
                            index if index == quit_index => match quit_receiver.try_recv() {
                                Ok(()) => break,
                                Err(crossbeam::TryRecvError::Empty) => continue, // Handle a spuriously wake-up
                                Err(crossbeam::TryRecvError::Disconnected) => {
                                    cinfo!(NETAPI, "The quit channel for {} had been disconnected", name);
                                    break
                                }
                            },
                            index if index == event_index => {
                                assert!(!event_closed);
                                match event_receiver.try_recv() {
                                    Ok(event) => {
                                        extension.on_event(event);
                                    }
                                    Err(crossbeam::TryRecvError::Empty) => continue, // Handle a spuriously wake-up
                                    Err(crossbeam::TryRecvError::Disconnected) => {
                                        event_closed = true;
                                        cdebug!(NETAPI, "The event channel for {} had been disconnected", name);
                                        continue
                                    }
                                }
                            }
                            index => {
                                unreachable!("{} is not an expected index of message queue", index);
                            }
                        }
                    }
                })
                .unwrap(),
        )
        .into();

        if extensions
            .insert(
                name,
                Extension {
                    versions,
                    sender: channel.into(),
                    quit: quit_sender.into(),
                    join,
                },
            )
            .is_some()
        {
            unreachable!("Duplicated extension name : {}", name)
        }
        init_sender.send(()).unwrap();
        (event_sender, cloned_extension)
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
        extensions.iter().map(|(name, extension)| (name.to_string(), extension.versions.clone())).collect()
    }

    pub fn on_node_removed(&self, id: &NodeId) {
        let extensions = self.extensions.read();
        for (name, extension) in extensions.iter() {
            if let Err(err) = extension.sender.lock().send(ExtensionMessage::NodeRemoved(*id)) {
                cwarn!(NETAPI, "{} cannot remove {}: {:?}", name, id, err);
            }
        }
    }

    pub fn on_node_added(&self, name: &str, id: &NodeId, version: u64) {
        let extensions = self.extensions.read();
        if let Some(extension) = extensions.get(name) {
            if let Err(err) = extension.sender.lock().send(ExtensionMessage::NodeAdded(*id, version)) {
                cwarn!(NETAPI, "{} cannot add {}:{}: {:?}", name, id, version, err);
            }
        } else {
            cdebug!(NETAPI, "{} doesn't exist.", name);
        }
    }

    pub fn on_message(&self, name: &str, id: &NodeId, data: &[u8]) {
        let extensions = self.extensions.read();
        if let Some(extension) = extensions.get(name) {
            cdebug!(NETAPI, "`{}` receives {} bytes from {}", name, data.len(), id.into_addr());
            if let Err(err) = extension.sender.lock().send(ExtensionMessage::Message(*id, data.to_vec())) {
                cwarn!(NETAPI, "{} cannot message {}: {:?}", name, id, err);
            }
        } else {
            cwarn!(NETAPI, "{} doesn't exist.", name);
        }
    }
}

enum ExtensionMessage {
    Message(NodeId, Vec<u8>),
    NodeAdded(NodeId, u64),
    NodeRemoved(NodeId),
    Timeout(TimerToken),
}

#[cfg(test)]
mod tests {
    use cio::IoService;
    use never::Never;

    use super::*;
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

    impl NetworkExtension<Never> for TestExtension {
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

        let _e1 = client.register_extension(|_| TestExtension::new("e1"));
        let _e2 = client.register_extension(|_| TestExtension::new("e2"));

        // FIXME: The callback is asynchronous, find a way to test it.

        client.on_message(&"e1".to_string(), &node_id1, &[]);

        client.on_message(&"e2".to_string(), &node_id1, &[]);

        client.on_message(&"e2".to_string(), &node_id5, &[]);
        client.on_message(&"e2".to_string(), &node_id1, &[]);
    }
}
