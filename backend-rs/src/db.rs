use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;

pub async fn init_pool(db_path: &Path) -> Result<SqlitePool, sqlx::Error> {
    let url = format!("sqlite:{}?mode=rwc", db_path.display());
    let options = SqliteConnectOptions::from_str(&url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    // Run migrations
    run_migrations(&pool).await?;

    Ok(pool)
}

async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let sql = include_str!("../migrations/001_init.sql");

    // Split SQL respecting BEGIN...END blocks (for triggers)
    let statements = split_sql_statements(sql);
    for stmt in &statements {
        if !stmt.is_empty() {
            sqlx::query(stmt).execute(pool).await?;
        }
    }
    tracing::info!("Database migrations applied ({} statements)", statements.len());
    Ok(())
}

/// Split SQL on `;` but keep `BEGIN...END;` blocks together.
fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut in_begin = false;

    for line in sql.lines() {
        let trimmed = line.trim().to_uppercase();

        if trimmed.starts_with("--") {
            continue; // skip comments
        }

        current.push_str(line);
        current.push('\n');

        if trimmed.contains("BEGIN") {
            in_begin = true;
        }

        if in_begin && trimmed.contains("END;") {
            in_begin = false;
            let stmt = current.trim().to_string();
            if !stmt.is_empty() {
                statements.push(stmt);
            }
            current.clear();
        } else if !in_begin && current.contains(';') {
            // Split on ; outside of BEGIN blocks
            for part in current.split(';') {
                let s = part.trim().to_string();
                if !s.is_empty() {
                    statements.push(s);
                }
            }
            current.clear();
        }
    }

    let remaining = current.trim().to_string();
    if !remaining.is_empty() {
        statements.push(remaining);
    }

    statements
}
