use crate::{
    LaunchContextSnapshot, LaunchEnvBinding, LaunchEnvSource, LaunchPolicyDecision,
    LaunchPolicyDecisionSnapshot, LaunchPolicySnapshot,
};

pub(super) fn command_label(command: &portable_pty::CommandBuilder) -> String {
    if command.is_default_prog() {
        command.get_shell()
    } else {
        command
            .get_argv()
            .first()
            .and_then(|arg| arg.to_str())
            .unwrap_or("command")
            .to_string()
    }
}

pub(super) fn shell_type(label: &str) -> String {
    let label = label.to_lowercase();
    if label.contains("powershell") || label.contains("pwsh") {
        "powershell"
    } else if label.contains("cmd") {
        "cmd"
    } else if label.contains("bash") {
        "bash"
    } else if label.contains("zsh") {
        "zsh"
    } else if label.contains("fish") {
        "fish"
    } else {
        "unknown"
    }
    .to_string()
}

pub(super) fn command_cwd(
    command: &portable_pty::CommandBuilder,
    fallback: Option<String>,
) -> Option<String> {
    command
        .get_cwd()
        .and_then(|cwd| cwd.to_str().map(|cwd| cwd.to_string()))
        .or(fallback)
}

pub(super) fn launch_context(
    env: &[(String, String)],
    launch_policy: &LaunchPolicySnapshot,
) -> LaunchContextSnapshot {
    let proxy_env_keys = proxy_env_keys(env);
    let mut policy = if launch_policy.env.is_empty()
        && launch_policy.profile.is_none()
        && launch_policy.proxy_env_keys.is_empty()
    {
        infer_launch_policy(env)
    } else {
        launch_policy.clone()
    };
    if policy.profile.is_none() {
        policy.profile = profile_env(env);
    }
    if policy.proxy_env_keys.is_empty() {
        policy.proxy_env_keys = proxy_env_keys.clone();
    }
    complete_launch_policy_decisions(&mut policy);

    LaunchContextSnapshot {
        profile: policy.profile.clone().or_else(|| profile_env(env)),
        proxy_env_keys,
        env_key_count: env.len(),
        policy,
    }
}

pub(super) fn prepare_command(
    command: Option<portable_pty::CommandBuilder>,
    command_dir: Option<String>,
    env: Vec<(String, String)>,
) -> (portable_pty::CommandBuilder, Option<String>) {
    let mut command = command.unwrap_or_else(portable_pty::CommandBuilder::new_default_prog);
    if let Some(command_dir) = command_dir {
        if command.get_cwd().is_none() {
            command.cwd(&command_dir);
        }
    }
    for (key, value) in env {
        command.env(key, value);
    }
    let cwd = command_cwd(&command, None);
    (command, cwd)
}

fn complete_launch_policy_decisions(policy: &mut LaunchPolicySnapshot) {
    let default_decision = LaunchPolicyDecisionSnapshot::default();
    if policy.domain == default_decision {
        policy.domain = LaunchPolicyDecisionSnapshot::new(
            LaunchPolicyDecision::NotRequested,
            false,
            "next-core currently launches local-domain sessions only",
        );
    }
    if policy.privilege == default_decision {
        policy.privilege = LaunchPolicyDecisionSnapshot::new(
            LaunchPolicyDecision::NotRequested,
            false,
            "elevation is host-owned and not applied by next-core launch",
        );
    }
    if policy.proxy_rotation == default_decision {
        policy.proxy_rotation = if policy.proxy_env_keys.is_empty() {
            LaunchPolicyDecisionSnapshot::new(
                LaunchPolicyDecision::NotRequested,
                false,
                "no proxy env keys were provided",
            )
        } else {
            LaunchPolicyDecisionSnapshot::new(
                LaunchPolicyDecision::Deferred,
                false,
                "proxy env is applied; proxy rotation remains product-managed",
            )
        };
    }
    if policy.restart == default_decision {
        policy.restart = LaunchPolicyDecisionSnapshot::new(
            LaunchPolicyDecision::NotRequested,
            false,
            "restart policy is not applied during next-core session launch",
        );
    }
}

fn infer_launch_policy(env: &[(String, String)]) -> LaunchPolicySnapshot {
    let mut proxy_env_keys = Vec::new();
    let bindings = env
        .iter()
        .map(|(key, _)| {
            let upper = key.to_ascii_uppercase();
            let source = if key.eq_ignore_ascii_case("UNTERM_PROFILE") {
                LaunchEnvSource::Profile
            } else if matches!(
                upper.as_str(),
                "HTTP_PROXY" | "HTTPS_PROXY" | "ALL_PROXY" | "NO_PROXY"
            ) {
                proxy_env_keys.push(key.clone());
                LaunchEnvSource::Proxy
            } else {
                LaunchEnvSource::Explicit
            };
            LaunchEnvBinding {
                key: key.clone(),
                source,
            }
        })
        .collect();
    proxy_env_keys.sort();
    proxy_env_keys.dedup();
    LaunchPolicySnapshot {
        profile: profile_env(env),
        proxy_env_keys,
        env: bindings,
        ..Default::default()
    }
}

fn profile_env(env: &[(String, String)]) -> Option<String> {
    env.iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("UNTERM_PROFILE"))
        .map(|(_, value)| value.clone())
        .filter(|value| !value.trim().is_empty())
}

fn proxy_env_keys(env: &[(String, String)]) -> Vec<String> {
    let mut proxy_env_keys = env
        .iter()
        .filter_map(|(key, _)| {
            let upper = key.to_ascii_uppercase();
            matches!(
                upper.as_str(),
                "HTTP_PROXY" | "HTTPS_PROXY" | "ALL_PROXY" | "NO_PROXY"
            )
            .then(|| key.clone())
        })
        .collect::<Vec<_>>();
    proxy_env_keys.sort();
    proxy_env_keys.dedup();
    proxy_env_keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_command_applies_env_and_cwd() {
        let expected_cwd = std::env::current_dir()
            .expect("current dir")
            .display()
            .to_string();
        let (command, cwd) = prepare_command(
            None,
            Some(expected_cwd.clone()),
            vec![
                ("UNTERM_PROFILE".to_string(), "work-acme".to_string()),
                (
                    "HTTPS_PROXY".to_string(),
                    "http://127.0.0.1:7890".to_string(),
                ),
            ],
        );

        assert!(command.is_default_prog());
        assert_eq!(cwd.as_deref(), Some(expected_cwd.as_str()));
        assert_eq!(
            command
                .get_env("UNTERM_PROFILE")
                .and_then(|value| value.to_str()),
            Some("work-acme")
        );
        assert_eq!(
            command
                .get_env("HTTPS_PROXY")
                .and_then(|value| value.to_str()),
            Some("http://127.0.0.1:7890")
        );
    }

    #[test]
    fn launch_context_summarizes_profile_and_proxy_env_without_values() {
        let env = [
            ("GITHUB_TOKEN".to_string(), "secret-token".to_string()),
            ("UNTERM_PROFILE".to_string(), "work-acme".to_string()),
            (
                "HTTPS_PROXY".to_string(),
                "http://127.0.0.1:7890".to_string(),
            ),
            ("NO_PROXY".to_string(), "localhost".to_string()),
        ];
        let context = launch_context(&env, &Default::default());

        assert_eq!(context.profile.as_deref(), Some("work-acme"));
        assert_eq!(context.proxy_env_keys, vec!["HTTPS_PROXY", "NO_PROXY"]);
        assert_eq!(context.env_key_count, 4);
        assert_eq!(context.policy.profile.as_deref(), Some("work-acme"));
        assert_eq!(
            context.policy.domain.decision,
            LaunchPolicyDecision::NotRequested
        );
        assert_eq!(context.policy.domain.supported, false);
        assert_eq!(
            context.policy.privilege.decision,
            LaunchPolicyDecision::NotRequested
        );
        assert_eq!(
            context.policy.proxy_rotation.decision,
            LaunchPolicyDecision::Deferred
        );
        assert_eq!(
            context.policy.restart.decision,
            LaunchPolicyDecision::NotRequested
        );
        assert_eq!(
            context
                .policy
                .env
                .iter()
                .map(|binding| (binding.key.as_str(), binding.source))
                .collect::<Vec<_>>(),
            vec![
                ("GITHUB_TOKEN", LaunchEnvSource::Explicit),
                ("UNTERM_PROFILE", LaunchEnvSource::Profile),
                ("HTTPS_PROXY", LaunchEnvSource::Proxy),
                ("NO_PROXY", LaunchEnvSource::Proxy)
            ]
        );
    }
}
