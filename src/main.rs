#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
use chrono::prelude::*;
use directories::{self, ProjectDirs};
use eframe::egui;
use egui_modal::Modal;
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

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([460.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Yawada",
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

#[derive(Clone, Serialize, Deserialize)]
struct HostsSource {
    url: String,
    last_updated: u64,
    enabled: bool,
}

struct MyApp {
    blocked_hosts: Vec<String>,
    allowed_hosts: Vec<String>,
    redirected_hosts: Vec<String>,
    is_hosts_file_installed: bool,
    hosts_sources: Vec<HostsSource>,
    hosts_sources_last_updated: u64,
    // UI parts
    show_edit_sources: bool,
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
            allowed_to_close: false,
            first_run: true,
            dialog_error_body: String::new(),
            dialog_error_title: String::new(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                if ui.button("Edit").clicked() {
                    println!("TODO");
                }
            });
            ui.horizontal(|ui| {
                ui.label(format!("Allowed hosts: {}", self.allowed_hosts.len()));
                if ui.button("Edit").clicked() {
                    println!("TODO");
                }
            });
            ui.horizontal(|ui| {
                ui.label(format!("Redirected hosts: {}", self.redirected_hosts.len()));
                if ui.button("Edit").clicked() {
                    println!("TODO");
                }
            });
            ui.label(format!(
                "Is hosts file installed?: {}",
                self.is_hosts_file_installed
            ));
            ui.horizontal(|ui| {
                if ui.button("Install").clicked() {
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
                    println!("TODO");
                }
                if ui.button("Edit sources").clicked() {
                    self.show_edit_sources = true;
                }
            });
        });
        // Modals
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
                let hosts_sources: Vec<HostsSource> = match serde_json::from_str(&hosts_sources_file) {
                    Ok(s) => s,
                    Err(e) => {
                        println!("Failed to load hosts sources file: {}", e);
                        show_modal("Error".to_string(), format!("Failed to load hosts sources file: {}", e));
                        vec![]
                    }
                };
                self.hosts_sources = hosts_sources;
                if self.hosts_sources.len() > 0 {
                    self.hosts_sources_last_updated = self.hosts_sources[0].last_updated;
                }
            }
            self.first_run = false;
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
                        if ui.button("Close").clicked() {
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
                            match fs::write(hosts_sources_path, serde_json::to_string(&self.hosts_sources).unwrap()) {
                                Ok(_) => {
                                    println!("Saved hosts sources");
                                }
                                Err(e) => {
                                    println!("Failed to save hosts sources: {}", e);
                                    show_modal("Error".to_string(), format!("Failed to save hosts sources: {}", e));
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
                            if ui.button("Update").clicked() {
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
                                println!("TODO");
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
