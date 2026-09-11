use std::fmt::Display;
use std::ops::Deref;
use std::str::FromStr;

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash};
use axum::Json;
use axum::response::IntoResponse;
use serde::de::Visitor;
use serde::{Deserialize, Serialize};
use toasty::stmt::{Expr, IntoExpr};
use validator::ValidateEmail;

/// The JSON-serialized part of an error response.
///
/// Do not return this struct directly from handlers. Use
/// [`Error`](super::errors::Error) instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorSchema {
    pub title: String,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub struct Page<T> {
    items: Vec<T>,
    meta: PageMeta,
}

impl<T> Page<T> {
    pub fn new(items: impl IntoIterator<Item = T>, total: u64, cursor: u64) -> Self {
        Self {
            items: items.into_iter().collect(),
            meta: PageMeta { total, cursor },
        }
    }
}

impl<T> IntoResponse for Page<T>
where
    T: Serialize,
{
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}

#[derive(Serialize, Deserialize)]
pub struct PageMeta {
    total: u64,
    cursor: u64,
}

/// A validated username.
///
/// Usernames held by this struct will:
/// * Contain at least 2 characters.
/// * Contain no more than 32 characters.
/// * Contain only 7-bit ASCII alphanumerics, dots, and/or underscores.
/// * Contain at least one ASCII alphanumeric.
///
/// To initialize this struct, use `parse()` or `from_str()`, like follows:
///
/// ```
/// let username: Username = "nyeki".parse()?;
/// let username: Username = Username::from_str("nyeki")?;
/// ```
#[derive(Debug, Clone)]
pub struct Username(String);

impl Deref for Username {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Username {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Username {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !(2..=32).contains(&s.len()) {
            anyhow::bail!(
                "The username had either less than 2 or over 32 characters. Fit the username within those bounds."
            );
        }

        let mut has_alphanumeric = false;
        for ch in s.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '.' && ch != '_' {
                anyhow::bail!("Usernames can only contain ASCII letters, ");
            }

            if ch.is_ascii_alphanumeric() {
                has_alphanumeric = true;
            }
        }

        if !has_alphanumeric {
            anyhow::bail!(
                "Usernames must have at least one alphanumeric character (a number or a letter)."
            );
        }

        Ok(Username(s.to_string()))
    }
}

impl Serialize for Username {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Username {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct UsernameVisitor;

        impl<'de> Visitor<'de> for UsernameVisitor {
            type Value = Username;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(
                    formatter,
                    concat!(
                        "A valid username, consisting of only from 2 to 32 ASCII alphanumerics, ",
                        "dots, and/or underscores. At least one alphanumeric is required."
                    )
                )
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Username::from_str(v).map_err(|e| E::custom(e.to_string()))
            }
        }

        deserializer.deserialize_str(UsernameVisitor)
    }
}

impl IntoExpr<String> for Username {
    fn by_ref(&self) -> Expr<String> {
        self.0.by_ref()
    }

    fn into_expr(self) -> Expr<String> {
        self.0.into_expr()
    }
}

impl IntoExpr<String> for &Username {
    fn by_ref(&self) -> Expr<String> {
        self.0.by_ref()
    }

    fn into_expr(self) -> Expr<String> {
        self.0.clone().into_expr()
    }
}

/// A validated email address.
///
/// Email addresses held by this struct will:
/// * Be compliant with the [HTML5 spec].
/// * Not contain `<` nor `>`.
/// * Not contain `+`.
///
/// To initialize this struct, use `parse()` or `from_str()`, like follows:
///
/// ```
/// let email: Email = "alex@example.com".parse()?;
/// let email: Email = Email::from_str("alex@example.com")?;
/// ```
///
/// [HTML5 spec]: https://html.spec.whatwg.org/multipage/forms.html#valid-e-mail-address
#[derive(Debug, Clone)]
pub struct Email(String);

impl Deref for Email {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for Email {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.validate_email() {
            anyhow::bail!("The string provided (`{s}`) was not a valid email.");
        }

        if s.contains('<') || s.contains('>') {
            anyhow::bail!(
                "The email must not have a name, i.e. not `Alex <alex@example.com>`. The email provided was `{s}`."
            );
        }

        if s.contains('+') {
            anyhow::bail!(
                "Plus signs are not allowed in email addresses. The email you provided, `{s}`, had one or more of them."
            );
        }

        Ok(Email(s.to_string()))
    }
}

impl Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for Email {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Email {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct EmailVisitor;

        impl<'de> Visitor<'de> for EmailVisitor {
            type Value = Email;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(
                    formatter,
                    concat!(
                        "A supported email format, without name (i.e. not Alex ",
                        "<alex@example.com>) and no plus (i.e. not alex+123@example.com)."
                    )
                )
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Email::from_str(v).map_err(|e| E::custom(e.to_string()))
            }
        }

        deserializer.deserialize_str(EmailVisitor)
    }
}

impl IntoExpr<String> for Email {
    fn by_ref(&self) -> Expr<String> {
        self.0.by_ref()
    }

    fn into_expr(self) -> Expr<String> {
        self.0.into_expr()
    }
}

impl IntoExpr<String> for &Email {
    fn by_ref(&self) -> Expr<String> {
        self.0.by_ref()
    }

    fn into_expr(self) -> Expr<String> {
        self.0.clone().into_expr()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Login {
    Email(Email),
    Username(Username),
}

/// A zxcvbn-validated password scoring 3 or higher.
///
/// To initialize this struct, use `parse()` or `from_str()`, like follows:
///
/// ```
/// let password: Password = "jernfkjnefkjenfkerjnfekjwefnk".parse()?;
/// let password: Password = Password::from_str("wkjenjknkjnkj212nnkj3nkjn")?;
/// ```
#[derive(Debug, Clone)]
pub struct Password(String);

impl Password {
    /// Hashes the password and returns the hashed string.
    ///
    /// Returns:
    /// [`String`] - The Argon2id-hashed password, e.g. `$argon2id$v=19$...`.
    pub async fn hash(&self) -> String {
        let password = self.0.clone();

        tokio::task::spawn_blocking(move || {
            Argon2::default()
                .hash_password(password.as_bytes())
                .expect("Could not hash password.")
                .to_string()
        })
        .await
        .expect("Panicked while hashing password.")
    }

    /// Compares the current password with a hashed password string and returns
    /// whether they match.
    ///
    /// Arguments:
    /// * `hash` - The hash string, e.g. `$argon2id$v=19$...`. The value
    ///   returned by [`Password::hash()`].
    ///
    /// Returns:
    /// [`bool`] - Whether the password is the same as the one in the provided
    /// hash.
    pub async fn verify(&self, hash: impl Into<String>) -> bool {
        let hash = PasswordHash::new(&hash.into()).unwrap();
        let password = self.0.clone();

        let result = tokio::task::spawn_blocking(move || {
            // May fail if algorithms are unsupported. Since it's all internal,
            // that will never happen.
            Argon2::default().verify_password(password.as_bytes(), &hash)
        })
        .await
        .expect("Panicked while verifying password.");

        match result {
            Ok(()) => true,
            Err(error) => match error {
                password_hash::Error::PasswordInvalid => false,
                _ => unreachable!(
                    "Argon2 failed to verify the password for a reason that was not a mismatch."
                ),
            },
        }
    }
}

impl Deref for Password {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Password {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let entropy = zxcvbn::zxcvbn(s, &[]);
        let score: u8 = entropy.score().into();

        if score < 3 {
            anyhow::bail!("The password provided was too weak.");
        }

        Ok(Password(s.to_string()))
    }
}

impl Serialize for Password {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Password {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PasswordVisitor;

        impl<'de> Visitor<'de> for PasswordVisitor {
            type Value = Password;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "A strong password, scoring 3 or more in zxcvbn.")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Password::from_str(v).map_err(|e| E::custom(e.to_string()))
            }
        }

        deserializer.deserialize_str(PasswordVisitor)
    }
}
