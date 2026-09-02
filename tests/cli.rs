use std::process::Command;

fn pi_calc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pi-calc"))
}

#[test]
fn prints_the_requested_number_of_decimal_digits() {
    let output = pi_calc()
        .args(["--quiet", "20"])
        .output()
        .expect("pi-calc should run");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "3.14159265358979323846\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn rejects_out_of_range_digit_counts() {
    let output = pi_calc().arg("0").output().expect("pi-calc should run");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("between 1 and 100,000,000"));
}

#[test]
fn rejects_unexpected_arguments() {
    let output = pi_calc()
        .args(["10", "20"])
        .output()
        .expect("pi-calc should run");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unexpected argument '20'"));
}

#[test]
fn help_documents_the_supported_interface() {
    let output = pi_calc()
        .arg("--help")
        .output()
        .expect("pi-calc should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pi-calc [FLAGS] [DIGITS]"));
    assert!(stdout.contains("--quiet"));
}
