use std::io::{BufRead, BufReader};
use crate::structs;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use anyhow::Context;
use log::{trace, debug, warn, error, info};

use nix::unistd::Pid;
use nix::sys::signal::{self, Signal};


const CLOUDFLARED_PATH: &str = "/usr/local/bin/cloudflared";
const MAX_RETRIES: u8 = 5;

fn execute_command(args: Vec<&str>) -> anyhow::Result<()> {
    debug!("executing {} with args {:?}", &CLOUDFLARED_PATH, &args);

    let output = Command::new(&CLOUDFLARED_PATH)
        .args(args)
        .output()?;

    info!("{}", &output.status);
    info!("stdout:\n{}", String::from_utf8_lossy(&output.stdout));
    info!("stderr:\n{}", String::from_utf8_lossy(&output.stderr));

    if !&output.status.success() {
        anyhow::bail!("command returned non-zero exit code")
    } else {
        Ok(())
    }

}

/// Create a Tunnel and then add said tunnel to the load balancer
pub(crate) fn cloudflared_create_tunnel(config: &structs::TunnelConfig) -> anyhow::Result<()> {
    let args = vec![
        "tunnel",
        "create",
        "--output=json",
        config.tunnel_name.as_str()
    ];

    // create the tunnel
    info!("creating tunnel {}", config.tunnel_name);
    execute_command(args)?;

    let args = vec![
        "tunnel",
        "route",
        "lb",
        config.tunnel_name.as_str(),
        config.target_lb.as_str(),
        config.target_pool.as_str()
    ];

    // add tunnel to the load balancer pool as an origin
    info!("adding tunnel {} to lb {} under pool {}", &config.tunnel_name, &config.target_lb, &config.target_pool);
    execute_command(args)?;

    Ok(())
}

/// Teardown the tunnel.
pub(crate) fn cloudflared_delete_tunnel(config: &structs::TunnelConfig) -> anyhow::Result<()> {

    let args = vec![
        "tunnel",
        "cleanup",
        config.tunnel_name.as_str()
    ];

    // cleanup the tunnel
    info!("cleaning up tunnel {}", config.tunnel_name);
    execute_command(args)?;

    // delete the tunnel
    let args = vec![
        "tunnel",
        "delete",
        "--force",
        config.tunnel_name.as_str()
    ];
    info!("deleting tunnel {}", config.tunnel_name);
    execute_command(args)?;

    // TODO Delete the record from the LB Pool so we don't have orphaned records lying around
    Ok(())
}


pub(crate) fn run_and_watch(config: &structs::TunnelConfig, sig_handler: Arc<AtomicBool>) -> anyhow::Result<()> {

    let args = vec![
        "tunnel".to_string(),
        "--no-autoupdate".to_string(),
        "--metrics=localhost:9981".to_string(),
        "run".to_string(),
        format!("--url={}", config.url),
        config.tunnel_name.clone()
    ];

    let mut consecutive_failures: u8 = 0;
    // launch tunnel
    loop {
        // launch inside a loop so we can easily re-start the process if it exists
        info!("launching tunnel {}", config.tunnel_name);
        debug!("executing {} with args {:?}", &CLOUDFLARED_PATH, &args);
        let mut proc = Command::new(&CLOUDFLARED_PATH)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = proc.stdout.take().context("Grabbing stdout")?;
        let stderr = proc.stderr.take().context("Grabbing stderr")?;

        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            reader
                .lines()
                .filter_map(|line| line.ok())
                .for_each(|line| println!("  {}", line));
        });

        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            reader
                .lines()
                .filter_map(|line| line.ok())
                .for_each(|line| println!("  {}", line));
        });

        info!("tunnel has been spawned with pid {}", &proc.id());

        // give it some time to start
        thread::sleep(Duration::from_secs(5));

        info!("starting to monitor tunnel health");

        if proc.try_wait()?.is_some() {
            warn!("child is already dead");
            consecutive_failures += 1;
            if consecutive_failures > 5 {
                error!("child could not stay alive");
                break;
            } else {
                continue;
            }
        }

        // now, just monitor it
        loop {

            if sig_handler.load(Ordering::Relaxed) {
                warn!("caught exit signal");
                let pid = Pid::from_raw(proc.id() as i32);

                let _ = signal::kill(pid, Signal::SIGTERM);

                let mut waited = 0;
                loop {
                    thread::sleep(Duration::from_secs(1));
                    match proc.try_wait()? {
                        None => {
                            if waited > 10 {
                                proc.kill()?;
                                break;
                            } else {
                                waited += 1;
                            }
                        }
                        Some(_) => { break; }
                    }
                }

                break;
            }

            // make sure the pid is still running
            match proc.try_wait()? {
                // child is still alive, so carry on
                None => { },
                // child has died, log it and break the monitoring loop
                Some(code) => {
                    warn!("child process has exited with code {}", code);
                    break;
                }
            }

            // the process is alive, let's poll the metrics endpoint to see if it's responsive
            let resp = reqwest::blocking::get("http://localhost:9981/metrics")?;
            if !&resp.status().is_success() {
                if consecutive_failures < MAX_RETRIES {
                    warn!("metrics endpoint returned invalid status code. {}/{}", &consecutive_failures, &MAX_RETRIES);
                    consecutive_failures += 1;
                } else {
                    // we have exceeded max retries, so we kill the process and try again
                    error!("exceeded max retries. killing child");
                    proc.kill()?;

                    // now, break the monitoring loop so it's respawned
                    break;
                }
            } else {
                trace!("health check response was successful");
                if consecutive_failures != 0 {
                    debug!("resetting consecutive_failures to zero as last response was successful");
                    consecutive_failures = 0;
                }
            }

            thread::sleep(Duration::from_secs(2));
        }

        if sig_handler.load(Ordering::Relaxed) {
            break;
        }

    }

    Ok(())
}