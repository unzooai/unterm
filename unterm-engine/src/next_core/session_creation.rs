use super::{launch, session_registry, session_runtime, session_snapshots, state};
use crate::{CreateSessionRequest, SessionSnapshot, SplitSessionRequest};
use anyhow::Result;

pub(super) fn create(request: CreateSessionRequest) -> Result<SessionSnapshot> {
    let launch_env_keys = request.env.iter().map(|(key, _)| key.clone()).collect();
    let launch_context = launch::launch_context(&request.env, &request.launch_policy);
    let (command, cwd) = launch::prepare_command(request.command, request.command_dir, request.env);
    let mut state_guard = state().write();
    let id = session_registry::next_session_id(&mut state_guard);
    drop(state_guard);

    let mut session = session_runtime::spawn(
        id,
        format!("next-core:{id}"),
        request.cols,
        request.rows,
        command,
        cwd,
        launch_env_keys,
    )?;

    session.snapshot.shell.launch_context = launch_context;
    let snapshot = session.snapshot.clone();
    let mut state_guard = state().write();
    session_registry::insert_created(&mut state_guard, session);
    Ok(snapshot)
}

pub(super) fn split(request: SplitSessionRequest) -> Result<SessionSnapshot> {
    let state_guard = state().read();
    let source = session_snapshots::clone_base(&state_guard, request.source_pane_id);
    drop(state_guard);
    let source = source?;

    let mut command = portable_pty::CommandBuilder::new_default_prog();
    if let Some(cwd) = request.command_dir.or(source.shell.cwd) {
        command.cwd(cwd);
    }
    let cwd = launch::command_cwd(&command, None);
    let launch_env_keys = Vec::new();

    let mut state_guard = state().write();
    let id = session_registry::next_session_id(&mut state_guard);
    drop(state_guard);

    let session = session_runtime::spawn(
        id,
        format!("next-core:{id}"),
        source.cols,
        source.rows,
        command,
        cwd,
        launch_env_keys,
    )?;

    let snapshot = session.snapshot.clone();
    let mut state_guard = state().write();
    session_registry::insert_created(&mut state_guard, session);
    Ok(snapshot)
}
