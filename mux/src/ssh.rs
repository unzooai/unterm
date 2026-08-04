use crate::connui::ConnectionUI;
use anyhow::{bail, Context};
use config::{SshBackend, SshDomain};
use termwiz::cell::{unicode_column_width, AttributeChange, Intensity};
use termwiz::lineedit::*;
use termwiz::surface::{Change, LineAttribute};
use wezterm_ssh::{ConfigMap, HostVerificationFailed, Session, SessionEvent};

#[derive(Default)]
struct PasswordPromptHost {
    history: BasicHistory,
    echo: bool,
}
impl LineEditorHost for PasswordPromptHost {
    fn history(&mut self) -> &mut dyn History {
        &mut self.history
    }

    fn highlight_line(&self, line: &str, cursor_position: usize) -> (Vec<OutputElement>, usize) {
        if self.echo {
            (vec![OutputElement::Text(line.to_string())], cursor_position)
        } else {
            // Rewrite the input so that we can obscure the password
            // characters when output to the terminal widget
            let placeholder = "馃攽";
            let grapheme_count = unicode_column_width(line, None);
            let mut output = vec![];
            for _ in 0..grapheme_count {
                output.push(OutputElement::Text(placeholder.to_string()));
            }
            (
                output,
                unicode_column_width(placeholder, None) * cursor_position,
            )
        }
    }
}

pub fn ssh_connect_with_ui(
    ssh_config: wezterm_ssh::ConfigMap,
    ui: &mut ConnectionUI,
) -> anyhow::Result<Session> {
    let cloned_ui = ui.clone();
    cloned_ui.run_and_log_error(move || {
        let remote_address = ssh_config
            .get("hostname")
            .expect("ssh config to always set hostname");
        ui.output_str(&format!("Connecting to {} using SSH\n", remote_address));
        let (session, events) = Session::connect(ssh_config.clone())?;

        while let Ok(event) = smol::block_on(events.recv()) {
            match event {
                SessionEvent::Banner(banner) => {
                    if let Some(banner) = banner {
                        ui.output_str(&format!("{}\n", banner));
                    }
                }
                SessionEvent::HostVerify(verify) => {
                    ui.output_str(&format!("{}\n", verify.message));
                    let ok = if let Ok(line) = ui.input("Enter [y/n]> ") {
                        match line.as_ref() {
                            "y" | "Y" | "yes" | "YES" => true,
                            "n" | "N" | "no" | "NO" | _ => false,
                        }
                    } else {
                        false
                    };
                    smol::block_on(verify.answer(ok)).context("send verify response")?;
                }
                SessionEvent::Authenticate(auth) => {
                    if !auth.username.is_empty() {
                        ui.output_str(&format!("Authentication for {}\n", auth.username));
                    }
                    if !auth.instructions.is_empty() {
                        ui.output_str(&format!("{}\n", auth.instructions));
                    }
                    let mut answers = vec![];
                    for prompt in &auth.prompts {
                        let mut prompt_lines = prompt.prompt.split('\n').collect::<Vec<_>>();
                        let editor_prompt = prompt_lines.pop().unwrap();
                        for line in &prompt_lines {
                            ui.output_str(&format!("{}\n", line));
                        }
                        let res = if prompt.echo {
                            ui.input(editor_prompt)
                        } else {
                            ui.password(editor_prompt)
                        };
                        if let Ok(line) = res {
                            answers.push(line);
                        } else {
                            anyhow::bail!("Authentication was cancelled");
                        }
                    }
                    smol::block_on(auth.answer(answers))?;
                }
                SessionEvent::HostVerificationFailed(failed) => {
                    let message = format_host_verification_for_terminal(failed);
                    ui.output(message);
                    anyhow::bail!("Host key verification failed");
                }
                SessionEvent::Error(err) => {
                    anyhow::bail!("Error: {}", err);
                }
                SessionEvent::Authenticated => return Ok(session),
            }
        }
        bail!("unable to authenticate session");
    })
}

fn format_host_verification_for_terminal(failed: HostVerificationFailed) -> Vec<Change> {
    vec![
        AttributeChange::Intensity(Intensity::Bold).into(),
        LineAttribute::DoubleHeightTopHalfLine.into(),
        Change::Text("REMOTE HOST IDENTIFICATION CHANGED\r\n".to_string()),
        LineAttribute::DoubleHeightBottomHalfLine.into(),
        Change::Text("REMOTE HOST IDENTIFICATION CHANGED\r\n".to_string()),
        Change::Text("SOMEONE MAY BE DOING SOMETHING NASTY!\r\n".to_string()),
        AttributeChange::Intensity(Intensity::Normal).into(),
        Change::Text("\r\nThere are two likely causes for this:\r\n".to_string()),
        Change::Text(
            " 1. Someone is eavesdropping right now (man-in-the-middle attack)\r\n".to_string(),
        ),
        Change::Text(" 2. The host key may have been changed by the administrator\r\n".to_string()),
        Change::Text("\r\n".to_string()),
        AttributeChange::Intensity(Intensity::Bold).into(),
        Change::Text(
            "Please contact your system administrator to discuss how to proceed!\r\n".to_string(),
        ),
        AttributeChange::Intensity(Intensity::Normal).into(),
        Change::Text("\r\n".to_string()),
        match failed.file {
            Some(file) => Change::Text(format!(
                "The host is {}, and its fingerprint is\r\n{}\r\n\
                If the administrator confirms that the key has changed, you can\r\n\
                fix this for yourself by removing the offending entry from\r\n\
                {} and then try connecting again.\r\n",
                failed.remote_address,
                failed.key,
                file.display(),
            )),
            None => Change::Text(format!(
                "The host is {}, and its fingerprint is\r\n{}\r\n",
                failed.remote_address, failed.key
            )),
        },
    ]
}

pub fn ssh_domain_to_ssh_config(ssh_dom: &SshDomain) -> anyhow::Result<ConfigMap> {
    let mut ssh_config = wezterm_ssh::Config::new();
    ssh_config.add_default_config_files();

    let (remote_host_name, port) = {
        let parts: Vec<&str> = ssh_dom.remote_address.split(':').collect();

        if parts.len() == 2 {
            (parts[0], Some(parts[1].parse::<u16>()?))
        } else {
            (ssh_dom.remote_address.as_str(), None)
        }
    };

    let mut ssh_config = ssh_config.for_host(&remote_host_name);
    ssh_config.insert(
        "wezterm_ssh_backend".to_string(),
        match ssh_dom
            .ssh_backend
            .unwrap_or_else(|| config::configuration().ssh_backend)
        {
            SshBackend::Ssh2 => "ssh2",
            SshBackend::LibSsh => "libssh",
        }
        .to_string(),
    );
    for (k, v) in &ssh_dom.ssh_option {
        ssh_config.insert(k.to_string(), v.to_string());
    }

    if let Some(username) = &ssh_dom.username {
        ssh_config.insert("user".to_string(), username.to_string());
    }
    if let Some(port) = port {
        ssh_config.insert("port".to_string(), port.to_string());
    }
    if ssh_dom.no_agent_auth {
        ssh_config.insert("identitiesonly".to_string(), "yes".to_string());
    }
    if let Some("true") = ssh_config.get("wezterm_ssh_verbose").map(|s| s.as_str()) {
        log::info!("Using ssh config: {ssh_config:#?}");
    }
    Ok(ssh_config)
}
