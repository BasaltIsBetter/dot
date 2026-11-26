use super::*;

const NEAR: f64 = 0.01;

fn close(found: Option<[f64; 2]>, expected: [f64; 2]) -> bool {
    found.is_some_and(|[left, right]| {
        (left - expected[0]).abs() < NEAR && (right - expected[1]).abs() < NEAR
    })
}

#[test]
fn draws_the_head_as_a_disc_of_the_prototype() {
    let across_the_middle = span_at(HEAD_CENTRE);
    let near_the_top = span_at(0.4).unwrap();

    assert!(
        close(across_the_middle, [2.2, 6.8]),
        "{across_the_middle:?}"
    );
    assert!(
        near_the_top[0] > 3.6 && near_the_top[1] < 5.4,
        "{near_the_top:?}"
    );
    assert!((near_the_top[0] + near_the_top[1] - WIDTH).abs() < NEAR);
}

#[test]
fn leaves_nothing_above_the_head_nor_in_the_neck() {
    assert_eq!(span_at(0.2), None);
    assert_eq!(span_at(5.4), None);
    assert_eq!(span_at(-1.0), None);
}

#[test]
fn draws_the_shoulders_from_a_point_under_the_head_to_the_full_width() {
    let top = span_at(5.9);
    let bottom = span_at(HEIGHT);

    assert!(close(top, [MIDDLE, MIDDLE]), "{top:?}");
    assert!(close(bottom, [0.5, 8.5]), "{bottom:?}");
    assert_eq!(span_at(HEIGHT + 0.1), None);
}

#[test]
fn follows_the_curve_of_the_shoulders() {
    let downs = [6.0, 6.5, 7.0, 8.0, 9.0, 10.0];

    let halves: Vec<f64> = downs
        .iter()
        .map(|&down| {
            let [left, right] = span_at(down).unwrap();
            assert!((left + right - WIDTH).abs() < NEAR);
            MIDDLE - left
        })
        .collect();

    assert!(
        halves.windows(2).all(|pair| pair[0] < pair[1]),
        "{halves:?}"
    );
    assert!((halves[1] - 2.1).abs() < 0.1, "{halves:?}");
    assert!((halves[3] - 3.4).abs() < 0.1, "{halves:?}");
}
