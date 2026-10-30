use rusqlite::hooks::Action;
use rusqlite_migration::{M, Migrations};
use std::{iter, path::PathBuf};
use tokio::sync::mpsc;
use tokio_rusqlite_new::Connection;

const MIGRATIONS: &[&str] = &[
    include_str!("migrations/0001.sql"),
    include_str!("migrations/0002.sql"),
    include_str!("migrations/0003.sql"),
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

        conn.update_hook(Some(move |_action: Action, _db: &str, _table: &str, _rowid: i64| {
            let _ = db_tx.try_send(());
        }))?;

        let third_migration_sql = MIGRATIONS[2].replace(":node_key", &node_key.to_string());

        let migrations = Migrations::from_iter(
            MIGRATIONS[..2]
                .iter()
                .chain(iter::once(&third_migration_sql.as_str()))
                .chain(MIGRATIONS.get(3..).unwrap_or_default().iter())
                .map(|sql| M::up(sql)),
        );

        migrations.to_latest(conn)
    })
    .await?;

    Ok(Repository::new(conn))
}

#[derive(thiserror::Error, Debug)]
pub enum RepositoryError {
    #[error("connection closed")]
    ConnectionClosed,
    #[error("connection is closing")]
    Close(Connection, rusqlite::Error),
    #[error("db error: {0}")]
    DbError(#[from] rusqlite::Error),
    #[error("serialization error")]
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
