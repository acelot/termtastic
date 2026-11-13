use rusqlite::{functions::FunctionFlags, hooks::Action};
use rusqlite_migration::{M, Migrations};
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio_rusqlite_new::Connection;
use uuid::{NoContext, Timestamp, Uuid};

const MIGRATIONS: &[&str] = &[
    include_str!("migrations/0001.sql"),
    include_str!("migrations/0002.sql"),
    include_str!("migrations/0003.sql"),
    include_str!("migrations/0004.sql"),
];

pub async fn create_sqlite_repository<'a>(
    file_path: PathBuf,
    node_key: u32,
    db_tx: mpsc::Sender<()>,
) -> Result<Repository, anyhow::Error> {
    let conn = Connection::open(file_path).await?;

    conn.call(move |conn| {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        conn.create_scalar_function("uuid_v7", 0, FunctionFlags::SQLITE_UTF8, |_| {
            Ok(Uuid::now_v7().as_bytes().to_vec())
        })?;

        conn.create_scalar_function("uuid_v7", 1, FunctionFlags::SQLITE_UTF8, |ctx| {
            let timestamp = ctx.get::<i64>(0)?;

            Ok(Uuid::new_v7(Timestamp::from_unix(NoContext, timestamp as u64, 0))
                .as_bytes()
                .to_vec())
        })?;

        conn.update_hook(Some(move |_action: Action, _db: &str, _table: &str, _rowid: i64| {
            let _ = db_tx.try_send(());
        }))?;

        let migrations: Vec<String> = MIGRATIONS
            .iter()
            .map(|s| s.replace(":node_key", &node_key.to_string()))
            .collect();

        let migrations = Migrations::from_iter(migrations.iter().map(|sql| M::up(sql)));

        migrations.to_latest(conn)
    })
    .await?;

    Ok(Repository::new(conn))
}

#[derive(thiserror::Error, Debug)]
pub enum RepositoryError {
    #[error("connection closed")]
    ConnectionClosed,
    #[error("connection is closing: {1}")]
    Close(Connection, rusqlite::Error),
    #[error("db error: {0}")]
    DbError(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    SerializationError(#[from] serde_rusqlite::Error),
}

impl<E> From<tokio_rusqlite_new::Error<E>> for RepositoryError
where
    RepositoryError: From<E>,
{
    fn from(err: tokio_rusqlite_new::Error<E>) -> Self {
        match err {
            tokio_rusqlite_new::Error::ConnectionClosed => Self::ConnectionClosed,
            tokio_rusqlite_new::Error::Close((conn, e)) => Self::Close(conn, e),
            tokio_rusqlite_new::Error::Error(e) => e.into(),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone)]
pub struct Repository {
    pub conn: Connection,
}

impl<'a> Repository {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }
}
