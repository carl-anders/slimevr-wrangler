use self_update::{
    backends::github, cargo_crate_version, errors::Error, update::ReleaseUpdate, version,
};

fn update_config() -> Result<Box<dyn ReleaseUpdate>, Error> {
    github::Update::configure()
        .repo_owner("carl-anders")
        .repo_name("slimevr-wrangler")
        .bin_name("slimevr-wrangler")
        .show_download_progress(true)
        .current_version(cargo_crate_version!())
        .no_confirm(true)
        .build()
}
pub async fn check_updates() -> Option<String> {
    let version = async_std::task::spawn_blocking(|| {
        if let Ok(conf) = update_config() {
            if let Ok(release) = conf.get_latest_release() {
                match version::bump_is_greater(env!("CARGO_PKG_VERSION"), &release.version) {
                    Ok(new_version) if new_version => {
                        return Some(release.version);
                    }
                    _ => {}
                }
            }
        }
        None
    })
    .await;
    version
}
pub fn update() {
    if let Ok(conf) = update_config() {
        match conf.update() {
            Ok(_) => {
                println!("Update complete.");
            }
            Err(e) => {
                println!("Update not successful.\n{}", e);
            }
        }
    }
}
