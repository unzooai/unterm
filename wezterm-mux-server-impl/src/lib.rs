use config::ConfigHandle;
use mux::domain::{CoreDomain, Domain};
use mux::Mux;
use portable_pty::cmdbuilder::CommandBuilder;
use std::sync::Arc;
use wezterm_client::domain::{ClientDomain, ClientDomainConfig};

pub mod dispatch;
pub mod local;
pub mod pki;
pub mod sessionhandler;

fn client_domains(config: &config::ConfigHandle) -> Vec<ClientDomainConfig> {
    let mut domains = vec![];
    for unix_dom in &config.unix_domains {
        domains.push(ClientDomainConfig::Unix(unix_dom.clone()));
    }

    for tls_client in &config.tls_clients {
        domains.push(ClientDomainConfig::Tls(tls_client.clone()));
    }
    domains
}

pub fn update_mux_domains(config: &ConfigHandle) -> anyhow::Result<()> {
    update_mux_domains_impl(config, false)
}

pub fn update_mux_domains_for_server(config: &ConfigHandle) -> anyhow::Result<()> {
    update_mux_domains_impl(config, true)
}

fn update_mux_domains_impl(config: &ConfigHandle, is_standalone_mux: bool) -> anyhow::Result<()> {
    let mux = Mux::get();

    for client_config in client_domains(&config) {
        if mux.get_domain_by_name(client_config.name()).is_some() {
            continue;
        }

        let domain: Arc<dyn Domain> = Arc::new(ClientDomain::new(client_config));
        mux.add_domain(&domain);
    }

    // SSH domains are terminal sessions owned by unterm-core.  Even domains
    // that historically requested WezTerm multiplexing use the Core PTY
    // boundary now; this removes the in-process ClientDomain/old mux runtime
    // from the terminal path while retaining the configured host/user/options.
    for ssh_dom in config.ssh_domains().into_iter() {

        if mux.get_domain_by_name(&ssh_dom.name).is_some() {
            continue;
        }

        let domain: Arc<dyn Domain> = Arc::new(CoreDomain::with_command(
            &ssh_dom.name,
            ssh_command(&ssh_dom),
        ));
        mux.add_domain(&domain);
    }

    for wsl_dom in config.wsl_domains() {
        if mux.get_domain_by_name(&wsl_dom.name).is_some() {
            continue;
        }

        let mut command = CommandBuilder::new("wsl.exe");
        if let Some(distribution) = &wsl_dom.distribution {
            command.arg("-d");
            command.arg(distribution);
        }
        if let Some(username) = &wsl_dom.username {
            command.arg("-u");
            command.arg(username);
        }
        command.arg("--");
        if let Some(default_prog) = &wsl_dom.default_prog {
            command.args(default_prog);
        }
        let domain: Arc<dyn Domain> = Arc::new(CoreDomain::with_command(&wsl_dom.name, command));
        mux.add_domain(&domain);
    }

    for exec_dom in &config.exec_domains {
        if mux.get_domain_by_name(&exec_dom.name).is_some() {
            continue;
        }

        let domain: Arc<dyn Domain> = Arc::new(CoreDomain::with_exec_domain(exec_dom.clone()));
        mux.add_domain(&domain);
    }

    for serial in &config.serial_ports {
        if mux.get_domain_by_name(&serial.name).is_some() {
            continue;
        }

        let port = serial.port.as_ref().unwrap_or(&serial.name).clone();
        let domain: Arc<dyn Domain> =
            Arc::new(CoreDomain::with_serial(&serial.name, port, serial.baud));
        mux.add_domain(&domain);
    }

    if is_standalone_mux {
        if let Some(name) = &config.default_mux_server_domain {
            if let Some(dom) = mux.get_domain_by_name(name) {
                if dom.is::<ClientDomain>() {
                    anyhow::bail!("default_mux_server_domain cannot be set to a client domain!");
                }
                mux.set_default_domain(&dom);
            }
        }
    } else {
        if let Some(name) = &config.default_domain {
            if let Some(dom) = mux.get_domain_by_name(name) {
                mux.set_default_domain(&dom);
            }
        }
    }

    Ok(())
}

fn ssh_command(domain: &config::SshDomain) -> CommandBuilder {
    let mut command = CommandBuilder::new("ssh");
    command.arg("-t");
    for (key, value) in &domain.ssh_option {
        command.arg("-o");
        command.arg(format!("{key}={value}"));
    }
    if domain.no_agent_auth {
        command.arg("-o");
        command.arg("IdentitiesOnly=yes");
    }
    let (host, port) = domain
        .remote_address
        .rsplit_once(':')
        .and_then(|(host, port)| port.parse::<u16>().ok().map(|port| (host, Some(port))))
        .unwrap_or((domain.remote_address.as_str(), None));
    if let Some(port) = port {
        command.arg("-p");
        command.arg(port.to_string());
    }
    command.arg(match &domain.username {
        Some(user) => format!("{user}@{host}"),
        None => host.to_string(),
    });
    if let Some(default_prog) = &domain.default_prog {
        command.args(default_prog);
    }
    command
}

lazy_static::lazy_static! {
    pub static ref PKI: pki::Pki = pki::Pki::init().expect("failed to initialize PKI");
}
