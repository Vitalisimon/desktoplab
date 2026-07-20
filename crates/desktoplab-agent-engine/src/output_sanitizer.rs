pub(crate) fn sanitize_model_output(output: &str) -> String {
    let mut sanitized = String::new();
    let mut chars = output.chars().peekable();

    while let Some(character) = chars.next() {
        if character == '\x1b' {
            skip_escape_sequence(&mut chars);
            continue;
        }
        if character == '\x08' {
            sanitized.pop();
            continue;
        }
        if character.is_control() && character != '\n' && character != '\t' {
            continue;
        }
        sanitized.push(character);
    }

    collapse_horizontal_whitespace(&sanitized)
}

fn skip_escape_sequence(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    if chars.peek() == Some(&'[') {
        chars.next();
        for character in chars.by_ref() {
            if ('@'..='~').contains(&character) {
                break;
            }
        }
    }
}

fn collapse_horizontal_whitespace(input: &str) -> String {
    let mut output = String::new();
    let mut previous_was_space = false;

    for character in input.trim().chars() {
        if character == ' ' || character == '\t' {
            if !previous_was_space {
                output.push(' ');
            }
            previous_was_space = true;
            continue;
        }
        previous_was_space = false;
        output.push(character);
    }

    output
}
