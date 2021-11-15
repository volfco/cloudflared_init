use std::env;
use std::os::unix::fs::PermissionsExt;
use std::process::exit;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

mod structs;
mod cloudflared;


#[paw::main]
fn main(args: structs::Args) -> anyhow::Result<()> {
    env_logger::init();

    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term))?;
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&term))?;

    // ensure /etc/cloudflared is writable
    let mut tdir = std::fs::metadata("/etc/cloudflared")?.permissions();
    log::info!("/etc/cloudflared has permissions: {:?}", &tdir);

    tdir.set_readonly(false);
    tdir.set_mode(0o644);
    std::fs::set_permissions("/etc/cloudflared", tdir)?;

    // curl http://169.254.170.2/v4/8e887b9dba484906b029140ee7b0265a-3767383552/task
    if env::var("ECS_CONTAINER_METADATA_URI_V4").is_err() {
        log::error!("ECS_CONTAINER_METADATA_URI_V4 not found in ENV");
        #[cfg(not(debug_assertions))]
        exit(1);
    }

    let metadata: structs::EcsTask;
    if cfg!(debug_assertions) {
        log::warn!("debug build- using local data.json");
        let file = std::fs::read_to_string("./data.json").unwrap();
        metadata = serde_json::from_str(&file).unwrap();
    } else {
        log::debug!("loading ECS Container Metadata (v4) Task Metadata");
        let request = reqwest::blocking::get(format!("{}/task", env::var("ECS_CONTAINER_METADATA_URI_V4").unwrap()))?;
        metadata = request.json()?;
    }

    let az = metadata.availability_zone;
    let mut region = az.clone();
    region.pop();

    let task_arn = metadata.task_arn.split("/").collect::<Vec<&str>>();
    let tunnel_name = format!("{}-{}-{}", &az, &args.service_name, task_arn.last().unwrap());

    let tunnel_config = structs::TunnelConfig {
        tunnel_name,
        target_lb: "solidus-dev-testing.wyvrn.net".to_string(),
        target_pool: region,
        url: args.target_url.clone()
    };

    log::info!("using tunnel configuration: {:?}", tunnel_config);

    let mut result;
    result = cloudflared::cloudflared_create_tunnel(&tunnel_config);

    if args.health_delay > 0 {
        log::info!("waiting up to {} seconds for target url to become healthy", &args.health_delay);

        let mut healthy = false;
        let ticks = args.health_delay / 2;
        for i in 0..ticks {
            log::debug!("tick {}/{}", &i, &ticks);

            let request = reqwest::blocking::get(&args.target_url);
            if !request.is_err() {
                let status = request.unwrap().status();
                if status.is_success() {
                    healthy = true;
                    break;
                } else {
                    log::warn!("target url is not healthy. got status code {}", status);
                }
            } else {
                log::warn!("unable to connect to {}. {:?}", &args.target_url, request.err().unwrap());
            }

            std::thread::sleep(Duration::from_secs(2));
        }

    }


    result = cloudflared::run_and_watch(&tunnel_config, term.clone());
    cloudflared::cloudflared_delete_tunnel(&tunnel_config)?;

    result
}
