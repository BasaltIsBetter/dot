use super::*;

#[test]
fn only_the_top_of_the_window_moves_it() {
    assert!(is_on_the_grip(PhysicalPosition::new(500.0, 31.0), 1.0));
    assert!(!is_on_the_grip(PhysicalPosition::new(500.0, 32.0), 1.0));
    assert!(is_on_the_grip(PhysicalPosition::new(500.0, 63.0), 2.0));
}

#[test]
fn wsl_alone_goes_without_a_frame() {
    let dressed = dressed(window_attributes("Dot"));

    assert_eq!(dressed.decorations, env::var_os(WSL_VARIABLE).is_none());
}
