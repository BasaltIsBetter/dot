const RULERS: [char; 3] = ['─', '━', '═'];
const FIRST_BYTE_OF_A_RULER: u8 = 0xE2;
const LONGEST_KEPT: usize = 3;

pub(crate) fn shorten(text: &str) -> Option<String> {
    if !text.as_bytes().contains(&FIRST_BYTE_OF_A_RULER) {
        return None;
    }

    let mut shorter = String::with_capacity(text.len());
    let mut previous = None;
    let mut run = 0_usize;
    for character in text.chars() {
        run = if previous == Some(character) {
            run + 1
        } else {
            1
        };
        previous = Some(character);

        let lengthens_a_ruler = run > LONGEST_KEPT && RULERS.contains(&character);
        if !lengthens_a_ruler {
            shorter.push(character);
        }
    }
    (shorter.len() < text.len()).then_some(shorter)
}

#[cfg(test)]
#[path = "../../../../tests/dot-compact/noise/ruler.rs"]
mod tests;
