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

use crate::config::DatabaseConfig;
use crate::error::Result;
use sea_orm::{ConnectOptions, Database as SeaDatabase, DatabaseConnection};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::OnceCell;
use tracing::info;

/// Database connection pool wrapper with lazy initialization
#[derive(Debug, Clone)]
pub struct Database {
    inner: Arc<DatabaseInner>,
}

#[derive(Debug)]
struct DatabaseInner {
    /// Lazy-initialized database connection
    connection: OnceCell<DatabaseConnection>,
    /// Database connection URL
    database_url: String,
}

impl Database {
    /// Creates a new Database instance without establishing connection
    /// The actual connection will be established on first use
    pub fn new(database_url: &str) -> Self {
        Self {
            inner: Arc::new(DatabaseInner {
                connection: OnceCell::new(),
                database_url: database_url.to_string(),
            }),
        }
    }

    /// Creates a Database instance from configuration
    pub fn from_config(config: &DatabaseConfig) -> Self {
        Self::new(&config.url())
    }

    /// Initializes the database connection with configured parameters
    async fn init_connection(&self) -> Result<DatabaseConnection> {
        let mut opt = ConnectOptions::new(self.inner.database_url.clone());

        opt.min_connections(1)
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(8))
            .connect_timeout(Duration::from_secs(8))
            .idle_timeout(Duration::from_secs(8))
            .max_lifetime(Duration::from_secs(8))
            .sqlx_logging(true);

        // Establish the connection
        let connection = SeaDatabase::connect(opt).await?;

        // Test the connection immediately to ensure it's working
        connection.ping().await?;

        info!("Database connected successfully");

        Ok(connection)
    }

    /// Gets the database connection, initializing it if necessary
    /// This method is thread-safe and will only initialize the connection once
    pub async fn connection(&self) -> Result<&DatabaseConnection> {
        self.inner
            .connection
            .get_or_try_init(|| self.init_connection())
            .await
    }

    /// Checks if the database connection has been initialized
    pub fn is_connected(&self) -> bool {
        self.inner.connection.get().is_some()
    }

    /// Forces a connection initialization without returning it
    /// Useful for warming up the connection pool during startup
    pub async fn ensure_connected(&self) -> Result<()> {
        self.connection().await?;
        Ok(())
    }
}
