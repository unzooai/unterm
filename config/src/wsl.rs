use crate::config::validate_domain_name;
use crate::*;
use std::collections::HashMap;
use wezterm_dynamic::{FromDynamic, ToDynamic};

#[derive(Default, Debug, Clone, FromDynamic, ToDynamic)]
pub struct WslDomain {
    #[dynamic(validate = "validate_domain_name")]
    pub name: String,
    pub distribution: Option<String>,
    pub username: Option<String>,
    pub default_cwd: Option<PathBuf>,
    pub default_prog: Option<Vec<String>>,
}

impl WslDomain {
    pub fn default_domains() -> Vec<WslDomain> {
        #[allow(unused_mut)]
        let mut domains = vec![];

        #[cfg(windows)]
        if let Ok(distros) = WslDistro::load_distro_list() {
            for distro in distros {
                domains.push(WslDomain {
                    name: format!("WSL:{}", distro.name),
                    distribution: Some(distro.name.clone()),
                    username: None,
                    default_cwd: Some("~".into()),
                    default_prog: None,
                });
            }
        }

        domains
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WslDistro {
    pub name: String,
    pub state: String,
    pub version: String,
    pub is_default: bool,
}

impl WslDistro {
    /// Enumerate installed WSL distros from the registry. This is what
    /// `wsl.exe -l -v` reads too, but without spawning a subprocess —
    /// the subprocess route costs 50-80ms on the startup critical path.
    #[cfg(windows)]
    fn load_distro_list_from_registry() -> anyhow::Result<Vec<Self>> {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let lxss = match hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Lxss") {
            Ok(k) => k,
            // No Lxss key means WSL has never registered a distro on
            // this machine; report "no distros" rather than erroring,
            // so the caller doesn't fall back to spawning wsl.exe.
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(err) => return Err(err.into()),
        };
        let default_guid: String = lxss.get_value("DefaultDistribution").unwrap_or_default();

        let mut distros = vec![];
        for guid in lxss.enum_keys().flatten() {
            let distro_key = match lxss.open_subkey(&guid) {
                Ok(k) => k,
                Err(_) => continue,
            };
            let name: String = match distro_key.get_value("DistributionName") {
                Ok(name) => name,
                Err(_) => continue,
            };
            // Modern distro packages install under a "State" of 1 once
            // they are fully registered; anything else is mid-install
            // or partially removed, matching what wsl.exe -l shows.
            let state: u32 = distro_key.get_value("State").unwrap_or(1);
            if state != 1 {
                continue;
            }
            let version: u32 = distro_key.get_value("Version").unwrap_or(2);
            distros.push(WslDistro {
                name,
                state: "Registered".to_string(),
                version: version.to_string(),
                is_default: guid == default_guid,
            });
        }
        Ok(distros)
    }

    pub fn load_distro_list() -> anyhow::Result<Vec<Self>> {
        // Prefer the registry: same data as `wsl.exe -l -v`, none of the
        // subprocess cost. Fall back to wsl.exe if the registry shape
        // ever changes.
        #[cfg(windows)]
        match Self::load_distro_list_from_registry() {
            Ok(distros) => return Ok(distros),
            Err(err) => {
                log::debug!("WSL registry enumeration failed, falling back to wsl.exe: {err:#}");
            }
        }

        #[cfg(windows)]
        use std::os::windows::process::CommandExt;
        let mut cmd = std::process::Command::new("wsl.exe");
        cmd.arg("-l");
        cmd.arg("-v");
        #[cfg(windows)]
        cmd.creation_flags(winapi::um::winbase::CREATE_NO_WINDOW);
        let output = cmd.output()?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::ensure!(
            output.status.success(),
            "wsl -l command invocation failed: {}",
            stderr
        );

        /// Ungh: https://github.com/microsoft/WSL/issues/4456
        fn utf16_to_utf8(bytes: &[u8]) -> anyhow::Result<String> {
            if bytes.len() % 2 != 0 {
                anyhow::bail!("input data has odd length, cannot be utf16");
            }

            // This is "safe" because we checked that the length seems reasonable,
            // and our new slice is within those same bounds.
            let wide: &[u16] = unsafe {
                std::slice::from_raw_parts(bytes.as_ptr() as *const u16, bytes.len() / 2)
            };

            String::from_utf16(wide).map_err(|_| anyhow!("wsl -l -v output is not valid utf16"))
        }

        let wsl_list = utf16_to_utf8(&output.stdout)?.replace("\r\n", "\n");

        Ok(parse_wsl_distro_list(&wsl_list))
    }
}

/// This function parses the `wsl -l -v` output.
/// It tries to be robust in the face of future changes
/// by looking at the tabulated output headers, determining
/// where the columns are and then collecting the information
/// into a hashmap and then grokking from there.
#[allow(dead_code)]
fn parse_wsl_distro_list(output: &str) -> Vec<WslDistro> {
    let lines = output.lines().collect::<Vec<_>>();

    // Determine where the field columns start
    let mut field_starts = vec![];
    {
        let mut last_char = ' ';
        for (idx, c) in lines[0].char_indices() {
            if last_char == ' ' && c != ' ' {
                field_starts.push(idx);
            }
            last_char = c;
        }
    }

    fn field_slice(s: &str, start: usize, end: Option<usize>) -> &str {
        if let Some(end) = end {
            &s[start..end]
        } else {
            &s[start..]
        }
    }

    fn opt_field_slice(s: &str, start: usize, end: Option<usize>) -> Option<&str> {
        if let Some(end) = end {
            s.get(start..end)
        } else {
            s.get(start..)
        }
    }

    // Now build up a name -> column position map
    let mut field_map = HashMap::new();
    {
        let mut iter = field_starts.into_iter().peekable();

        while let Some(start_idx) = iter.next() {
            let end_idx = iter.peek().copied();
            let label = field_slice(&lines[0], start_idx, end_idx).trim();
            field_map.insert(label, (start_idx, end_idx));
        }
    }

    let mut result = vec![];

    // and now process the output rows
    for line in lines.iter().skip(1) {
        if line.is_empty() {
            continue;
        }

        let is_default = line.starts_with("*");

        let mut fields = HashMap::new();
        for (label, (start_idx, end_idx)) in field_map.iter() {
            if let Some(value) = opt_field_slice(line, *start_idx, *end_idx) {
                fields.insert(*label, value.trim().to_string());
            } else {
                return result;
            }
        }

        result.push(WslDistro {
            name: fields.get("NAME").cloned().unwrap_or_default(),
            state: fields.get("STATE").cloned().unwrap_or_default(),
            version: fields.get("VERSION").cloned().unwrap_or_default(),
            is_default,
        });
    }

    result
}

#[cfg(test)]
#[test]
fn test_parse_wsl_distro_list() {
    let data = "  NAME                   STATE           VERSION
* Arch                   Running         2
  docker-desktop-data    Stopped         2
  docker-desktop         Stopped         2
  Ubuntu                 Stopped         2
  nvim                   Stopped         2";

    assert_eq!(
        parse_wsl_distro_list(data),
        vec![
            WslDistro {
                name: "Arch".to_string(),
                state: "Running".to_string(),
                version: "2".to_string(),
                is_default: true
            },
            WslDistro {
                name: "docker-desktop-data".to_string(),
                state: "Stopped".to_string(),
                version: "2".to_string(),
                is_default: false
            },
            WslDistro {
                name: "docker-desktop".to_string(),
                state: "Stopped".to_string(),
                version: "2".to_string(),
                is_default: false
            },
            WslDistro {
                name: "Ubuntu".to_string(),
                state: "Stopped".to_string(),
                version: "2".to_string(),
                is_default: false
            },
            WslDistro {
                name: "nvim".to_string(),
                state: "Stopped".to_string(),
                version: "2".to_string(),
                is_default: false
            },
        ]
    );
}
