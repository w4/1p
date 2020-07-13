//! Handles OTP code generation
use std::convert::TryFrom;
use url::Url;

pub enum TwoFactorAuth {
    Totp(libreauth::oath::TOTP),
}

pub struct TwoFactorAuthResponse {
    pub value: String,
}

impl TwoFactorAuth {
    pub fn generate(&self) -> TwoFactorAuthResponse {
        match &self {
            TwoFactorAuth::Totp(inner) => TwoFactorAuthResponse {
                value: inner.generate(),
            },
        }
    }
}

impl TryFrom<&str> for TwoFactorAuth {
    type Error = ();

    fn try_from(key: &str) -> Result<TwoFactorAuth, ()> {
        if let Ok(uri) = url::Url::parse(&key) {
            if let Ok(tfa) = Self::try_from(uri) {
                return Ok(tfa);
            }
        }

        Ok(TwoFactorAuth::Totp(
            libreauth::oath::TOTPBuilder::new()
                .base32_key(&key.replace(" ", ""))
                .finalize()
                .unwrap(),
        ))
    }
}

impl TryFrom<Url> for TwoFactorAuth {
    type Error = ();

    fn try_from(url: Url) -> Result<TwoFactorAuth, ()> {
        if url.scheme() != "otpauth" {
            return Err(());
        }

        if url.host_str() != Some("totp") {
            return Err(());
        }

        let mut query = url.query_pairs();

        let mut builder = &mut libreauth::oath::TOTPBuilder::new();

        if let Some(secret) = query.find(|v| v.0 == "secret") {
            builder = builder.base32_key(&secret.1);
        }

        if let Some(digits) = query.find(|v| v.0 == "digits") {
            builder = builder.output_len(digits.1.parse().map_err(|_| ())?);
        }

        if let Some(algorithm) = query.find(|v| v.0 == "algorithm") {
            builder = builder.hash_function(match algorithm.1.as_ref() {
                "sha1" => libreauth::hash::HashFunction::Sha1,
                "sha256" => libreauth::hash::HashFunction::Sha256,
                "sha512" => libreauth::hash::HashFunction::Sha512,
                _ => return Err(()),
            });
        }

        if let Some(period) = query.find(|v| v.0 == "period") {
            builder = builder.period(period.1.parse().map_err(|_| ())?);
        }

        Ok(TwoFactorAuth::Totp(builder.finalize().unwrap()))
    }
}
