//! Reglas CHEF de presentación segura y acotada de texto local.
//!
//! Estas funciones no interpretan el texto como comandos, HTML ni URLs. El
//! llamador sigue siendo responsable de limitar el tamaño de los datos antes
//! de almacenarlos o enviarlos por IPC.

/// Indica si el texto carece de caracteres de control, salvo tabulación y
/// saltos de línea que son útiles en mensajes y transcripciones.
pub fn is_printable_text(value: &str) -> bool {
    value
        .chars()
        .all(|character| !character.is_control() || matches!(character, '\t' | '\n' | '\r'))
}

/// Sustituye caracteres de control por escapes ASCII para que no alteren una
/// terminal al representar texto recibido como evidencia.
///
/// Conserva tabulación, salto de línea y retorno de carro. Los controles hasta
/// `U+00FF` usan `\\xNN`; los demás usan `\\u{...}`.
pub fn escape_terminal_controls(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if !character.is_control() || matches!(character, '\t' | '\n' | '\r') {
            escaped.push(character);
        } else if character <= '\u{00ff}' {
            use std::fmt::Write as _;
            let _ = write!(escaped, "\\x{:02x}", character as u32);
        } else {
            use std::fmt::Write as _;
            let _ = write!(escaped, "\\u{{{:x}}}", character as u32);
        }
    }
    escaped
}

/// Trunca por puntos de código Unicode y respeta el máximo solicitado.
///
/// Usa `...` cuando hay espacio suficiente y tantos puntos como permita un
/// límite inferior a tres caracteres. Un límite cero produce una cadena vacía.
pub fn truncate_text(value: &str, max_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_chars {
        return value.to_owned();
    }
    if max_chars == 0 {
        return String::new();
    }

    let suffix = if max_chars >= 3 {
        "...".to_owned()
    } else {
        ".".repeat(max_chars)
    };
    let prefix_len = max_chars.saturating_sub(suffix.chars().count());
    let mut truncated = value.chars().take(prefix_len).collect::<String>();
    truncated.push_str(&suffix);
    truncated
}
