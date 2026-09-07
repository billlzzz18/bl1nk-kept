//! Thai Kedmanee keyboard-layout conversion utilities.

// NOTE-001: ตารางแปลงปุ่ม QWERTY <-> ภาษาไทยเกษมณี (แบบ static const ไม่ต้องจอง heap runtime)
const QWERTY_TO_THAI: &[(char, char)] = &[
    ('1', '\u{0E45}'),
    ('2', '/'),
    ('3', '-'),
    ('4', '\u{0E20}'),
    ('5', '\u{0E16}'),
    ('6', '\u{0E38}'),
    ('7', '\u{0E36}'),
    ('8', '\u{0E04}'),
    ('9', '\u{0E15}'),
    ('0', '\u{0E08}'),
    ('-', '\u{0E02}'),
    ('=', '\u{0E0A}'),
    ('q', '\u{0E46}'),
    ('w', '\u{0E44}'),
    ('e', '\u{0E33}'),
    ('r', '\u{0E1E}'),
    ('t', '\u{0E30}'),
    ('y', '\u{0E31}'),
    ('u', '\u{0E39}'),
    ('i', '\u{0E35}'),
    ('o', '\u{0E19}'),
    ('p', '\u{0E22}'),
    ('[', '\u{0E2D}'),
    (']', '\u{0E2E}'),
    ('a', '\u{0E1D}'),
    ('s', '\u{0E01}'),
    ('d', '\u{0E14}'),
    ('f', '\u{0E40}'),
    ('g', '\u{0E49}'),
    ('h', '\u{0E48}'),
    ('j', '\u{0E32}'),
    ('k', '\u{0E2A}'),
    ('l', '\u{0E27}'),
    (';', '\u{0E07}'),
    ('\'', '\u{0E25}'),
    ('z', '\u{0E17}'),
    ('x', '\u{0E18}'),
    ('c', '\u{0E33}'),
    ('v', '\u{0E34}'),
    ('b', '\u{0E38}'),
    ('n', '\u{0E19}'),
    ('m', '\u{0E21}'),
];

fn map_qwerty_char(character: char) -> Option<char> {
    QWERTY_TO_THAI
        .iter()
        .find(|(q, _)| *q == character)
        .map(|(_, t)| *t)
}

fn map_thai_char(character: char) -> Option<char> {
    QWERTY_TO_THAI
        .iter()
        .find(|(_, t)| *t == character)
        .map(|(q, _)| *q)
}

/// Convert English-layout keystrokes to Thai Kedmanee characters.
pub fn qwerty_to_thai(text: &str) -> String {
    text.chars()
        .map(|character| {
            let lower = character.to_lowercase().next().unwrap_or(character);
            map_qwerty_char(lower).unwrap_or(character)
        })
        .collect()
}

/// Convert Thai Kedmanee characters to their English-layout keys.
pub fn thai_to_qwerty(text: &str) -> String {
    text.chars()
        .map(|character| map_thai_char(character).unwrap_or(character))
        .collect()
}

/// NOTE-001: ใช้สัดส่วน 70% เพื่อหลีกเลี่ยงการแปลงคำอังกฤษปกติที่มีตัวอักษรตรงกับ map บางส่วน
pub fn likely_thai_wrong_layout(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    // NOTE-001: ไม่แปลงคำที่มีเครื่องหมายคั่น เช่น api-docs เพราะมักเป็นภาษาอังกฤษที่ถูกต้อง
    if chars.is_empty()
        || chars
            .iter()
            .any(|character| !character.is_ascii_alphabetic())
    {
        return false;
    }
    let qwerty_chars = chars
        .iter()
        .filter(|character| {
            let lower = character.to_lowercase().next().unwrap_or(**character);
            map_qwerty_char(lower).is_some()
        })
        .count();
    (qwerty_chars as f64 / chars.len() as f64) > 0.7
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_sawasdee() {
        assert_eq!(qwerty_to_thai("klykdi"), "สวัสดี");
    }

    #[test]
    fn reverses_sawasdee() {
        assert_eq!(thai_to_qwerty("สวัสดี"), "klykdi");
    }

    #[test]
    fn detects_wrong_layout() {
        assert!(likely_thai_wrong_layout("klykdi"));
        assert!(!likely_thai_wrong_layout("สวัสดี"));
        assert!(!likely_thai_wrong_layout("api-docs"));
    }
}
