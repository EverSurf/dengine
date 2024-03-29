use super::helpers::{TonClient, HD_PATH};
use ton_client::crypto::{
    register_encryption_box, remove_encryption_box,
    EncryptionBoxHandle, RegisteredEncryptionBox, ChaCha20ParamsEB, ChaCha20EncryptionBox,
    NaclBoxParamsEB, NaclEncryptionBox, NaclSecretBoxParamsEB, NaclSecretEncryptionBox
};

#[derive(Clone, Copy)]
pub(crate) enum EncryptionBoxType {
    SecretNaCl,
    NaCl,
    ChaCha20,
}

pub(crate) struct ParamsOfTestEncryptionBox {
    pub box_type: EncryptionBoxType,
    pub their_pubkey: String,
    pub nonce: String,
    pub context: TonClient,
}

pub(super) struct TestEncryptionBox {
    pub handle: EncryptionBoxHandle,
    pub client: TonClient,
}

impl Drop for TestEncryptionBox {
    fn drop(&mut self) {
        if self.handle.0 != 0 {
            let _ = remove_encryption_box(
                self.client.clone(),
                RegisteredEncryptionBox {
                    handle: self.handle(),
                },
            );
        }
    }
}

impl TestEncryptionBox {
    pub async fn new(params: ParamsOfTestEncryptionBox, key: String) -> Result<Self, String> {

        let registered_box = match params.box_type {
            EncryptionBoxType::SecretNaCl => {
                register_encryption_box(
                    params.context.clone(),
                    NaclSecretEncryptionBox::new(NaclSecretBoxParamsEB {
                        key,
                        nonce: params.nonce,
                    },
                    Some(HD_PATH.to_owned()))
                )
                .await.map_err(|e| e.to_string())?.handle
            },
            EncryptionBoxType::NaCl => {
                register_encryption_box(
                    params.context.clone(),
                    NaclEncryptionBox::new(NaclBoxParamsEB {
                        their_public: params.their_pubkey,
                        secret: key,
                        nonce: params.nonce,
                    },
                    Some(HD_PATH.to_owned()))
                )
                .await.map_err(|e| e.to_string())?.handle
            },
            EncryptionBoxType::ChaCha20 => {
                register_encryption_box(
                    params.context.clone(),
                    ChaCha20EncryptionBox::new(
                        ChaCha20ParamsEB {
                            key,
                            nonce: params.nonce,
                        },
                        Some(HD_PATH.to_owned())
                    ).map_err(|e| e.to_string())?
                )
                .await.map_err(|e| e.to_string())?.handle
            },
        };
        Ok(Self {
            handle: registered_box,
            client: params.context.clone(),
        })
    }
    pub fn handle(&self) -> EncryptionBoxHandle {
        self.handle.clone()
    }
}

