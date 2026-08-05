use super::{launch, runtime, session_runtime};
use crate::{CreateSessionRequest, SessionSnapshot, SplitSessionRequest};
use anyhow::Result;

pub(super) fn create(request: CreateSessionRequest) -> Result<SessionSnapshot> {
    let launch_env_keys = request.env.iter().map(|(key, _)| key.clone()).collect();
    let launch_context = launch::launch_context(&request.env, &request.launch_policy);
    let (command, cwd) = launch::prepare_command(request.command, request.command_dir, request.env);
    let id = runtime::next_session_id();
    let title = launch::command_label(&command);

    let mut session = session_runtime::spawn(
        id,
        title,
        request.cols,
        request.rows,
        command,
        cwd,
        launch_env_keys,
        None,
    )?;

    session.snapshot.shell.launch_context = launch_context;
    let snapshot = session.snapshot.clone();
    runtime::insert_created(session);
    Ok(snapshot)
}

pub(super) fn split(request: SplitSessionRequest) -> Result<SessionSnapshot> {
    let source = runtime::clone_session_base(request.source_pane_id)?;
    // Resolve the direction into what an arrangement is actually made
    // of, here rather than in whoever draws it: a front end rebuilding
    // this pane after a restart never saw the request, and two front
    // ends resolving it separately is two chances to disagree.
    let (split_axis, split_ratio) =
        crate::next_core::layout::resolve_split(request.direction, request.size_percent);

    let launch_env_keys = request.env.iter().map(|(key, _)| key.clone()).collect();
    let launch_context = launch::launch_context(&request.env, &request.launch_policy);
    let command_dir = request.command_dir.or(source.shell.cwd);
    let (command, cwd) = launch::prepare_command(request.command, command_dir, request.env);

    let id = runtime::next_session_id();
    let title = launch::command_label(&command);

    let mut session = session_runtime::spawn(
        id,
        title,
        source.cols,
        source.rows,
        command,
        cwd,
        launch_env_keys,
        Some(request.source_pane_id),
    )?;

    session.snapshot.shell.launch_context = launch_context;
    session.snapshot.split_axis = Some(split_axis);
    session.snapshot.split_ratio = Some(split_ratio);
    let snapshot = session.snapshot.clone();
    runtime::insert_created(session);
    Ok(snapshot)
}
