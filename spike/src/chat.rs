use serde_json::Value;

fn byte_to_hex(byte: i32) -> Option<&'static str> {
    Some(match byte {
        0 => "#000000",
        1 => "#0000AA",
        2 => "#00AA00",
        3 => "#00AAAA",
        4 => "#AA0000",
        5 => "#AA00AA",
        6 => "#FFAA00",
        7 => "#AAAAAA",
        8 => "#555555",
        9 => "#5555FF",
        10 => "#55FF55",
        11 => "#55FFFF",
        12 => "#FF5555",
        13 => "#FF55FF",
        14 => "#FFFF55",
        15 => "#FFFFFF",
        _ => return None,
    })
}

fn char_to_hex(c: char) -> Option<&'static str> {
    Some(match c.to_ascii_lowercase() {
        '0' => "#000000",
        '1' => "#0000AA",
        '2' => "#00AA00",
        '3' => "#00AAAA",
        '4' => "#AA0000",
        '5' => "#AA00AA",
        '6' => "#FFAA00",
        '7' => "#AAAAAA",
        '8' => "#555555",
        '9' => "#5555FF",
        'a' => "#55FF55",
        'b' => "#55FFFF",
        'c' => "#FF5555",
        'd' => "#FF55FF",
        'e' => "#FFFF55",
        'f' => "#FFFFFF",
        _ => return None,
    })
}

fn name_to_hex(name: &str) -> Option<&'static str> {
    Some(match name.to_ascii_lowercase().as_str() {
        "black" => "#000000",
        "dark_blue" => "#0000AA",
        "dark_green" => "#00AA00",
        "dark_aqua" => "#00AAAA",
        "dark_red" => "#AA0000",
        "dark_purple" => "#AA00AA",
        "gold" => "#FFAA00",
        "gray" => "#AAAAAA",
        "dark_gray" => "#555555",
        "blue" => "#5555FF",
        "green" => "#55FF55",
        "aqua" => "#55FFFF",
        "red" => "#FF5555",
        "light_purple" => "#FF55FF",
        "yellow" => "#FFFF55",
        "white" => "#FFFFFF",
        "orange" => "#FFA500",
        "pink" => "#FF55FF",
        "lime" => "#55FF55",
        "cyan" => "#55FFFF",
        "magenta" => "#FF55FF",
        "brown" => "#AA5500",
        "light_blue" => "#55FFFF",
        "light_gray" => "#AAAAAA",
        _ => return None,
    })
}

const GAME_TEAM_COLOR_NAMES: &[&str] = &[
    "black",
    "dark_blue",
    "dark_green",
    "dark_aqua",
    "dark_red",
    "dark_purple",
    "gold",
    "dark_gray",
    "light_purple",
    "light_blue",
    "light_gray",
    "gray",
    "blue",
    "green",
    "aqua",
    "red",
    "yellow",
    "white",
    "orange",
    "pink",
    "lime",
    "cyan",
    "magenta",
    "brown",
];

fn is_format_code_char(c: char) -> bool {
    matches!(
        c.to_ascii_lowercase(),
        '0'..='9' | 'a'..='f' | 'k'..='o' | 'r'
    )
}

pub fn strip_color_codes(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '§' || c == '\u{FFFD}' {
            if let Some(&next) = chars.get(i + 1) {
                if is_format_code_char(next) {
                    i += 2;
                    continue;
                }
            }
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

pub fn parse_wire_string_component(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()))
}

pub fn extract_component_text(component: &Value) -> String {
    match component {
        Value::String(s) => match serde_json::from_str::<Value>(s) {
            Ok(parsed) => extract_component_text(&parsed),
            Err(_) => s.clone(),
        },
        Value::Array(items) => items.iter().map(extract_component_text).collect(),
        Value::Object(fields) => {
            let mut text = String::new();
            if let Some(Value::String(t)) = fields.get("text") {
                text.push_str(t);
            }
            if let Some(Value::String(t)) = fields.get("translate") {
                text.push_str(t);
            }
            if let Some(Value::Array(extra)) = fields.get("extra") {
                for item in extra {
                    text.push_str(&extract_component_text(item));
                }
            }
            if let Some(Value::Array(with)) = fields.get("with") {
                for item in with {
                    text.push_str(&extract_component_text(item));
                }
            }
            text
        }
        _ => String::new(),
    }
}

pub fn parse_chat_to_plain(component: &Value) -> String {
    strip_color_codes(&extract_component_text(component))
}

fn extract_section_color(text: &str) -> Option<&'static str> {
    let chars: Vec<char> = text.chars().collect();
    for i in 0..chars.len() {
        if (chars[i] == '§' || chars[i] == '\u{FFFD}') && i + 1 < chars.len() {
            if let Some(hex) = char_to_hex(chars[i + 1]) {
                return Some(hex);
            }
        }
    }
    None
}

#[derive(Debug, Clone)]
pub enum TeamColorField {
    Numeric(i32),
    Named(String),
    None,
}

pub fn extract_team_color(prefix: &Value, color_field: &TeamColorField, raw_name: &str) -> String {
    if let Value::Object(fields) = prefix {
        if let Some(Value::String(color)) = fields.get("color") {
            if let Some(hex) = color.strip_prefix('#') {
                return format!("#{hex}");
            }
            if let Some(hex) = name_to_hex(color) {
                return hex.to_string();
            }
        }
    }

    let prefix_text = match prefix {
        Value::String(s) => s.clone(),
        other => extract_component_text(other),
    };

    if let Some(hex) = extract_section_color(&prefix_text) {
        return hex.to_string();
    }

    if let Some(hex) = extract_section_color(raw_name) {
        return hex.to_string();
    }

    let lower_name = raw_name.to_ascii_lowercase();
    for &color_name in GAME_TEAM_COLOR_NAMES {
        if lower_name.contains(color_name) {
            if let Some(hex) = name_to_hex(color_name) {
                return hex.to_string();
            }
        }
    }

    match color_field {
        TeamColorField::Numeric(n) if (0..=15).contains(n) => {
            if let Some(hex) = byte_to_hex(*n) {
                return hex.to_string();
            }
        }
        TeamColorField::Named(name) => {
            if let Some(hex) = name_to_hex(name) {
                return hex.to_string();
            }
        }
        _ => {}
    }

    "#AAAAAA".to_string()
}

pub fn format_team_name(team_id: &str) -> String {
    let Some((slot, color_name)) = team_id.split_once('-') else {
        return String::new();
    };
    if slot.is_empty() || !slot.chars().all(|character| character.is_ascii_digit()) {
        return String::new();
    }
    let Some(color_name) = GAME_TEAM_COLOR_NAMES
        .iter()
        .find(|candidate| candidate.eq_ignore_ascii_case(color_name))
    else {
        return String::new();
    };
    let mut chars = color_name.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

pub fn is_game_team(team_id: &str) -> bool {
    !format_team_name(team_id).is_empty()
}

pub fn is_npc(gamemode: Option<i32>, display_name: Option<&Value>) -> bool {
    if let Some(g) = gamemode {
        if g < 0 {
            return true;
        }
    }
    if let Some(name) = display_name {
        if extract_component_text(name).contains("[NPC]") {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_valid_color_codes_but_leaves_dangling_marker_content() {
        assert_eq!(strip_color_codes("§cHello §rWorld"), "Hello World");
        assert_eq!(strip_color_codes("§zHello"), "zHello");
        assert_eq!(strip_color_codes("plain text"), "plain text");
        assert_eq!(strip_color_codes("trailing§"), "trailing");
    }

    #[test]
    fn extracts_text_from_plain_component_object() {
        let v = serde_json::json!({"text": "Hello ", "extra": [{"text": "World"}]});
        assert_eq!(extract_component_text(&v), "Hello World");
    }

    #[test]
    fn extracts_text_from_wire_json_string() {
        let v = Value::String(r#"{"text":"Hi"}"#.to_string());
        assert_eq!(extract_component_text(&v), "Hi");
    }

    #[test]
    fn falls_back_to_literal_text_when_not_valid_json() {
        let v = Value::String("§cRaw legacy text".to_string());
        assert_eq!(extract_component_text(&v), "§cRaw legacy text");
        assert_eq!(parse_chat_to_plain(&v), "Raw legacy text");
    }

    #[test]
    fn extracts_team_color_from_component_color_field() {
        let prefix = serde_json::json!({"text": "", "color": "red"});
        let color = extract_team_color(&prefix, &TeamColorField::None, "team_red");
        assert_eq!(color, "#FF5555");
    }

    #[test]
    fn extracts_team_color_from_section_code_in_prefix_text() {
        let prefix = Value::String("§9".to_string());
        let color = extract_team_color(&prefix, &TeamColorField::None, "team1");
        assert_eq!(color, "#5555FF");
    }

    #[test]
    fn extracts_team_color_from_numeric_field_when_no_prefix_color() {
        let prefix = Value::String(String::new());
        let color = extract_team_color(&prefix, &TeamColorField::Numeric(12), "team1");
        assert_eq!(color, "#FF5555");
    }

    #[test]
    fn prefers_named_game_team_color_over_gray_metadata() {
        let prefix = Value::String(String::new());
        assert_eq!(
            extract_team_color(&prefix, &TeamColorField::Numeric(7), "1-WHITE"),
            "#FFFFFF"
        );
        assert_eq!(
            extract_team_color(&prefix, &TeamColorField::Numeric(7), "6-PINK"),
            "#FF55FF"
        );
    }

    #[test]
    fn falls_back_to_default_gray_when_nothing_matches() {
        let prefix = Value::String(String::new());
        let color = extract_team_color(&prefix, &TeamColorField::None, "team1");
        assert_eq!(color, "#AAAAAA");
    }

    #[test]
    fn formats_and_detects_game_team_names() {
        assert_eq!(format_team_name("1-RED"), "Red");
        assert_eq!(format_team_name("2-blue"), "Blue");
        assert_eq!(format_team_name("RED_team"), "");
        assert_eq!(format_team_name("blueTeam1"), "");
        assert!(!is_game_team("RED"));
        assert!(is_game_team("3-AQUA"));
        assert!(!is_game_team("team_aqua"));
        assert!(!is_game_team("random"));
    }

    #[test]
    fn detects_npc_via_negative_gamemode_or_marker_text() {
        assert!(is_npc(Some(-1), None));
        assert!(!is_npc(Some(0), None));
        let name = serde_json::json!({"text": "Steve [NPC]"});
        assert!(is_npc(Some(0), Some(&name)));
    }
}
