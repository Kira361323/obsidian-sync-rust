#[cfg(not(target_os = "android"))]
pub fn run(ui: &crate::ui::Ui) -> Result<(), String> {
    use self_update::backends::github::Update;
    let t = &ui.theme;

    let current = self_update::cargo_crate_version!();
    ui.out(&format!(
        " {}Текущая версия: {current}{}",
        t.dim(),
        t.reset()
    ));

    let status = Update::configure()
        .repo_owner(env!("CARGO_PKG_REPOSITORY_OWNER"))
        .repo_name(env!("CARGO_PKG_REPOSITORY_NAME"))
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

#[cfg(target_os = "android")]
pub fn run(ui: &crate::ui::Ui) -> Result<(), String> {
    let t = &ui.theme;
    ui.out(&format!(
        " {}На Termux/Android автообновление отключено.{}",
        t.yellow(),
        t.reset()
    ));
    ui.out(&format!(
        " {}Скачай архив из GitHub Releases и замени бинарник вручную.{}",
        t.dim(),
        t.reset()
    ));
    Err("self-update недоступен на android".to_owned())
}