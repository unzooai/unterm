use super::{launch, runtime, session_runtime};
use crate::{CreateSessionRequest, SessionSnapshot, SplitSessionRequest};
use anyhow::Result;

pub(super) fn create(request: CreateSessionRequest) -> Result<SessionSnapshot> {
    let launch_env_keys = request.env.iter().map(|(key, _)| key.clone()).collect();
    let launch_context = launch::launch_context(&request.env, &request.launch_policy);
    let (command, cwd) = launch::prepare_command(request.command, request.command_dir, request.env);
    let id = runtime::next_session_id();

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
    runtime::insert_created(session);
    Ok(snapshot)
}

pub(super) fn split(request: SplitSessionRequest) -> Result<SessionSnapshot> {
    let source = runtime::clone_session_base(request.source_pane_id)?;

    let mut command = portable_pty::CommandBuilder::new_default_prog();
    if let Some(cwd) = request.command_dir.or(source.shell.cwd) {
        command.cwd(cwd);
    }
    let cwd = launch::command_cwd(&command, None);
    let launch_env_keys = Vec::new();

    let id = runtime::next_session_id();

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
    runtime::insert_created(session);
    Ok(snapshot)
}
