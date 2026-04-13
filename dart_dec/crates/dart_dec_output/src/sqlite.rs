use crate::{OutputMeta, OutputStats};
use rusqlite::{Connection, params};
use std::path::Path;

pub fn create_database(path: &Path) -> Result<Connection, rusqlite::Error> {
    let conn = Connection::open(path)?;
    
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT);
        CREATE TABLE IF NOT EXISTS libraries (id INTEGER PRIMARY KEY, name TEXT, url TEXT);
        CREATE TABLE IF NOT EXISTS classes (
            id INTEGER PRIMARY KEY,
            library_id INTEGER REFERENCES libraries(id),
            name TEXT, super_class TEXT, is_abstract BOOLEAN,
            is_sealed BOOLEAN, is_mixin BOOLEAN
        );
        CREATE TABLE IF NOT EXISTS functions (
            id INTEGER PRIMARY KEY,
            class_id INTEGER REFERENCES classes(id),
            name TEXT, kind TEXT, is_async BOOLEAN, is_static BOOLEAN,
            return_type TEXT, dart_code TEXT, ir_json TEXT, cfg_json TEXT
        );
        CREATE TABLE IF NOT EXISTS fields (
            id INTEGER PRIMARY KEY,
            class_id INTEGER REFERENCES classes(id),
            name TEXT, type TEXT, is_static BOOLEAN, is_final BOOLEAN
        );
        CREATE TABLE IF NOT EXISTS strings (id INTEGER PRIMARY KEY, value TEXT, refs_count INTEGER);
        CREATE TABLE IF NOT EXISTS xrefs (
            from_func_id INTEGER REFERENCES functions(id),
            to_func_id INTEGER REFERENCES functions(id),
            call_type TEXT
        );
        CREATE TABLE IF NOT EXISTS security_findings (
            id INTEGER PRIMARY KEY,
            type TEXT, severity TEXT, description TEXT,
            function_id INTEGER, string_id INTEGER
        );
        CREATE INDEX IF NOT EXISTS idx_classes_name ON classes(name);
        CREATE INDEX IF NOT EXISTS idx_functions_name ON functions(name);
        CREATE INDEX IF NOT EXISTS idx_strings_value ON strings(value);
        CREATE INDEX IF NOT EXISTS idx_xrefs_from ON xrefs(from_func_id);
        CREATE INDEX IF NOT EXISTS idx_xrefs_to ON xrefs(to_func_id);
    ")?;
    
    Ok(conn)
}

pub fn write_meta(conn: &Connection, meta: &OutputMeta) -> Result<(), rusqlite::Error> {
    conn.execute("INSERT OR REPLACE INTO meta VALUES (?1, ?2)", params!["tool", &meta.tool])?;
    conn.execute("INSERT OR REPLACE INTO meta VALUES (?1, ?2)", params!["version", &meta.version])?;
    conn.execute("INSERT OR REPLACE INTO meta VALUES (?1, ?2)", params!["timestamp", &meta.timestamp])?;
    conn.execute("INSERT OR REPLACE INTO meta VALUES (?1, ?2)", params!["input_file", &meta.input_file])?;
    conn.execute("INSERT OR REPLACE INTO meta VALUES (?1, ?2)", params!["input_sha256", &meta.input_sha256])?;
    conn.execute("INSERT OR REPLACE INTO meta VALUES (?1, ?2)", params!["architecture", &meta.architecture])?;
    Ok(())
}
