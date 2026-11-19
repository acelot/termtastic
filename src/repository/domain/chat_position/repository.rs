use crate::repository::domain::chat_position::ChatPosition;
use crate::repository::{Repository, RepositoryError};
use serde_rusqlite::{from_rows, to_params_named};

impl Repository {
    pub async fn chat_position_find_all(&self) -> Result<Vec<ChatPosition>, RepositoryError> {
        let result: Vec<ChatPosition> = self
            .conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare("SELECT * FROM chat_position")?;

                from_rows::<ChatPosition>(statement.query([])?)
                    .collect::<Result<_, _>>()
                    .map_err(Into::into)
            })
            .await?;

        Ok(result)
    }

    pub async fn chat_position_upsert(&self, chat_position: ChatPosition) -> Result<(), RepositoryError> {
        self.conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(
                    r#"
                    INSERT INTO chat_position (
                        chat_id,
                        selected_id,
                        max_id
                    )
                    VALUES (
                        :chat_id,
                        :selected_id,
                        :max_id
                    )
                    ON CONFLICT(chat_id) DO UPDATE SET
                        selected_id = excluded.selected_id,
                        max_id = excluded.max_id
                    "#,
                )?;

                statement
                    .execute(to_params_named(&chat_position)?.to_slice().as_slice())
                    .map_err(Into::into)
            })
            .await?;

        Ok(())
    }
}
