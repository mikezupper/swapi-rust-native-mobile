//! Shared saved-library commands and cancellable, page-by-page downloads.
use crate::{
    ApiError, ArchiveCache, CatalogRecord, CollectionInfo, Loaded, Page, ResourceKind, ResourceRef,
    SavedRecord, SwapiClient,
};
use std::{collections::HashSet, future::Future, pin::Pin, sync::Arc};
use tokio::{
    runtime::Handle,
    sync::{mpsc, watch},
    task::AbortHandle,
};

#[derive(Clone, Debug, PartialEq)]
pub enum DownloadState {
    Idle,
    Running {
        kind: ResourceKind,
        saved: u32,
        total: u32,
    },
    Complete {
        kind: ResourceKind,
        count: u32,
    },
    Failed {
        kind: ResourceKind,
        error: ApiError,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct ArchiveSnapshot {
    pub revision: u64,
    pub query: String,
    pub favorites_only: bool,
    pub offline: bool,
    pub items: Vec<SavedRecord>,
    pub favorites: Vec<ResourceRef>,
    pub collections: Vec<CollectionInfo>,
    pub download: DownloadState,
    pub error: Option<ApiError>,
    pub exploration: crate::Exploration,
}
impl Default for ArchiveSnapshot {
    fn default() -> Self {
        Self {
            revision: 0,
            query: String::new(),
            favorites_only: false,
            offline: false,
            items: vec![],
            favorites: vec![],
            collections: vec![],
            download: DownloadState::Idle,
            error: None,
            exploration: Default::default(),
        }
    }
}
enum Command {
    Discover,
    Visit(ResourceKind, u32),
    SelectShip(u32, bool),
    Search(String),
    FavoritesOnly(bool),
    Offline(bool),
    Favorite(ResourceKind, u32, bool),
    Download(ResourceKind),
    CancelDownload,
    ClearCache,
}
type DiscoveryRequest =
    Pin<Box<dyn Future<Output = Result<Loaded<CatalogRecord>, ApiError>> + Send>>;
type Request = Pin<Box<dyn Future<Output = Result<Loaded<Page<CatalogRecord>>, ApiError>> + Send>>;
struct Download {
    kind: ResourceKind,
    page: u32,
    force: bool,
    seen: HashSet<u32>,
    total: Option<u32>,
    pending: Request,
}
fn request(client: SwapiClient, kind: ResourceKind, page: u32, force: bool) -> Request {
    Box::pin(async move { client.resources_loaded(kind, "", page, force).await })
}

pub struct ArchiveStore {
    commands: mpsc::UnboundedSender<Command>,
    states: watch::Receiver<ArchiveSnapshot>,
    actor: AbortHandle,
}
impl ArchiveStore {
    pub fn new(client: SwapiClient, cache: Arc<ArchiveCache>, runtime: &Handle) -> Self {
        let (commands, mut inbox) = mpsc::unbounded_channel();
        let (publisher, states) = watch::channel(ArchiveSnapshot::default());
        let actor = runtime.spawn(async move {
            let mut snapshot = ArchiveSnapshot::default();
            let mut changes = cache.observe();
            let mut download: Option<Download> = None;
            let mut discovery: Option<DiscoveryRequest> = None;
            loop {
                snapshot.offline = cache.is_offline();
                match cache.records(&snapshot.query, snapshot.favorites_only) {
                    Ok(items) => snapshot.items = items,
                    Err(error) => snapshot.error = Some(error),
                }
                match cache.records("", true) {
                    Ok(items) => snapshot.favorites = items.into_iter().map(|s| ResourceRef { kind: s.record.kind, id: s.record.id }).collect(),
                    Err(error) => snapshot.error = Some(error),
                }
                match cache.collections() {
                    Ok(collections) => snapshot.collections = collections,
                    Err(error) => snapshot.error = Some(error),
                }
                match cache.exploration(crate::cache::now() / 86400) {
                    Ok(value) => snapshot.exploration = value,
                    Err(error) => snapshot.error = Some(error),
                }
                snapshot.revision += 1;
                publisher.send_replace(snapshot.clone());
                tokio::select! {
                    biased;
                    command = inbox.recv() => {
                        snapshot.error = None;
                        let result = match command {
                            Some(Command::Discover) => {
                                if !cache.is_offline() && discovery.is_none() && snapshot.exploration.featured.is_none() {
                                    let client = client.clone();
                                    discovery = Some(Box::pin(async move { client.resource_loaded(ResourceKind::Planet, 1, false).await }));
                                }
                                Ok(())
                            },
                            Some(Command::Visit(kind, id)) => cache.visit(kind, id),
                            Some(Command::SelectShip(id, value)) => cache.select_ship(id, value),
                            Some(Command::Search(query)) => { snapshot.query = query; Ok(()) },
                            Some(Command::FavoritesOnly(value)) => { snapshot.favorites_only = value; Ok(()) },
                            Some(Command::Offline(value)) => {
                                if value { discovery = None; download = None; snapshot.download = DownloadState::Idle; }
                                cache.set_offline(value)
                            },
                            Some(Command::Favorite(kind, id, value)) => cache.set_favorite(kind, id, value),
                            Some(Command::ClearCache) => {
                                discovery = None; download = None; snapshot.download = DownloadState::Idle;
                                cache.clear_cache()
                            },
                            Some(Command::CancelDownload) => {
                                download = None; snapshot.download = DownloadState::Idle; Ok(())
                            },
                            Some(Command::Download(kind)) => {
                                if download.is_some() { continue; }
                                if cache.is_offline() {
                                    snapshot.download = DownloadState::Failed { kind, error: ApiError::Network }; continue;
                                }
                                let collection = snapshot.collections.iter().find(|c| c.kind == kind);
                                let force = collection.is_some_and(|c| c.complete);
                                let total = collection.and_then(|c| c.total).unwrap_or(0);
                                match cache.mark_download(kind, total, false) {
                                    Ok(()) => {
                                        download = Some(Download { kind, page: 1, force, seen: HashSet::new(), total: None, pending: request(client.clone(), kind, 1, force) });
                                        snapshot.download = DownloadState::Running { kind, saved: 0, total };
                                        Ok(())
                                    },
                                    Err(error) => { snapshot.download = DownloadState::Failed { kind, error: error.clone() }; Err(error) },
                                }
                            },
                            None => break,
                        };
                        if let Err(error) = result { snapshot.error = Some(error); }
                    }
                    result = async { download.as_mut().expect("guarded download").pending.as_mut().await }, if download.is_some() => {
                        let mut job = download.take().expect("completed request");
                        match result.and_then(|loaded| {
                            if loaded.cache.saving_failed { Err(ApiError::Storage) }
                            else if loaded.cache.refresh_failed { Err(ApiError::Network) }
                            else { Ok(loaded.value) }
                        }) {
                            Ok(page) => {
                                if job.total.is_some_and(|total| total != page.total) {
                                    snapshot.download = DownloadState::Failed { kind: job.kind, error: ApiError::InvalidData };
                                    continue;
                                }
                                job.total = Some(page.total);
                                job.seen.extend(page.items.iter().map(|r| r.id));
                                if let Some(next) = page.next_page {
                                    if next > 1000 || job.seen.len() > page.total as usize {
                                        snapshot.download = DownloadState::Failed { kind: job.kind, error: ApiError::InvalidData };
                                        continue;
                                    }
                                    job.page = next;
                                    job.pending = request(client.clone(), job.kind, next, job.force);
                                    snapshot.download = DownloadState::Running { kind: job.kind, saved: job.seen.len() as u32, total: page.total };
                                    download = Some(job);
                                } else if job.seen.len() == page.total as usize {
                                    snapshot.download = match cache.mark_download(job.kind, page.total, true) {
                                        Ok(()) => DownloadState::Complete { kind: job.kind, count: page.total },
                                        Err(error) => DownloadState::Failed { kind: job.kind, error },
                                    };
                                } else {
                                    snapshot.download = DownloadState::Failed { kind: job.kind, error: ApiError::InvalidData };
                                }
                            },
                            Err(error) => snapshot.download = DownloadState::Failed { kind: job.kind, error },
                        }
                    }
                    result = async { discovery.as_mut().expect("guarded discovery").as_mut().await }, if discovery.is_some() => {
                        discovery = None;
                        if let Err(error) = result { snapshot.error = Some(error); }
                    }
                    changed = changes.changed() => { if changed.is_err() { break; } }
                }
            }
        }).abort_handle();
        Self {
            commands,
            states,
            actor,
        }
    }
    pub fn observe(&self) -> watch::Receiver<ArchiveSnapshot> {
        self.states.clone()
    }
    pub fn discover(&self) {
        let _ = self.commands.send(Command::Discover);
    }
    pub fn visit(&self, kind: ResourceKind, id: u32) {
        let _ = self.commands.send(Command::Visit(kind, id));
    }
    pub fn select_ship(&self, id: u32, value: bool) {
        let _ = self.commands.send(Command::SelectShip(id, value));
    }
    pub fn search(&self, query: String) {
        let _ = self.commands.send(Command::Search(query));
    }
    pub fn favorites_only(&self, value: bool) {
        let _ = self.commands.send(Command::FavoritesOnly(value));
    }
    pub fn offline(&self, value: bool) {
        let _ = self.commands.send(Command::Offline(value));
    }
    pub fn favorite(&self, kind: ResourceKind, id: u32, value: bool) {
        let _ = self.commands.send(Command::Favorite(kind, id, value));
    }
    pub fn download(&self, kind: ResourceKind) {
        let _ = self.commands.send(Command::Download(kind));
    }
    pub fn cancel_download(&self) {
        let _ = self.commands.send(Command::CancelDownload);
    }
    pub fn clear_cache(&self) {
        let _ = self.commands.send(Command::ClearCache);
    }
    pub fn close(&self) {
        self.actor.abort();
    }
}
impl Drop for ArchiveStore {
    fn drop(&mut self) {
        self.close();
    }
}
