#[cfg(feature = "self-update")]
pub fn run(ui: &crate::ui::Ui) -> Result<(), String> {
    use self_update::backends::github::Update;
    let t = &ui.theme;

    let current = self_update::cargo_crate_version!();
    ui.out(&format!(
        " {}Текущая версия: {current}{}",
        t.dim(),
        t.reset()
    ));

    let (owner, name) = parse_owner_name(env!("CARGO_PKG_REPOSITORY"))?;

    let status = Update::configure()
        .repo_owner(&owner)
        .repo_name(&name)
        .bin_name("obsidian-sync")
        .show_output(true)
        .show_download_progress(true)
        .current_version(current)
        .build()
        .map_err(|e| format!("self_update build: {e}"))?
        .update()
        .map_err(|e| format!("self_update update: {e}"))?;

    ui.out(&format!(
        " {}Версия после обновления: {}{}",
        t.green(),
        status.version(),
        t.reset()
    ));
    Ok(())
}

#[cfg(not(feature = "self-update"))]
pub fn run(ui: &crate::ui::Ui) -> Result<(), String> {
    let t = &ui.theme;
    ui.out(&format!(
        " {}Автообновление не вшито в эту сборку.{}",
        t.yellow(),
        t.reset()
    ));
    ui.out(&format!(
        " {}Соберите с `cargo build --release --features self-update` или скачайте архив из GitHub Releases вручную.{}",
        t.dim(),
        t.reset()
    ));
    Err("self-update отключён в сборке".to_owned())
}

#[cfg(feature = "self-update")]
fn parse_owner_name(repo: &str) -> Result<(String, String), String> {
    let trimmed = repo.trim().trim_end_matches('/');
    let no_git = trimmed.strip_suffix(".git").unwrap_or(trimmed);
    let no_scheme = no_git
        .split_once("://")
        .map(|(_, r)| r)
        .unwrap_or(no_git);

    let parts: Vec<&str> = no_scheme.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() < 2 {
        return Err(format!(
            "не удалось извлечь owner/name из repository='{repo}'. Задай в Cargo.toml: repository = \"https://github.com/OWNER/NAME\""
        ));
    }

    let name = parts[parts.len() - 1];
    let owner = parts[parts.len() - 2];
    if owner.is_empty() || name.is_empty() {
        return Err(format!("пустой owner/name в repository='{repo}'"));
    }
    Ok((owner.to_owned(), name.to_owned()))
}