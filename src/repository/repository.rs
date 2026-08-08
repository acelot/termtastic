use rusqlite_migration::{M, Migrations};
use std::path::PathBuf;
use tokio_rusqlite_new::Connection;

const MIGRATIONS_LIST: &[M<'_>] = &[
    M::up(include_str!("migrations/0001.sql")),
    M::up(include_str!("migrations/0002.sql")),
];

const MIGRATIONS: Migrations<'_> = Migrations::from_slice(MIGRATIONS_LIST);

pub async fn create_sqlite_repository<'a>(file_path: PathBuf) -> Result<Repository, anyhow::Error> {
    let conn = Connection::open(file_path).await?;

    conn.call(|conn| {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        MIGRATIONS.to_latest(conn)
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
