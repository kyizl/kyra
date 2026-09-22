use bytes::Bytes;
use std::collections::VecDeque;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionPhase {
    Login,
    Configuration,
    Play,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticationMode {
    Offline,
    Premium,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticationAttempt {
    OfflineFirst,
    PremiumRetry,
    Completed(AuthenticationMode),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AuthenticationTransitionError {
    #[error("premium authentication retry is only allowed after encryption begins")]
    PremiumRetryBeforeEncryption,
    #[error("authentication attempt has already completed")]
    AlreadyCompleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthenticationLifecycle {
    attempt: AuthenticationAttempt,
    encryption_started: bool,
}

impl Default for AuthenticationLifecycle {
    fn default() -> Self {
        Self {
            attempt: AuthenticationAttempt::OfflineFirst,
            encryption_started: false,
        }
    }
}

impl AuthenticationLifecycle {
    pub fn attempt(&self) -> AuthenticationAttempt {
        self.attempt
    }

    pub fn encryption_started(&self) -> bool {
        self.encryption_started
    }

    pub fn encryption_begin(&mut self) -> Result<(), AuthenticationTransitionError> {
        if matches!(self.attempt, AuthenticationAttempt::Completed(_)) {
            return Err(AuthenticationTransitionError::AlreadyCompleted);
        }
        self.encryption_started = true;
        self.attempt = AuthenticationAttempt::PremiumRetry;
        Ok(())
    }

    pub fn premium_succeeded(&mut self) -> Result<(), AuthenticationTransitionError> {
        if !self.encryption_started {
            return Err(AuthenticationTransitionError::PremiumRetryBeforeEncryption);
        }
        if matches!(self.attempt, AuthenticationAttempt::Completed(_)) {
            return Err(AuthenticationTransitionError::AlreadyCompleted);
        }
        self.attempt = AuthenticationAttempt::Completed(AuthenticationMode::Premium);
        Ok(())
    }

    pub fn offline_succeeded(&mut self) -> Result<(), AuthenticationTransitionError> {
        if matches!(self.attempt, AuthenticationAttempt::Completed(_)) {
            return Err(AuthenticationTransitionError::AlreadyCompleted);
        }
        self.attempt = AuthenticationAttempt::Completed(AuthenticationMode::Offline);
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StateTransitionError {
    #[error("cannot transition from {from:?} to {to:?}")]
    Invalid {
        from: ConnectionPhase,
        to: ConnectionPhase,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionState {
    pub client: ConnectionPhase,
    pub upstream: ConnectionPhase,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            client: ConnectionPhase::Login,
            upstream: ConnectionPhase::Login,
        }
    }
}

impl ConnectionState {
    pub fn transition_client(
        &mut self,
        phase: ConnectionPhase,
    ) -> Result<(), StateTransitionError> {
        Self::transition(&mut self.client, phase)
    }

    pub fn transition_upstream(
        &mut self,
        phase: ConnectionPhase,
    ) -> Result<(), StateTransitionError> {
        Self::transition(&mut self.upstream, phase)
    }

    pub fn set_client(&mut self, phase: ConnectionPhase) {
        self.client = phase;
    }

    fn transition(
        current: &mut ConnectionPhase,
        next: ConnectionPhase,
    ) -> Result<(), StateTransitionError> {
        let valid = matches!(
            (*current, next),
            (ConnectionPhase::Login, ConnectionPhase::Configuration)
                | (ConnectionPhase::Login, ConnectionPhase::Play)
                | (ConnectionPhase::Login, ConnectionPhase::Closed)
                | (ConnectionPhase::Configuration, ConnectionPhase::Play)
                | (ConnectionPhase::Configuration, ConnectionPhase::Closed)
                | (ConnectionPhase::Play, ConnectionPhase::Closed)
        );
        if !valid {
            return Err(StateTransitionError::Invalid {
                from: *current,
                to: next,
            });
        }
        *current = next;
        Ok(())
    }

    pub fn set_upstream(&mut self, phase: ConnectionPhase) {
        self.upstream = phase;
    }

    pub fn close_client(&mut self) {
        self.client = ConnectionPhase::Closed;
    }

    pub fn close_upstream(&mut self) {
        self.upstream = ConnectionPhase::Closed;
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum QueueError {
    #[error("pending packet queue capacity must be greater than zero")]
    InvalidCapacity,
    #[error("pending packet queue is full at {capacity} packets")]
    Full { capacity: usize },
    #[error("packets cannot be queued for the {0:?} phase")]
    InvalidPhase(ConnectionPhase),
}

#[derive(Debug)]
pub struct PendingPackets {
    capacity: usize,
    packets: VecDeque<Bytes>,
}

impl PendingPackets {
    pub fn new(capacity: usize) -> Result<Self, QueueError> {
        if capacity == 0 {
            return Err(QueueError::InvalidCapacity);
        }
        Ok(Self {
            capacity,
            packets: VecDeque::with_capacity(capacity),
        })
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        self.packets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.packets.is_empty()
    }

    pub fn push(&mut self, packet: Bytes) -> Result<(), QueueError> {
        if self.packets.len() >= self.capacity {
            return Err(QueueError::Full {
                capacity: self.capacity,
            });
        }
        self.packets.push_back(packet);
        Ok(())
    }

    pub fn pop(&mut self) -> Option<Bytes> {
        self.packets.pop_front()
    }

    pub fn drain(&mut self) -> impl Iterator<Item = Bytes> + '_ {
        self.packets.drain(..)
    }
}

#[derive(Debug)]
pub struct PendingPacketQueues {
    pub configuration: PendingPackets,
    pub play: PendingPackets,
}

impl PendingPacketQueues {
    pub fn new(configuration_capacity: usize, play_capacity: usize) -> Result<Self, QueueError> {
        Ok(Self {
            configuration: PendingPackets::new(configuration_capacity)?,
            play: PendingPackets::new(play_capacity)?,
        })
    }
}

#[derive(Debug)]
pub struct ConnectionRuntime {
    state: ConnectionState,
    queues: PendingPacketQueues,
}

impl ConnectionRuntime {
    pub fn new(configuration_capacity: usize, play_capacity: usize) -> Result<Self, QueueError> {
        Ok(Self {
            state: ConnectionState::default(),
            queues: PendingPacketQueues::new(configuration_capacity, play_capacity)?,
        })
    }

    pub fn state(&self) -> ConnectionState {
        self.state
    }

    pub fn queues(&self) -> &PendingPacketQueues {
        &self.queues
    }

    pub fn queues_mut(&mut self) -> &mut PendingPacketQueues {
        &mut self.queues
    }

    pub fn transition_client(
        &mut self,
        phase: ConnectionPhase,
    ) -> Result<(), StateTransitionError> {
        self.state.transition_client(phase)
    }

    pub fn transition_upstream(
        &mut self,
        phase: ConnectionPhase,
    ) -> Result<(), StateTransitionError> {
        self.state.transition_upstream(phase)
    }

    pub fn queue(&mut self, phase: ConnectionPhase, packet: Bytes) -> Result<(), QueueError> {
        match phase {
            ConnectionPhase::Configuration => self.queues.configuration.push(packet),
            ConnectionPhase::Play => self.queues.play.push(packet),
            invalid => Err(QueueError::InvalidPhase(invalid)),
        }
    }

    pub fn drain(&mut self, phase: ConnectionPhase) -> Result<Vec<Bytes>, QueueError> {
        match phase {
            ConnectionPhase::Configuration => Ok(self.queues.configuration.drain().collect()),
            ConnectionPhase::Play => Ok(self.queues.play.drain().collect()),
            invalid => Err(QueueError::InvalidPhase(invalid)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_tracks_client_and_upstream_independently() {
        let mut state = ConnectionState::default();
        state.set_client(ConnectionPhase::Configuration);
        state.set_upstream(ConnectionPhase::Play);
        assert_eq!(state.client, ConnectionPhase::Configuration);
        assert_eq!(state.upstream, ConnectionPhase::Play);
        state.close_client();
        assert_eq!(state.client, ConnectionPhase::Closed);
        assert_eq!(state.upstream, ConnectionPhase::Play);
    }

    #[test]
    fn rejects_invalid_state_transitions() {
        let mut state = ConnectionState::default();
        assert_eq!(state.transition_client(ConnectionPhase::Closed), Ok(()));
        assert_eq!(
            state.transition_client(ConnectionPhase::Play),
            Err(StateTransitionError::Invalid {
                from: ConnectionPhase::Closed,
                to: ConnectionPhase::Play,
            })
        );
    }

    #[test]
    fn queues_preserve_order_and_bound_growth() {
        let mut queue = PendingPackets::new(2).unwrap();
        queue.push(Bytes::from_static(&[1])).unwrap();
        queue.push(Bytes::from_static(&[2])).unwrap();
        assert_eq!(
            queue.push(Bytes::from_static(&[3])),
            Err(QueueError::Full { capacity: 2 })
        );
        assert_eq!(queue.pop(), Some(Bytes::from_static(&[1])));
        queue.push(Bytes::from_static(&[3])).unwrap();
        assert_eq!(
            queue.drain().collect::<Vec<_>>(),
            vec![Bytes::from_static(&[2]), Bytes::from_static(&[3])]
        );
    }

    #[test]
    fn rejects_zero_capacity() {
        assert_eq!(
            PendingPackets::new(0).unwrap_err(),
            QueueError::InvalidCapacity
        );
        assert_eq!(
            PendingPacketQueues::new(1, 0).unwrap_err(),
            QueueError::InvalidCapacity
        );
    }

    #[test]
    fn authentication_starts_offline_and_retries_premium_only_after_encryption() {
        let mut lifecycle = AuthenticationLifecycle::default();
        assert_eq!(lifecycle.attempt(), AuthenticationAttempt::OfflineFirst);
        assert_eq!(
            lifecycle.premium_succeeded(),
            Err(AuthenticationTransitionError::PremiumRetryBeforeEncryption)
        );
        lifecycle.encryption_begin().unwrap();
        assert_eq!(lifecycle.attempt(), AuthenticationAttempt::PremiumRetry);
        lifecycle.premium_succeeded().unwrap();
        assert_eq!(
            lifecycle.attempt(),
            AuthenticationAttempt::Completed(AuthenticationMode::Premium)
        );
    }

    #[test]
    fn offline_success_prevents_retries() {
        let mut lifecycle = AuthenticationLifecycle::default();
        lifecycle.offline_succeeded().unwrap();
        assert_eq!(
            lifecycle.encryption_begin(),
            Err(AuthenticationTransitionError::AlreadyCompleted)
        );
    }

    #[test]
    fn runtime_validates_transitions_and_queues() {
        let mut runtime = ConnectionRuntime::new(2, 2).unwrap();
        assert_eq!(
            runtime.queue(ConnectionPhase::Login, Bytes::from_static(&[1])),
            Err(QueueError::InvalidPhase(ConnectionPhase::Login))
        );
        runtime
            .transition_client(ConnectionPhase::Configuration)
            .unwrap();
        runtime
            .queue(ConnectionPhase::Configuration, Bytes::from_static(&[2]))
            .unwrap();
        assert_eq!(
            runtime.drain(ConnectionPhase::Configuration).unwrap(),
            vec![Bytes::from_static(&[2])]
        );
    }
}
