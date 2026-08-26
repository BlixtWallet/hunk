use hunk_domain::config::{TerminalConfig, TerminalShell};
use hunk_terminal::resolve_terminal_shell;

#[test]
fn explicit_program_shell_resolution_preserves_program_and_defaults_args() {
    let config = TerminalConfig {
        shell: TerminalShell::Program("/bin/zsh".to_string()),
        ..TerminalConfig::default()
    };

    let resolved = resolve_terminal_shell(&config);

    assert_eq!(resolved.program(), "/bin/zsh");
    assert_eq!(resolved.label(), "zsh");
    assert_eq!(resolved.interactive_shell_args(true), ["-l", "-i"]);
}

#[test]
fn explicit_shell_args_are_preserved() {
    let config = TerminalConfig {
        shell: TerminalShell::WithArguments {
            program: "pwsh.exe".to_string(),
            args: vec!["-NoLogo".to_string()],
        },
        ..TerminalConfig::default()
    };

    let resolved = resolve_terminal_shell(&config);

    assert_eq!(resolved.program(), "pwsh.exe");
    assert_eq!(resolved.label(), "pwsh.exe");
    assert_eq!(resolved.interactive_shell_args(true), ["-NoLogo"]);
}

#[test]
fn powershell_interactive_args_honor_profile_opt_out() {
    let config = TerminalConfig {
        shell: TerminalShell::Program("pwsh.exe".to_string()),
        ..TerminalConfig::default()
    };
    let resolved = resolve_terminal_shell(&config);

    assert_eq!(resolved.interactive_shell_args(true), ["-NoLogo"]);
    assert_eq!(
        resolved.interactive_shell_args(false),
        ["-NoLogo", "-NoProfile"]
    );
}
