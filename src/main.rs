#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
use blake3;
use chrono::prelude::*;
use directories::{self, ProjectDirs};
use eframe::egui;
use egui_modal::Modal;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

static PROJECT_DIRS: LazyLock<ProjectDirs> = LazyLock::new(|| {
    // Hardcode the name for now
    let proj_dirs = directories::ProjectDirs::from("me", "tretrauit", "yawada")
        .expect("Failed to get config directory");
    proj_dirs
});

static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    let client = Client::new();
    client
});

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([720.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Yawada",
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

#[derive(Clone, Serialize, Deserialize)]
struct AllowedHost {
    host: String,
    enabled: bool,
}

#[derive(Clone, Serialize, Deserialize)]
struct HostsSource {
    url: String,
    last_updated: u64,
    enabled: bool,
}

#[derive(Clone, Serialize, Deserialize)]
struct RedirectedHost {
    host: String,
    ip: String,
    enabled: bool,
}

struct MyApp {
    blocked_hosts: Vec<String>,
    allowed_hosts: Vec<AllowedHost>,
    redirected_hosts: Vec<RedirectedHost>,
    is_hosts_file_installed: bool,
    hosts_sources: Vec<HostsSource>,
    hosts_sources_last_updated: u64,
    // UI parts
    show_edit_sources: bool,
    show_edit_allowed_hosts: bool,
    show_edit_redirect_hosts: bool,
    show_confirmation_dialog: bool,
    allowed_to_close: bool,
    // HACK
    first_run: bool,
    dialog_error_body: String,
    dialog_error_title: String,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            blocked_hosts: vec![],
            allowed_hosts: vec![],
            redirected_hosts: vec![],
            is_hosts_file_installed: false,
            hosts_sources: vec![],
            hosts_sources_last_updated: 0,
            show_confirmation_dialog: false,
            show_edit_sources: false,
            show_edit_allowed_hosts: false,
            show_edit_redirect_hosts: false,
            allowed_to_close: false,
            first_run: true,
            dialog_error_body: String::new(),
            dialog_error_title: String::new(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Generic modal
        let modal = Modal::new(ctx, "generic_modal");
        modal.show(|ui| {
            modal.title(ui, self.dialog_error_title.clone());
            modal.body(ui, self.dialog_error_body.clone());
            modal.buttons(ui, |ui| {
                // After clicking, the modal is automatically closed
                modal.button(ui, "OK").clicked();
            });
        });
        let mut show_modal = |title: String, body: String| {
            self.dialog_error_title = title;
            self.dialog_error_body = body;
            modal.open();
        };
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.menu_button("Menu", |ui| {
                if ui.button("Settings").clicked() {
                    ui.close_menu();
                }
            });
            ui.heading("Yawada");
            ui.label("An open-source system-wide adblocker");
            ui.add_space(10.0);
            ui.heading("Statistics");
            ui.horizontal(|ui| {
                ui.label(format!("Blocked hosts: {}", self.blocked_hosts.len()));
                // I don't want to implement this so okay :)
                // if ui.button("Edit").clicked() {
                //     println!("TODO");
                // }
            });
            ui.horizontal(|ui| {
                ui.label(format!("Allowed hosts: {}", self.allowed_hosts.len()));
                if ui.button("Edit").clicked() {
                    self.show_edit_allowed_hosts = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(format!("Redirected hosts: {}", self.redirected_hosts.len()));
                if ui.button("Edit").clicked() {
                    self.show_edit_redirect_hosts = true;
                }
            });
            ui.label(format!(
                "Is hosts file installed?: {}",
                self.is_hosts_file_installed
            ));
            ui.horizontal(|ui| {
                if ui.button("Install/Update").clicked() {
                    println!("TODO");
                }
                if ui.button("Uninstall").clicked() {
                    println!("TODO");
                }
            });
            ui.add_space(10.0);
            ui.heading("Hosts sources");
            ui.label(format!(
                "{} sources are installed",
                self.hosts_sources.len()
            ));
            ui.label(format!(
                "Last updated: {}",
                if self.hosts_sources_last_updated == 0 {
                    String::from("Never")
                } else {
                    let datetime =
                        DateTime::from_timestamp(self.hosts_sources_last_updated as i64, 0)
                            .unwrap();
                    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                }
            ));
            ui.horizontal(|ui| {
                if ui.button("Update").clicked() {
                    for (i, source) in self.hosts_sources.clone().into_iter().enumerate() {
                        // Actually update the source
                        match CLIENT.get(source.url.clone()).send() {
                            Ok(response) => {
                                let body = response.text().unwrap();
                                let file_name =
                                    blake3::hash(source.url.as_bytes()).to_hex().to_string();
                                let config_dir = PROJECT_DIRS.config_dir();
                                let hosts_sources_path =
                                    config_dir.join("hosts_sources").join(file_name);
                                match fs::write(&hosts_sources_path, body) {
                                    Ok(_) => {
                                        println!("Fetched hosts source for index: {}", i);
                                        let current_time = SystemTime::now()
                                            .duration_since(UNIX_EPOCH)
                                            .unwrap()
                                            .as_secs();
                                        self.hosts_sources[i].last_updated = current_time;
                                        self.hosts_sources_last_updated = current_time;
                                    }
                                    Err(e) => {
                                        println!("Failed to fetch hosts source: {}", e);
                                        show_modal(
                                            "Error".to_string(),
                                            format!("Failed to fetch hosts source: {}", e),
                                        );
                                        return;
                                    }
                                }
                            }
                            Err(e) => {
                                println!("Failed to fetch hosts source: {}", e);
                                show_modal(
                                    "Error".to_string(),
                                    format!("Failed to fetch hosts source: {}", e),
                                );
                                return;
                            }
                        }
                    }
                }
                if ui.button("Edit sources").clicked() {
                    self.show_edit_sources = true;
                }
            });
        });
        // Modals
        // Conflict modal
        let conflict_hosts_modal = Modal::new(ctx, "conflict_hosts_modal");
        conflict_hosts_modal.show(|ui| {
            conflict_hosts_modal.title(ui, "Error");
            conflict_hosts_modal.body(ui, "There are conflict entries in the hosts source list, please remove them before continuing.");
            conflict_hosts_modal.buttons(ui, |ui| {
                // After clicking, the modal is automatically closed
                conflict_hosts_modal.button(ui, "OK").clicked();
            });
        });
        let host_url_empty_modal = Modal::new(ctx, "host_url_empty_modal");
        host_url_empty_modal.show(|ui| {
            host_url_empty_modal.title(ui, "Error");
            host_url_empty_modal.body(ui, "The host URL cannot be empty.");
            host_url_empty_modal.buttons(ui, |ui| {
                // After clicking, the modal is automatically closed
                host_url_empty_modal.button(ui, "OK").clicked();
            });
        });
        // Close confirmation modal
        let close_confirmation_modal = Modal::new(ctx, "close_confirmation_modal");
        close_confirmation_modal.show(|ui| {
            close_confirmation_modal.title(ui, "Confirmation");
            close_confirmation_modal.body(ui, "Are you sure you want to quit?");
            close_confirmation_modal.buttons(ui, |ui| {
                // After clicking, the modal is automatically closed
                close_confirmation_modal.button(ui, "No").clicked();
                if close_confirmation_modal.button(ui, "Yes").clicked() {
                    self.show_confirmation_dialog = false;
                    self.allowed_to_close = true;
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
        // End of modals
        if ctx.input(|i| i.viewport().close_requested()) {
            if !self.allowed_to_close {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                close_confirmation_modal.open();
            }
        }
        // First run of the loop
        if self.first_run {
            // Load the hosts sources
            let config_dir = PROJECT_DIRS.config_dir();
            println!("Config dir: {:?}", config_dir);
            // Creates yawada/config and yawada/config/host_sources/ directories
            fs::create_dir_all(&config_dir.join("hosts_sources")).unwrap();
            println!("Created config directory");
            let hosts_sources_path = config_dir.join("hosts_sources.json");
            if hosts_sources_path.exists() {
                println!("Hosts sources file exists, loading...");
                let hosts_sources_file = fs::read_to_string(hosts_sources_path).unwrap();
                let hosts_sources: Vec<HostsSource> =
                    match serde_json::from_str(&hosts_sources_file) {
                        Ok(s) => s,
                        Err(e) => {
                            println!("Failed to load hosts sources file: {}", e);
                            show_modal(
                                "Error".to_string(),
                                format!("Failed to load hosts sources file: {}", e),
                            );
                            vec![]
                        }
                    };
                self.hosts_sources = hosts_sources;
                if self.hosts_sources.len() > 0 {
                    self.hosts_sources_last_updated = self.hosts_sources[0].last_updated;
                }
            }
            let allowed_hosts_path = config_dir.join("allowed_hosts.json");
            if allowed_hosts_path.exists() {
                println!("Allowed hosts file exists, loading...");
                let allowed_hosts_file = fs::read_to_string(allowed_hosts_path).unwrap();
                let allowed_hosts: Vec<AllowedHost> =
                    match serde_json::from_str(&allowed_hosts_file) {
                        Ok(s) => s,
                        Err(e) => {
                            println!("Failed to load alllowed hosts file: {}", e);
                            show_modal(
                                "Error".to_string(),
                                format!("Failed to load allowed hosts file: {}", e),
                            );
                            vec![]
                        }
                    };
                self.allowed_hosts = allowed_hosts;
            }
            let redirected_hosts_path = config_dir.join("redirected_hosts.json");
            if redirected_hosts_path.exists() {
                println!("Redirected hosts file exists, loading...");
                let redirected_hosts_file = fs::read_to_string(redirected_hosts_path).unwrap();
                let redirected_hosts: Vec<RedirectedHost> =
                    match serde_json::from_str(&redirected_hosts_file) {
                        Ok(s) => s,
                        Err(e) => {
                            println!("Failed to load redirected hosts file: {}", e);
                            show_modal(
                                "Error".to_string(),
                                format!("Failed to load redirected hosts file: {}", e),
                            );
                            vec![]
                        }
                    };
                self.redirected_hosts = redirected_hosts;
            }
            self.first_run = false;
        }
        if self.show_edit_allowed_hosts {
            egui::Window::new("Allowed hosts")
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.label("Allowed hosts are used to allow a host to be accessed.");
                    ui.horizontal(|ui| {
                        if ui.button("Add").clicked() {
                            self.allowed_hosts.push(AllowedHost {
                                host: String::new(),
                                enabled: true,
                            });
                        }
                        if ui.button("Save & Close").clicked() {
                            // Check if there is conflicting sources
                            // If there is, show a dialog
                            let mut urls: Vec<String> = vec![];
                            for sources in self.allowed_hosts.iter() {
                                if sources.host.is_empty() {
                                    host_url_empty_modal.open();
                                    return;
                                }
                                if urls.contains(&sources.host) {
                                    // Show a dialog
                                    conflict_hosts_modal.open();
                                    return;
                                }
                                urls.push(sources.host.clone());
                            }
                            // Actually save the sources
                            let config_dir = PROJECT_DIRS.config_dir();
                            let hosts_sources_path = config_dir.join("allowed_hosts.json");
                            match fs::write(
                                hosts_sources_path,
                                serde_json::to_string(&self.allowed_hosts).unwrap(),
                            ) {
                                Ok(_) => {
                                    println!("Saved allowed hosts");
                                }
                                Err(e) => {
                                    println!("Failed to save allowed hosts: {}", e);
                                    show_modal(
                                        "Error".to_string(),
                                        format!("Failed to save allowed hosts: {}", e),
                                    );
                                    return;
                                }
                            }
                            self.show_edit_allowed_hosts = false;
                        }
                    });
                    // Create a list of sources so we can modify them ourselves :)
                    let allowed_hosts = self.allowed_hosts.clone();
                    for (i, _) in allowed_hosts.iter().enumerate() {
                        ui.horizontal(|ui| {
                            // Stop if we reach the end of the list
                            // Otherwise it'll panic lol
                            if i == self.allowed_hosts.len() {
                                return;
                            }
                            ui.checkbox(&mut self.allowed_hosts[i].enabled, "");
                            ui.text_edit_singleline(&mut self.allowed_hosts[i].host);
                            if ui.button("X").clicked() {
                                println!("Removing index: {}", i);
                                self.allowed_hosts.remove(i);
                            }
                        });
                    }
                });
        }
        if self.show_edit_redirect_hosts {
            egui::Window::new("Redirected hosts")
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.label("Redirected hosts are used to redirect a host to a specific IP address.");
                    ui.label("The left column is the host, and the right column is the IP address.");
                    ui.horizontal(|ui| {
                        if ui.button("Add").clicked() {
                            self.redirected_hosts.push(RedirectedHost {
                                host: String::new(),
                                ip: String::new(),
                                enabled: true,
                            });
                        }
                        if ui.button("Save & Close").clicked() {
                            // Check if there is conflicting sources
                            // If there is, show a dialog
                            let mut urls: Vec<String> = vec![];
                            for sources in self.redirected_hosts.iter() {
                                if sources.host.is_empty() {
                                    host_url_empty_modal.open();
                                    return;
                                }
                                if urls.contains(&sources.host) {
                                    // Show a dialog
                                    conflict_hosts_modal.open();
                                    return;
                                }
                                urls.push(sources.host.clone());
                            }
                            // Actually save the sources
                            let config_dir = PROJECT_DIRS.config_dir();
                            let hosts_sources_path = config_dir.join("redirected_hosts.json");
                            match fs::write(
                                hosts_sources_path,
                                serde_json::to_string(&self.redirected_hosts).unwrap(),
                            ) {
                                Ok(_) => {
                                    println!("Saved redirected hosts");
                                }
                                Err(e) => {
                                    println!("Failed to save redirected hosts: {}", e);
                                    show_modal(
                                        "Error".to_string(),
                                        format!("Failed to save redirected hosts: {}", e),
                                    );
                                    return;
                                }
                            }
                            self.show_edit_redirect_hosts = false;
                        }
                    });
                    // Create a list of sources so we can modify them ourselves :)
                    let redirected_hosts = self.redirected_hosts.clone();
                    for (i, _) in redirected_hosts.iter().enumerate() {
                        ui.horizontal(|ui| {
                            // Stop if we reach the end of the list
                            // Otherwise it'll panic lol
                            if i == self.redirected_hosts.len() {
                                return;
                            }
                            ui.checkbox(&mut self.redirected_hosts[i].enabled, "");
                            ui.text_edit_singleline(&mut self.redirected_hosts[i].host);
                            ui.text_edit_singleline(&mut self.redirected_hosts[i].ip);
                            if ui.button("X").clicked() {
                                println!("Removing index: {}", i);
                                self.redirected_hosts.remove(i);
                            }
                        });
                    }
                });
        }
        if self.show_edit_sources {
            egui::Window::new("Hosts sources")
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Add").clicked() {
                            self.hosts_sources.push(HostsSource {
                                url: String::new(),
                                last_updated: 0,
                                enabled: true,
                            });
                        }
                        if ui.button("Save & Close").clicked() {
                            // Check if there is conflicting sources
                            // If there is, show a dialog
                            let mut urls: Vec<String> = vec![];
                            for sources in self.hosts_sources.iter() {
                                if sources.url.is_empty() {
                                    host_url_empty_modal.open();
                                    return;
                                }
                                if urls.contains(&sources.url) {
                                    // Show a dialog
                                    conflict_hosts_modal.open();
                                    return;
                                }
                                urls.push(sources.url.clone());
                            }
                            // Actually save the sources
                            let config_dir = PROJECT_DIRS.config_dir();
                            let hosts_sources_path = config_dir.join("hosts_sources.json");
                            match fs::write(
                                hosts_sources_path,
                                serde_json::to_string(&self.hosts_sources).unwrap(),
                            ) {
                                Ok(_) => {
                                    println!("Saved hosts sources");
                                }
                                Err(e) => {
                                    println!("Failed to save hosts sources: {}", e);
                                    show_modal(
                                        "Error".to_string(),
                                        format!("Failed to save hosts sources: {}", e),
                                    );
                                    return;
                                }
                            }
                            self.show_edit_sources = false;
                        }
                    });
                    // Create a list of sources so we can modify them ourselves :)
                    let hosts_source = self.hosts_sources.clone();
                    for (i, _) in hosts_source.iter().enumerate() {
                        ui.horizontal(|ui| {
                            // Stop if we reach the end of the list
                            // Otherwise it'll panic lol
                            if i == self.hosts_sources.len() {
                                return;
                            }
                            ui.checkbox(&mut self.hosts_sources[i].enabled, "");
                            ui.text_edit_singleline(&mut self.hosts_sources[i].url);
                            ui.label(format!(
                                "Last updated: {}",
                                if self.hosts_sources[i].last_updated == 0 {
                                    String::from("Never")
                                } else {
                                    let datetime = DateTime::from_timestamp(
                                        self.hosts_sources[i].last_updated as i64,
                                        0,
                                    )
                                    .unwrap();
                                    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                                }
                            ));
                            let update_btn = ui.button("Update");
                            if update_btn.clicked() {
                                // Check if there is conflicting sources
                                // If there is, show a dialog
                                if self.hosts_sources[i].url.is_empty() {
                                    host_url_empty_modal.open();
                                    return;
                                }
                                let mut urls: Vec<String> = vec![];
                                for sources in self.hosts_sources.iter() {
                                    if urls.contains(&sources.url) {
                                        // Show a dialog
                                        conflict_hosts_modal.open();
                                        return;
                                    }
                                    urls.push(sources.url.clone());
                                }
                                // Actually update the source
                                match CLIENT.get(&self.hosts_sources[i].url).send() {
                                    Ok(response) => {
                                        let body = response.text().unwrap();
                                        let file_name =
                                            blake3::hash(&self.hosts_sources[i].url.as_bytes())
                                                .to_hex()
                                                .to_string();
                                        let config_dir = PROJECT_DIRS.config_dir();
                                        let hosts_sources_path =
                                            config_dir.join("hosts_sources").join(file_name);
                                        match fs::write(&hosts_sources_path, body) {
                                            Ok(_) => {
                                                println!("Fetched hosts source");
                                            }
                                            Err(e) => {
                                                println!("Failed to fetch hosts source: {}", e);
                                                show_modal(
                                                    "Error".to_string(),
                                                    format!("Failed to fetch hosts source: {}", e),
                                                );
                                                return;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        println!("Failed to fetch hosts source: {}", e);
                                        show_modal(
                                            "Error".to_string(),
                                            format!("Failed to fetch hosts source: {}", e),
                                        );
                                        return;
                                    }
                                }
                                let current_time = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs();
                                self.hosts_sources[i].last_updated = current_time;
                                self.hosts_sources_last_updated = current_time;
                            }
                            if ui.button("X").clicked() {
                                println!("Removing index: {}", i);
                                self.hosts_sources.remove(i);
                            }
                        });
                    }
                });
        }
    }
}
