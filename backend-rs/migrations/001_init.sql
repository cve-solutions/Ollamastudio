-- OllamaStudio database schema

CREATE TABLE IF NOT EXISTS sessions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    title       TEXT    NOT NULL DEFAULT 'Nouvelle session',
    skill_id    TEXT,
    model       TEXT    NOT NULL,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT    NOT NULL DEFAULT (datetime('now')),
    metadata    TEXT    NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS messages (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id   INTEGER NOT NULL,
    role         TEXT    NOT NULL,
    content      TEXT    NOT NULL,
    tool_calls   TEXT,
    tool_results TEXT,
    tokens_used  INTEGER,
    created_at   TEXT    NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);

CREATE TABLE IF NOT EXISTS documents (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    filename      TEXT    NOT NULL,
    relative_path TEXT    NOT NULL,
    mime_type     TEXT    NOT NULL,
    size_bytes    INTEGER NOT NULL,
    chunk_count   INTEGER NOT NULL DEFAULT 0,
    imported_at   TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS document_chunks (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    document_id  INTEGER NOT NULL,
    chunk_index  INTEGER NOT NULL,
    content      TEXT    NOT NULL,
    tokens       INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_chunks_doc ON document_chunks(document_id);

CREATE TABLE IF NOT EXISTS app_settings (
    key         TEXT PRIMARY KEY,
    value       TEXT    NOT NULL DEFAULT '',
    category    TEXT    NOT NULL DEFAULT 'general',
    label       TEXT    NOT NULL DEFAULT '',
    description TEXT    NOT NULL DEFAULT '',
    value_type  TEXT    NOT NULL DEFAULT 'string',
    updated_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- Auto-update updated_at on sessions
CREATE TRIGGER IF NOT EXISTS update_sessions_timestamp
AFTER UPDATE ON sessions
BEGIN
    UPDATE sessions SET updated_at = datetime('now') WHERE id = NEW.id;
END;

-- Auto-update updated_at on app_settings
CREATE TRIGGER IF NOT EXISTS update_settings_timestamp
AFTER UPDATE ON app_settings
BEGIN
    UPDATE app_settings SET updated_at = datetime('now') WHERE key = NEW.key;
END;
