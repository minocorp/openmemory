use bytes::Bytes;
use openmemory_ops::PagePath;
use rand::RngExt;
use time::OffsetDateTime;

use crate::access::{Actor, Folders, Scope};
use crate::files::link_queries::{self, ConnectionRow};
use crate::files::{FileContent, FileRecord, Files, FilesError};
use crate::storage::Sha256Hash;

const LINK_LIFETIME_MINUTES: i32 = 15;
const EVERYTHING: &str = "*";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UploadLink {
    pub(crate) token: String,
    pub(crate) expires_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpenUploadLink {
    pub(crate) path: PagePath,
    pub(crate) expires_at: OffsetDateTime,
}

impl Files {
    pub(crate) async fn create_upload_link(
        &self,
        actor: &Actor,
        path: &str,
    ) -> Result<UploadLink, FilesError> {
        let path = PagePath::parse(path)?;
        let connection_id = actor.connection_id.ok_or(FilesError::ConnectionRequired)?;
        if !actor.scope.can_write(&path) {
            return Err(FilesError::Forbidden);
        }
        let token = Sha256Hash::from_bytes(rand::rng().random()).to_hex();
        let mut conn = self.pool.acquire().await?;
        let expires_at = link_queries::insert_link(
            &mut conn,
            &Sha256Hash::of(token.as_bytes()),
            path.as_str(),
            connection_id,
            LINK_LIFETIME_MINUTES,
        )
        .await?;
        Ok(UploadLink { token, expires_at })
    }

    pub(crate) async fn open_upload_link(&self, token: &str) -> Result<OpenUploadLink, FilesError> {
        let mut conn = self.pool.acquire().await?;
        let open = link_queries::find_open_link(&mut conn, &Sha256Hash::of(token.as_bytes()))
            .await?
            .ok_or(FilesError::LinkInvalid)?;
        Ok(OpenUploadLink {
            path: PagePath::parse(&open.path).map_err(|_| FilesError::StoredRowInvalid)?,
            expires_at: open.expires_at,
        })
    }

    pub(crate) async fn redeem_upload_link(
        &self,
        token: &str,
        bytes: Bytes,
        mime: &str,
    ) -> Result<FileRecord, FilesError> {
        let mut conn = self.pool.acquire().await?;
        let claimed = link_queries::claim_link(&mut conn, &Sha256Hash::of(token.as_bytes()))
            .await?
            .ok_or(FilesError::LinkInvalid)?;
        let outcome = self.upload_for_connection(&claimed, bytes, mime).await;
        if outcome.is_err() {
            link_queries::release_link(&mut conn, claimed.id).await?;
        }
        outcome
    }

    async fn upload_for_connection(
        &self,
        claimed: &link_queries::ClaimedLink,
        bytes: Bytes,
        mime: &str,
    ) -> Result<FileRecord, FilesError> {
        let mut conn = self.pool.acquire().await?;
        let row = link_queries::find_live_connection(&mut conn, claimed.connection_id)
            .await?
            .ok_or(FilesError::LinkInvalid)?;
        drop(conn);
        let actor = actor_for(claimed.connection_id, row);
        self.upload(&actor, &claimed.path, FileContent::Bytes(bytes), mime)
            .await
    }
}

fn actor_for(connection_id: crate::access::ConnectionId, row: ConnectionRow) -> Actor {
    let folders = if row.scopes.iter().any(|scope| scope == EVERYTHING) {
        Folders::All
    } else {
        Folders::Only(
            row.scopes
                .iter()
                .filter_map(|scope| PagePath::parse(scope).ok())
                .collect(),
        )
    };
    Actor {
        user_id: row.user_id,
        connection_id: Some(connection_id),
        name: format!("{}/{}", row.user_name, row.name),
        scope: Scope {
            folders,
            can_write: row.can_write,
        },
    }
}
