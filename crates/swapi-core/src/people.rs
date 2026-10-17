//! One serialized owner for query, paging and refresh. Replacing a query drops
//! both its debounce timer and its HTTP future before a result can be published.
use crate::{ApiError, PeoplePage, Person, SwapiClient};
use std::{future::Future, pin::Pin, sync::Arc, time::Duration};
use tokio::{
    runtime::Handle,
    sync::{mpsc, watch},
    task::AbortHandle,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PeopleOperation {
    Initial,
    Refresh,
    More,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PeoplePhase {
    Idle,
    Loading,
    Ready,
    Refreshing,
    LoadingMore,
    Failed {
        operation: PeopleOperation,
        error: ApiError,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PeopleSnapshot {
    pub revision: u64,
    pub query: String,
    pub people: Vec<Person>,
    pub total: u32,
    pub next_page: Option<u32>,
    pub phase: PeoplePhase,
}
impl Default for PeopleSnapshot {
    fn default() -> Self {
        Self {
            revision: 0,
            query: String::new(),
            people: vec![],
            total: 0,
            next_page: None,
            phase: PeoplePhase::Idle,
        }
    }
}

enum Command {
    Search(String),
    Refresh,
    More,
    Retry,
}
type Request = Pin<Box<dyn Future<Output = Result<PeoplePage, ApiError>> + Send>>;
type Loader = Arc<dyn Fn(String, u32) -> Request + Send + Sync>;

pub struct PeopleStore {
    commands: mpsc::UnboundedSender<Command>,
    states: watch::Receiver<PeopleSnapshot>,
    actor: AbortHandle,
}
impl PeopleStore {
    pub fn new(client: SwapiClient, runtime: &Handle) -> Self {
        Self::start(
            Arc::new(move |query, page| {
                let client = client.clone();
                Box::pin(async move { client.people(&query, page).await })
            }),
            runtime,
        )
    }

    fn start(load: Loader, runtime: &Handle) -> Self {
        let (commands, mut inbox) = mpsc::unbounded_channel();
        let (publisher, states) = watch::channel(PeopleSnapshot::default());
        let actor = runtime.spawn(async move {
            let mut snapshot = PeopleSnapshot::default();
            let mut pending: Option<(PeopleOperation, Request)> = None;
            loop {
                tokio::select! {
                    biased;
                    command = inbox.recv() => {
                        let (operation, debounce) = match command {
                            Some(Command::Search(query)) => {
                                let query = query.trim().to_owned();
                                if query == snapshot.query && snapshot.phase != PeoplePhase::Idle { continue; }
                                drop(pending.take());
                                snapshot = PeopleSnapshot { revision: snapshot.revision, query, ..Default::default() };
                                (PeopleOperation::Initial, true)
                            }
                            Some(Command::Refresh) if pending.is_none() => (if snapshot.people.is_empty() { PeopleOperation::Initial } else { PeopleOperation::Refresh }, false),
                            Some(Command::More) if snapshot.phase == PeoplePhase::Ready && snapshot.next_page.is_some() => (PeopleOperation::More, false),
                            Some(Command::Retry) => match &snapshot.phase {
                                PeoplePhase::Failed { operation, error } if error.is_retryable() => (*operation, false),
                                _ => continue,
                            },
                            None => break,
                            _ => continue,
                        };
                        let page = if operation == PeopleOperation::More { snapshot.next_page.expect("more requires a next page") } else { 1 };
                        let query = snapshot.query.clone();
                        let loader = load.clone();
                        pending = Some((operation, Box::pin(async move {
                            if debounce { tokio::time::sleep(Duration::from_millis(300)).await; }
                            loader(query, page).await
                        })));
                        snapshot.phase = match operation {
                            PeopleOperation::Initial => PeoplePhase::Loading,
                            PeopleOperation::Refresh => PeoplePhase::Refreshing,
                            PeopleOperation::More => PeoplePhase::LoadingMore,
                        };
                    }
                    result = async { pending.as_mut().expect("guarded request").1.as_mut().await }, if pending.is_some() => {
                        let operation = pending.take().expect("completed request").0;
                        match result {
                            Ok(page) => {
                                if operation != PeopleOperation::More { snapshot.people.clear(); }
                                let mut seen: std::collections::HashSet<_> = snapshot.people.iter().map(|p| p.id).collect();
                                snapshot.people.extend(page.people.into_iter().filter(|p| seen.insert(p.id)));
                                snapshot.total = page.total;
                                snapshot.next_page = page.next_page;
                                snapshot.phase = PeoplePhase::Ready;
                            }
                            Err(error) => snapshot.phase = PeoplePhase::Failed { operation, error },
                        }
                    }
                }
                snapshot.revision += 1;
                publisher.send_replace(snapshot.clone());
            }
        }).abort_handle();
        Self {
            commands,
            states,
            actor,
        }
    }
    pub fn observe(&self) -> watch::Receiver<PeopleSnapshot> {
        self.states.clone()
    }
    pub fn search(&self, query: String) {
        let _ = self.commands.send(Command::Search(query));
    }
    pub fn refresh(&self) {
        let _ = self.commands.send(Command::Refresh);
    }
    pub fn load_more(&self) {
        let _ = self.commands.send(Command::More);
    }
    pub fn retry(&self) {
        let _ = self.commands.send(Command::Retry);
    }
    pub fn close(&self) {
        self.actor.abort();
    }
}
impl Drop for PeopleStore {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests;
