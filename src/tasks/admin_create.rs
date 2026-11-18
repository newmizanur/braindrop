//! Create or repair an administrator account without going through public
//! registration.
//!
//! Public registration is disabled by default, so a fresh installation needs a
//! non-interactive way to create the first account. This task is intentionally
//! separate from `user:create`: it defaults to a verified account, never sends
//! verification mail unless asked to, and can idempotently reset an existing
//! account when `force:true` is passed.

use crate::{
    mailers::auth::AuthMailer,
    models::{_entities::users, users::RegisterParams},
};
use loco_rs::prelude::*;

pub struct AdminCreate;

/// Parse a boolean task argument, accepting the spellings an operator is
/// likely to type in a shell.
fn flag(vars: &task::Vars, name: &str, default: bool) -> bool {
    vars.cli_arg(name)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "true" | "1" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

/// Apply the requested verification state to an account.
///
/// A verified account gets a verification timestamp. An unverified account is
/// explicitly cleared (relevant when resetting an existing account), issued a
/// fresh verification token and sent the welcome mail.
async fn apply_verification(
    app_context: &AppContext,
    user: users::Model,
    verified: bool,
) -> Result<users::Model> {
    let email = user.email.clone();

    if verified {
        return user
            .into_active_model()
            .verified(&app_context.db)
            .await
            .map_err(|err| {
                tracing::error!(
                    message = err.to_string(),
                    user_email = email,
                    "could not verify administrator"
                );
                Error::string("failed to mark the account as verified")
            });
    }

    let mut active = user.into_active_model();
    active.email_verified_at = ActiveValue::Set(None);
    let user = active
        .set_email_verification_sent(&app_context.db)
        .await
        .map_err(|err| {
            tracing::error!(
                message = err.to_string(),
                user_email = email,
                "could not issue verification token"
            );
            Error::string("failed to issue an email verification token")
        })?;

    AuthMailer::send_welcome(app_context, &user)
        .await
        .map_err(|err| {
            tracing::error!(
                message = err.to_string(),
                user_email = user.email,
                "could not send welcome email"
            );
            Error::string("failed to send the welcome email")
        })?;

    Ok(user)
}

#[async_trait]
impl Task for AdminCreate {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "admin:create".to_string(),
            detail: "Create the initial administrator or another account without public registration.\n\
                     Accounts are verified by default. Pass verified:false to require email verification, \
                     and force:true to reset an existing account's password.\n\
                     Usage:\n\
                     braindrop-cli task admin:create email:admin@example.com name:\"Admin\" password:\"change-me\"\n\
                     braindrop-cli task admin:create email:admin@example.com name:\"Admin\" password:\"new-secret\" force:true"
                .to_string(),
        }
    }

    async fn run(&self, app_context: &AppContext, vars: &task::Vars) -> Result<()> {
        let email = vars
            .cli_arg("email")
            .map_err(|_| Error::string("email is mandatory"))?;
        let name = vars
            .cli_arg("name")
            .map_err(|_| Error::string("name is mandatory"))?;
        let password = vars
            .cli_arg("password")
            .map_err(|_| Error::string("password is mandatory"))?;
        let verified = flag(vars, "verified", true);
        let force = flag(vars, "force", false);

        if password.trim().is_empty() {
            return Err(Error::string("password must not be empty"));
        }

        let existing = users::Model::find_by_email(&app_context.db, email).await;

        let user = match existing {
            Ok(user) => {
                if !force {
                    return Err(Error::string(&format!(
                        "user {email} already exists; pass force:true to reset the account"
                    )));
                }

                let mut active = user.into_active_model();
                active.name = ActiveValue::Set(name.to_owned());
                let user = active
                    .reset_password(&app_context.db, password)
                    .await
                    .map_err(|err| {
                        tracing::error!(
                            message = err.to_string(),
                            user_email = email,
                            "could not reset administrator password"
                        );
                        Error::string("failed to reset the existing account")
                    })?;

                apply_verification(app_context, user, verified).await?
            }
            Err(ModelError::EntityNotFound) => {
                let params = RegisterParams {
                    email: email.to_owned(),
                    password: password.to_owned(),
                    name: name.to_owned(),
                };
                let user = users::Model::create_with_password(&app_context.db, &params)
                    .await
                    .map_err(|err| {
                        tracing::error!(
                            message = err.to_string(),
                            user_email = email,
                            "could not create administrator"
                        );
                        Error::string("failed to create the account")
                    })?;

                apply_verification(app_context, user, verified).await?
            }
            Err(err) => return Err(err.into()),
        };

        println!("Account ready.");
        println!("  Email:    {}", user.email);
        println!("  Name:     {}", user.name);
        println!("  PID:      {}", user.pid);
        println!(
            "  Verified: {}",
            if user.email_verified_at.is_some() {
                "yes"
            } else {
                "no (verification email sent)"
            }
        );

        tracing::info!(
            user_email = user.email,
            user_pid = user.pid.to_string(),
            force,
            verified,
            "administrator task completed"
        );

        Ok(())
    }
}
