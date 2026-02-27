use std::{
    sync::{self},
    time,
};

use serde::{Deserialize, Serialize};
use tokio::{sync::broadcast, task::JoinHandle};

use crate::{clock, platforms, updater};

pub fn blocking_abort_and_clear_tasks<T, F>(tasks: &mut Vec<T>, f: F)
where
    F: Fn(T) -> JoinHandle<()>,
{
    let updaters = std::mem::take(tasks);
    for task in updaters {
        let task = f(task);
        let task_id = task.id();
        log::debug!("# | task_mgr | task_id={task_id} | aborting...");
        task.abort();
        async_scoped::TokioScope::scope_and_block(|scope| {
            scope.spawn(async {
                let _ = task.await;
            });
        });
        // we can't use await here, because the provided vector from a mutex is not 'static
        // hence the scoped to work with non-'static data
        log::debug!("# | task_mgr | task_id={task_id} | aborting... | aborted_ok");
    }
}

/// Variation of the `tokio::sync::broadcast` channel, where the sender doesn't
/// care if any receiver is listening. Useful to ensure, that all receivers get only the latest value.
#[derive(Clone)]
pub struct FireAndForgetChannel<T, C = DefaultAlwaysImmediateSend<T>>
where
    C: SendBehavior,
{
    inner: broadcast::Sender<T>,
    send_behavior: C,
}

pub trait SendBehavior: Clone {}

#[derive(Clone)]
pub struct DefaultAlwaysImmediateSend<T> {
    most_recent_sent_value: Option<T>,
}

impl<T> Default for DefaultAlwaysImmediateSend<T> {
    fn default() -> Self {
        Self {
            most_recent_sent_value: None,
        }
    }
}

impl<T: Clone> SendBehavior for DefaultAlwaysImmediateSend<T> {}

#[derive(Clone)]
pub struct OnlyChangesImmediateSend<T> {
    most_recent_sent_value: Option<T>,
}

impl<T> Default for OnlyChangesImmediateSend<T> {
    fn default() -> Self {
        Self {
            most_recent_sent_value: None,
        }
    }
}

impl<T: Clone> SendBehavior for OnlyChangesImmediateSend<T> {}

#[derive(Clone)]
pub struct RateLimitedMostRecentSend<T> {
    /// The increment the rate-limiting will make the next send wait for, for each additional requested send.
    wait_increment: chrono::Duration,
    /// The maximum duration after a requested send where the next send will be pushed.
    wait_max: chrono::Duration,
    duration_to_count_over: chrono::Duration,
    rate_limit_unique_data: sync::Arc<sync::Mutex<RateLimitUniqueData<T>>>,
}

struct RateLimitUniqueData<T> {
    log_name: String,
    recently_received_sends: Vec<chrono::DateTime<chrono::Utc>>,
    /// Newest value which should be sent on the next push. If no new values arrived since the last push, then this should be empty.
    next_value_to_be_pushed: Option<T>,
    scheduled_sender: Option<tokio::task::JoinHandle<()>>,
    most_recent_sent_value: Option<T>,
}

impl<T> RateLimitedMostRecentSend<T> {
    #[must_use]
    pub fn new(
        log_name: String,
        wait_increment: chrono::Duration,
        wait_max: chrono::Duration,
        duration_to_count_over: chrono::Duration,
    ) -> Self {
        Self {
            wait_increment,
            wait_max,
            duration_to_count_over,
            rate_limit_unique_data: sync::Arc::new(sync::Mutex::new(RateLimitUniqueData {
                log_name,
                recently_received_sends: vec![],
                next_value_to_be_pushed: None,
                scheduled_sender: None,
                most_recent_sent_value: None,
            })),
        }
    }
}

impl<T: Clone> SendBehavior for RateLimitedMostRecentSend<T> {}

#[must_use]
pub fn fire_and_forget_channel<T: Clone, C: SendBehavior + Default>() -> FireAndForgetChannel<T, C>
{
    FireAndForgetChannel {
        inner: broadcast::channel(1).0,
        send_behavior: C::default(),
    }
}

pub fn fire_and_forget_channel_with<T: Clone, C: SendBehavior>(
    send_behavior: C,
) -> FireAndForgetChannel<T, C> {
    FireAndForgetChannel {
        inner: broadcast::channel(1).0,
        send_behavior,
    }
}

impl<T: Clone, C: SendBehavior> FireAndForgetChannel<T, C> {
    #[must_use]
    pub fn subscribe(&self) -> LatestReceiver<T> {
        LatestReceiver {
            inner: self.inner.subscribe(),
        }
    }
}

impl<T: Clone> FireAndForgetChannel<T, DefaultAlwaysImmediateSend<T>> {
    /// Sends the value through the channel.
    /// There is no guarantee that any receivers are subscribed and whether they receive the message.
    /// Returns the number of receivers at the moment of the sending. May be 0.
    pub fn send(&mut self, new_value: T) -> usize {
        self.send_behavior.most_recent_sent_value = Some(new_value.clone());
        self.inner.send(new_value).unwrap_or_default()
    }

    pub fn most_recent_sent_value(&self) -> Option<T> {
        self.send_behavior.most_recent_sent_value.clone()
    }
}

impl<T: Clone + Eq> FireAndForgetChannel<T, OnlyChangesImmediateSend<T>> {
    /// Sends the value through the channel.
    /// There is no guarantee that any receivers are subscribed and whether they receive the message.
    /// Returns the number of receivers at the moment of the sending. May be 0.
    ///
    /// In addition, changes are sent only when the new value is different from the old one.
    /// Returns None, if no update was sent.
    pub fn send(&mut self, new_value: T) -> Option<usize> {
        let is_different = self
            .send_behavior
            .most_recent_sent_value
            .as_ref()
            .map_or_else(|| true, |old| *old != new_value);
        if is_different {
            self.send_behavior.most_recent_sent_value = Some(new_value.clone());
            let receivers = self.inner.send(new_value).unwrap_or_default();
            Some(receivers)
        } else {
            None
        }
    }

    pub fn most_recent_sent_value(&self) -> Option<T> {
        self.send_behavior.most_recent_sent_value.clone()
    }
}

// 'static + Send is fine for owned-structs, as we don't want to send anything else through the channels
impl<T: Clone + 'static + Send> FireAndForgetChannel<T, RateLimitedMostRecentSend<T>> {
    /// Sends the value through the channel.
    /// There is no guarantee that any receivers are subscribed and whether they receive the message.
    ///
    /// In addition, it rate-limits. It will wait as configured in the `send_behavior` field.
    /// This means, that invocations of *send* won't immediately send the value.
    /// It will wait at least *`wait_min`* duration and at-max *`wait_max`* duration.
    /// The more frequent *send* was invoked in the last *`duration_to_count_over`* duration,
    /// the longer it will wait until it actually sends the most-recent value.
    ///
    /// This function has no return value, because the number of receivers is determind in future when the send actually occurs.
    #[allow(clippy::significant_drop_tightening)]
    pub fn send(&mut self, newest_value_requested_to_send: T) {
        let mut locked_rate_limit_data = match self.send_behavior.rate_limit_unique_data.lock() {
            Ok(x) => x,
            Err(e) => {
                log::error!("This should't happen! RateLimitedMostRecentSend -> send. {e}");
                return;
            }
        };

        let log_name = locked_rate_limit_data.log_name.clone();

        let current_send_time = clock::now();
        log::info!(
            "{log_name}: FireAndForgetChannel with RateLimitedMostRecentSend: Received send at {current_send_time}",
        );

        // note, that the current send happened.
        locked_rate_limit_data
            .recently_received_sends
            .push(current_send_time);

        // remove outdated sends
        locked_rate_limit_data
            .recently_received_sends
            .retain_mut(|t| {
                current_send_time.signed_duration_since(t)
                    < self.send_behavior.duration_to_count_over
            });

        // compute duration to wait for next send based on number of received send requests
        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_possible_wrap)]
        let count = locked_rate_limit_data.recently_received_sends.len() as i32;
        let duration_to_wait = self
            .send_behavior
            .wait_max
            .min(self.send_behavior.wait_increment * count.pow(2));

        locked_rate_limit_data
            .next_value_to_be_pushed
            .replace(newest_value_requested_to_send);

        // if no existing task is scheduled, then we schedule a task in duration_to_wait in future
        // to then send the most recent value.
        // if a task is already scheduled, then we don't do anything.

        let task_already_running = locked_rate_limit_data
            .scheduled_sender
            .as_ref()
            .map_or_else(|| false, |t| !t.is_finished());

        if task_already_running {
            return;
        }

        let self2 = self.clone();
        let task = tokio::task::spawn(async move {
            // send the most recent value, whatever it might be, after waiting duration_to_wait.
            let duration_to_wait = duration_to_wait
                .to_std()
                .unwrap_or_else(|_| time::Duration::from_secs(1)); // this error shouldn't happen
            log::debug!(
                "{log_name}: FireAndForgetChannel with RateLimitedMostRecentSend: Waiting to push send after '{duration_to_wait:?}' (count = {count})"
            );
            tokio::time::sleep(duration_to_wait).await;
            // now self2 might have changed and might have a new most recent value. whatever it is, we will send it now.
            match self2.send_behavior.rate_limit_unique_data.lock() {
                Ok(mut x) => {
                    if let Some(value_to_be_pushed) = x.next_value_to_be_pushed.clone() {
                        x.most_recent_sent_value = Some(value_to_be_pushed.clone());
                        x.scheduled_sender.take();
                        let _ = self2.inner.send(value_to_be_pushed).unwrap_or_default();
                        log::debug!(
                            "{log_name}: FireAndForgetChannel with RateLimitedMostRecentSend: New value sent."
                        );
                    }
                }
                Err(e) => {
                    log::error!(
                        "{log_name}: This should't happen! RateLimitedMostRecentSend -> send (2). {e}"
                    );
                }
            }
        });

        locked_rate_limit_data.scheduled_sender.replace(task);
    }

    #[must_use]
    pub fn most_recent_sent_value(&self) -> Option<T> {
        match self.send_behavior.rate_limit_unique_data.lock() {
            Ok(x) => x.most_recent_sent_value.clone(),
            Err(e) => {
                log::error!(
                    "This should't happen! RateLimitedMostRecentSend -> most_recent_sent_value. {e}"
                );
                None
            }
        }
    }
}

/// Variation of the `tokio::sync::broadcast` receiver, where we don't care if we miss out
/// on intermediate messages.
pub struct LatestReceiver<T> {
    inner: broadcast::Receiver<T>,
}

impl<T: Clone> LatestReceiver<T> {
    /// Await for the next message. Skips outdated messages since the previous await.
    /// Returns None, if the sender is closed and will never return.
    pub async fn recv(&mut self) -> Option<T> {
        loop {
            match self.inner.recv().await {
                Ok(value) => return Some(value),
                Err(broadcast::error::RecvError::Closed) => return None,
                Err(broadcast::error::RecvError::Lagged(_)) => (),
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ServerToBridgeSseMessage {
    // If None, then remove old actvity and show nothing.
    pub discord_rich_presence: Option<platforms::DiscordRichPresence>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BridgeToServerSseMessage {
    pub discord_updater_status: updater::UpdaterStatus,
}
