//! util.rs — Utilidades compartidas.
//!
//! Batch Image Generator — Eric Valls Gramunt

/// Recorta una cadena a `max` **caracteres** (no bytes) y añade `…` si hubo
/// recorte.
///
/// En Rust, `&s[..n]` indexa por bytes y **entra en pánico** si `n` cae en
/// medio de un carácter multibyte. La versión 2.4.0 hacía exactamente eso en
/// cuatro sitios (`main.rs`, `api.rs`, `randomizer.rs`), de modo que un
/// prompt con una `ñ`, un acento o un emoji en la posición equivocada tiraba
/// la aplicación entera. Esta función es la sustituta segura.
pub fn truncate_chars(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((idx, _)) => {
            let mut out = String::with_capacity(idx + 3);
            out.push_str(&s[..idx]);
            out.push('…');
            out
        }
        None => s.to_string(),
    }
}

/// Igual que `truncate_chars` pero sin sufijo, para prefijos de diagnóstico.
pub fn take_chars(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_trunca_si_cabe() {
        assert_eq!(truncate_chars("hola", 10), "hola");
        assert_eq!(truncate_chars("", 5), "");
    }

    #[test]
    fn trunca_por_caracteres_no_por_bytes() {
        assert_eq!(truncate_chars("abcdef", 3), "abc…");
    }

    /// Caso que hacía caer la 2.4.0: el corte cae justo dentro de un
    /// carácter multibyte.
    #[test]
    fn no_entra_en_panico_con_multibyte_en_la_frontera() {
        let s = format!("{}ñ{}", "a".repeat(119), "b".repeat(80));
        assert!(!s.is_char_boundary(120), "el caso de prueba debe cruzar frontera");
        let out = truncate_chars(&s, 120);
        assert!(out.ends_with('…'));
        assert_eq!(out.chars().count(), 121);
    }

    #[test]
    fn soporta_emojis() {
        let s = format!("{}🎨{}", "x".repeat(198), "y".repeat(50));
        assert!(!s.is_char_boundary(200));
        let out = truncate_chars(&s, 200);
        assert_eq!(out.chars().count(), 201);
    }

    #[test]
    fn acentos_y_emojis_mezclados() {
        let s = "Una mujer pelirroja 🔥 con expresión desafiante bajo la lluvia";
        for n in 0..=s.chars().count() + 5 {
            let _ = truncate_chars(s, n); // no debe entrar en pánico nunca
        }
    }

    #[test]
    fn take_chars_es_seguro() {
        let s = "año🎨nuevo";
        for n in 0..=12 {
            let _ = take_chars(s, n);
        }
        assert_eq!(take_chars("añor", 2), "añ");
    }
}
