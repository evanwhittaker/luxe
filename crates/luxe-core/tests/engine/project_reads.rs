//! Project reads — what the preview's FILE tab may read, as the engine answers it.

use std::sync::Arc;

use luxe_core::engine::{Action, Engine, EngineConfig};
use luxe_core::event::Event;
use luxe_core::external_work::ExternalWork;
use luxe_core::project_files::{FileReadRequestId, ProjectFileFailure};
use luxe_core::project_search::{SearchFailure, SearchOptions, SearchRequestId, SearchResults};
use luxe_core::session::SessionStore;

use crate::fixtures::*;

/// A file this session changed outside the project reads back like a project file, and a file
/// beside it that the session never touched is still refused.
#[tokio::test]
async fn the_preview_reads_what_the_session_changed_outside_the_project() {
    let (project, elsewhere) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let changed = elsewhere.path().join("notes.md");
    let work = ExternalWork::new();
    work.observe(&changed);
    std::fs::write(&changed, "# notes\n").unwrap();
    std::fs::write(elsewhere.path().join("secret.md"), "hush\n").unwrap();
    let (actions, mut events) = Engine::spawn(engine_with(project.path(), work));

    let read = read_back(&actions, &mut events, &changed).await;
    assert_eq!(read.map(|file| file.text), Ok("# notes\n".to_owned()));
    let refused = read_back(&actions, &mut events, &elsewhere.path().join("secret.md")).await;
    assert_eq!(refused, Err(ProjectFileFailure::OutsideProject));
    let _ = actions.send(Action::Close).await;
}

/// Find, asked through the engine, answers with the project's matching lines.
#[tokio::test]
async fn a_project_search_answers_with_the_matching_lines() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("main.rs"), "fn main() {}\nfn other() {}\n").unwrap();
    let (actions, mut events) = Engine::spawn(engine_with(project.path(), ExternalWork::new()));

    let found = search_back(&actions, &mut events, "main").await.expect("an answer");
    let rows: Vec<(String, usize)> =
        found.hits.into_iter().map(|hit| (hit.path, hit.line)).collect();
    assert_eq!(rows, [("main.rs".to_owned(), 1)]);
    let _ = actions.send(Action::Close).await;
}

/// Every file as one scope, `main`, over lines 0 to 9.
struct MainScope;

impl luxe_core::hooks::file_changed::OutlineSource for MainScope {
    fn functions(
        &self,
        _: &std::path::Path,
        _: &str,
    ) -> Option<Vec<luxe_core::hooks::file_changed::FunctionSize>> {
        None
    }

    fn scopes(
        &self,
        _: &std::path::Path,
        _: &str,
    ) -> Option<Vec<luxe_core::hooks::file_changed::Scope>> {
        let main = luxe_core::hooks::file_changed::Scope {
            name: "main".into(),
            start_line: 0,
            end_line: 9,
        };
        Some(vec![main])
    }
}

/// The break this catches: the engine dropping the outline it was given, so Find's rows never
/// name the function a hit sits in.
#[tokio::test]
async fn a_project_search_names_the_scope_of_each_hit() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("main.rs"), "fn main() {\n    needle();\n}\n").unwrap();
    let config = EngineConfig {
        outline: Some(Arc::new(MainScope)),
        ..engine_with(project.path(), ExternalWork::new())
    };
    let (actions, mut events) = Engine::spawn(config);

    let found = search_back(&actions, &mut events, "needle").await.expect("an answer");
    let scopes: Vec<Option<String>> = found.hits.into_iter().map(|hit| hit.scope).collect();
    assert_eq!(scopes, [Some("main".to_owned())]);
    let _ = actions.send(Action::Close).await;
}

/// What the engine answers a search for `query` with.
async fn search_back(
    actions: &tokio::sync::mpsc::Sender<Action>,
    events: &mut tokio::sync::mpsc::Receiver<Event>,
    query: &str,
) -> Result<SearchResults, SearchFailure> {
    let request_id = SearchRequestId::new();
    let options = SearchOptions::default();
    let asked =
        Action::SearchProject { request_id: request_id.clone(), query: query.into(), options };
    actions.send(asked).await.unwrap();
    first(events, "the search", |event| match event {
        Event::ProjectSearched(answer) if answer.request_id == request_id => {
            Some(Ok(answer.results))
        }
        Event::ProjectSearchFailed(failed) if failed.request_id == request_id => {
            Some(Err(failed.error))
        }
        _ => None,
    })
    .await
}

/// An engine on `project` that knows the session's outside work is `work`.
fn engine_with(project: &std::path::Path, work: ExternalWork) -> EngineConfig {
    let mut store = project_store(project);
    let session_id = store.create(&luxe_core::paths::display(project)).unwrap();
    let provider = Arc::new(RemediationProvider { reply: String::new() });
    EngineConfig {
        guided_review_project_root: Some(project.to_path_buf()),
        external_work: Some(work),
        ..base_config(store, session_id, provider)
    }
}

/// What the engine answers a FILE-tab read of `path` with.
async fn read_back(
    actions: &tokio::sync::mpsc::Sender<Action>,
    events: &mut tokio::sync::mpsc::Receiver<Event>,
    path: &std::path::Path,
) -> Result<luxe_core::project_files::ProjectFile, ProjectFileFailure> {
    let request_id = FileReadRequestId::new();
    let path = luxe_core::paths::display(path);
    let asked = Action::ReadProjectFile { request_id: request_id.clone(), path };
    actions.send(asked).await.unwrap();
    first(events, "the file read", |event| match event {
        Event::ProjectFileRead { request_id: id, file } if id == request_id => Some(Ok(*file)),
        Event::ProjectFileReadFailed { request_id: id, error, .. } if id == request_id => {
            Some(Err(error))
        }
        _ => None,
    })
    .await
}
