// Copyright 2025 AprilNEA LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use config::{Config, ConfigError, File};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::path::PathBuf;

pub static CONFIG: Lazy<Settings> =
    Lazy::new(|| Settings::new().expect("Failed to load configuration"));

#[derive(Parser, Debug)]
#[command(name = "oxidoor-server")]
#[command(about = "OXIDOOR Server", long_about = None)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Server address (override configuration file)
    #[arg(long, env = "APP_SERVER_HOST")]
    host: Option<String>,

    /// Server port (override configuration file)
    #[arg(short, long, env = "APP_SERVER_PORT")]
    port: Option<u16>,

    /// Log level
    #[arg(long, value_name = "LEVEL", default_value = "info")]
    log_level: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Settings {
    pub log_level: String,
    pub app: AppConfig,
    pub db: DatabaseConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub frontend_url: String,
    pub allowed_origins: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum DatabaseConfig {
    Sqlite {
        #[serde(default = "default_sqlite_url")]
        url: String,
    },
    Postgresql {
        url: String,
    },
}

impl DatabaseConfig {
    pub fn url(&self) -> String {
        match self {
            DatabaseConfig::Sqlite { url } => url.clone(),
            DatabaseConfig::Postgresql { url } => url.clone(),
        }
    }
}

fn default_sqlite_url() -> String {
    "sqlite://./data.db".to_string()
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let cli = Cli::parse();

        // 1. Start building configuration
        let mut builder = Config::builder();

        // 2. Load default configuration file
        builder = builder.add_source(File::with_name("config/default").required(false));

        // 3. Load environment-specific configuration
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());
        builder =
            builder.add_source(File::with_name(&format!("config/{}", run_mode)).required(false));

        // 4. If custom config file path is specified, load it
        if let Some(config_path) = &cli.config {
            builder = builder.add_source(File::with_name(config_path).required(true));
        }

        // 5. Load from environment variables (overrides file configuration)
        builder = builder.add_source(
            Environment::with_prefix("APP")
                .separator("__")
                .try_parsing(true),
        );

        // 6. Apply command line arguments (highest priority)
        builder = apply_cli_args(builder, &cli);

        // 7. Build and deserialize configuration
        let config = builder.build()?;
        config.try_deserialize()
    }
}

fn apply_cli_args(
    mut builder: config::ConfigBuilder<config::builder::DefaultState>,
    cli: &Cli,
) -> config::ConfigBuilder<config::builder::DefaultState> {
    // Override server configuration
    if let Some(host) = &cli.host {
        builder = builder.set_override("server.host", host.clone()).unwrap();
    }

    if let Some(port) = cli.port {
        builder = builder.set_override("server.port", port as i64).unwrap();
    }

    builder = builder
        .set_override("log_level", cli.log_level.clone())
        .unwrap();

    builder
}
