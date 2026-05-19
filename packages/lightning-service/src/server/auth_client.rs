use crate::client::error::LightningError;
use crate::client::models::{AuthAction, PlayerAuthSession, SetupProfile};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LnurlAuthCallbackPayload {
    pub challenge_id: String,
    pub public_key: String,
    pub signature: String,
    pub action: AuthAction,
}

pub trait AuthClient {
    fn begin_player_auth(
        &self,
        profile: &SetupProfile,
        action: AuthAction,
    ) -> Result<PlayerAuthSession, LightningError>;

    fn verify_callback(
        &self,
        session: &PlayerAuthSession,
        payload: &LnurlAuthCallbackPayload,
    ) -> Result<(), LightningError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LocalAuthClient;

impl AuthClient for LocalAuthClient {
    fn begin_player_auth(
        &self,
        profile: &SetupProfile,
        action: AuthAction,
    ) -> Result<PlayerAuthSession, LightningError> {
        crate::client::lab_service::begin_player_auth(profile, action)
    }

    fn verify_callback(
        &self,
        session: &PlayerAuthSession,
        payload: &LnurlAuthCallbackPayload,
    ) -> Result<(), LightningError> {
        if session.challenge_id != payload.challenge_id || session.action != payload.action {
            return Err(LightningError::AuthServiceUnavailable);
        }

        Ok(())
    }
}
