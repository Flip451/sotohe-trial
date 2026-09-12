//! Web binary entry point.
//!
//! The outermost layer. It obtains a fully wired primary
//! adapter from the composition root, and presents the resulting
//! [`CommandOutcome`] on stdout/stderr. It holds no business logic of its own.

use std::process::ExitCode;

use interface_adapters::CommandOutcome;
use web_composition::auth::AuthCompositionRoot;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let composition_root = AuthCompositionRoot::new();
    let server_outcome = composition_root.auth_server().serve().await;
    let outcome = CommandOutcome {
        message: server_outcome.message().to_owned(),
        success: server_outcome.is_success(),
    };
    present(outcome)
}

/// Prints the command result and maps it to a process exit code.
fn present(outcome: CommandOutcome) -> ExitCode {
    if outcome.success {
        println!("{}", outcome.message);
        ExitCode::SUCCESS
    } else {
        eprintln!("error: {}", outcome.message);
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_present_success_outcome_returns_success_exit_code() {
        let outcome = CommandOutcome { message: "Hello, ada!".to_owned(), success: true };

        assert_eq!(present(outcome), ExitCode::SUCCESS);
    }

    #[test]
    fn test_present_failure_outcome_returns_failure_exit_code() {
        let outcome =
            CommandOutcome { message: "username must not be empty".to_owned(), success: false };

        assert_eq!(present(outcome), ExitCode::FAILURE);
    }
}
