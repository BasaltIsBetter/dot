use std::io;

use super::*;

#[test]
fn writes_an_id_as_sixteen_hexadecimal_digits() {
    assert_eq!(KeptId::new(0).to_string(), "0000000000000000");
    assert_eq!(KeptId::new(u64::MAX).to_string(), "ffffffffffffffff");
    assert_eq!(KeptId::new(0xab).to_string(), "00000000000000ab");
}

#[test]
fn reads_back_every_id_it_writes() {
    for value in [0, 1, 0xab, 0x00a1_b2c3_d4e5_f607, u64::MAX] {
        let id = KeptId::new(value);

        assert_eq!(KeptId::parse(&id.to_string()), Some(id));
    }
}

#[test]
fn refuses_an_id_of_another_shape() {
    for written in [
        "",
        "00a1b2c3d4e5f60",
        "00a1b2c3d4e5f6070",
        "00A1B2C3D4E5F607",
        "00a1b2c3d4e5f60g",
        "+0a1b2c3d4e5f607",
        "../../../../etc/",
        "00a1b2c3d4e5f6é",
    ] {
        assert_eq!(KeptId::parse(written), None, "{written}");
    }
}

#[test]
fn an_error_says_its_cause() {
    let error = ArchiveError::from(io::Error::other("disk full"));

    assert_eq!(error.to_string(), "disk full");
}
