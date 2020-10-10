pub fn admin_subcommand(
    sub_m: &clap::ArgMatches,
    server_settings: &server::Settings,
    editable_settings: &mut server::EditableSettings,
    data_dir: &std::path::Path,
) {
    let login_provider =
        server::login_provider::LoginProvider::new(server_settings.auth_server_address.clone());

    match sub_m.subcommand() {
        ("add", Some(sub_m)) => {
            if let Some(username) = sub_m.value_of("username") {
                server::add_admin(username, &login_provider, editable_settings, data_dir)
            }
        },
        ("remove", Some(sub_m)) => {
            if let Some(username) = sub_m.value_of("username") {
                server::remove_admin(username, &login_provider, editable_settings, data_dir)
            }
        },
        // TODO: can clap enforce this?
        // or make this list current admins or something
        _ => tracing::error!(
            "Invalid input, use one of the subcommands listed using: \nveloren-server-cli help \
             admin"
        ),
    }
}
