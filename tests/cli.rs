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
fn rejects_zero_digit_count() {
    let output = pi_calc().arg("0").output().expect("pi-calc should run");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("digits must be positive"));
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

#[test]
fn preserves_leading_zeros_across_a_decimal_block_boundary() {
    let digits = 199_992;
    let output = pi_calc()
        .args(["--quiet", "199992"])
        .output()
        .expect("pi-calc should run");

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout.len(), digits + 3);
    let fractional = &output.stdout[2..2 + digits];
    // The right-hand block starts with two zero digits. Values checked
    // against MPFR's independently computed pi constant.
    assert_eq!(&fractional[100_004..100_028], b"002437968454377733902647");
    assert!(output
        .stdout
        .ends_with(b"13309702304687661585748313508014\n"));
}
