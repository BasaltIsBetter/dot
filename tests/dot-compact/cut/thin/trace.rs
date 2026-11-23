use super::*;

const JAVA: &str = "java.lang.IllegalStateException: no\n\tat a.B.c(B.java:1)\n\tat a.B.d(B.java:2)\n\tat a.B.e(B.java:3)\n\tat a.B.f(B.java:4)\n\tat a.B.g(B.java:5)\n\tat a.B.h(B.java:6)\n\tat a.B.i(B.java:7)\nCaused by: x\n";
const PYTHON: &str = "Traceback (most recent call last):\n  File \"a.py\", line 1, in <module>\n    main()\n  File \"a.py\", line 2, in main\n    run()\n  File \"b.py\", line 3, in run\n    step()\n  File \"c.py\", line 4, in step\n    raise ValueError\nValueError: no\n";

#[test]
fn keeps_both_ends_of_a_long_stack_and_counts_the_rest() {
    assert_eq!(
        frames(JAVA).as_deref(),
        Some(
            "java.lang.IllegalStateException: no\n\tat a.B.c(B.java:1)\n\tat a.B.d(B.java:2)\n[+3 lines of the stack]\n\tat a.B.h(B.java:6)\n\tat a.B.i(B.java:7)\nCaused by: x\n"
        )
    );
    assert_eq!(
        frames(PYTHON).as_deref(),
        Some(
            "Traceback (most recent call last):\n  File \"a.py\", line 1, in <module>\n    main()\n[+4 lines of the stack]\n  File \"c.py\", line 4, in step\n    raise ValueError\nValueError: no\n"
        )
    );
}

#[test]
fn leaves_alone_a_stack_of_six_lines_and_what_is_not_one() {
    let six = "error\n  at a\n  at b\n  at c\n  at d\n  at e\n  at f\nend";
    let seven = "error\n  at a\n  at b\n  at c\n  at d\n  at e\n  at f\n  at g\nend";
    let not_indented = "at a\nat b\nat c\nat d\nat e\nat f\nat g\nat h\n";
    let indented_text = "  one\n  two\n  three\n  four\n  five\n  six\n  seven\n  eight\n";

    assert_eq!(frames(six), None);
    assert_eq!(
        frames(seven).as_deref(),
        Some("error\n  at a\n  at b\n[+3 lines of the stack]\n  at f\n  at g\nend")
    );
    assert_eq!(frames(not_indented), None);
    assert_eq!(frames(indented_text), None);
    assert_eq!(frames(""), None);
}

#[test]
fn a_line_of_code_belongs_to_the_python_frame_above_it_only() {
    let lines = [
        "  File \"a.py\", line 1, in f",
        "    code()",
        "    more()",
        "File \"a.py\"",
    ];

    assert!(is_of_a_stack(&lines, 0));
    assert!(is_of_a_stack(&lines, 1));
    assert!(!is_of_a_stack(&lines, 2));
    assert!(!is_of_a_stack(&lines, 3));
}
