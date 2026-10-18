#[cfg(unix)]
#[test]
fn shell_integrations_pass_the_config_path_without_literal_quotes() {
    use std::fs;
    use std::process::Command;

    for (shell, integration) in [
        (
            "bash",
            include_str!("../modules/oh-my-posh/bash/10-oh-my-posh.bash"),
        ),
        (
            "zsh",
            include_str!("../modules/oh-my-posh/zsh/10-oh-my-posh.zsh"),
        ),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home with spaces");
        let integration_path = temp.path().join("integration.sh");
        let captured_args = temp.path().join("args");
        fs::write(&integration_path, integration).unwrap();

        let output = Command::new("bash")
            .arg("-c")
            .arg(
                r#"
                oh-my-posh() {
                    printf '%s\0' "$@" > "$OMP_CAPTURE"
                    printf ':'
                }
                source "$1"
                "#,
            )
            .arg("test-shell")
            .arg(&integration_path)
            .env("HOME", &home)
            .env("OMP_CAPTURE", &captured_args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{shell} integration failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let actual: Vec<_> = fs::read(captured_args)
            .unwrap()
            .split(|byte| *byte == 0)
            .filter(|argument| !argument.is_empty())
            .map(|argument| String::from_utf8(argument.to_vec()).unwrap())
            .collect();
        let expected = vec![
            "init".to_string(),
            shell.to_string(),
            "--config".to_string(),
            home.join(".config/oh-my-posh/config.json")
                .to_string_lossy()
                .into_owned(),
        ];
        assert_eq!(actual, expected);
    }
}
