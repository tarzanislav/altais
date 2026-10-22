//! Text folding for search: the same function prepares the index and the
//! query, so "vylet" finds "Výlet", "LODZ" finds "Łódź" and "елка" finds
//! "ёлка".
//!
//! - letters are lowercased;
//! - Latin letters with diacritics become their base letter (Slovak, Czech,
//!   Hungarian, Polish, German and other European languages), including
//!   letters Unicode does not decompose (ł, đ, ø, ß, æ, œ);
//! - Russian ё becomes е (й stays a separate letter, as in Russian usage);
//! - every Chinese or Japanese character becomes its own word, so a search
//!   for a word of two characters finds it inside a sentence (the query
//!   searches the characters as an exact sequence).

/// Folds one character into `out`.
fn fold_char(c: char, out: &mut String) {
    let folded = match c {
        'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'ā' | 'ă' | 'ą' => "a",
        'č' | 'ć' | 'ç' | 'ĉ' | 'ċ' => "c",
        'ď' | 'đ' => "d",
        'é' | 'è' | 'ê' | 'ë' | 'ě' | 'ē' | 'ę' | 'ė' => "e",
        'ğ' | 'ģ' => "g",
        'í' | 'ì' | 'î' | 'ï' | 'ī' | 'į' | 'ı' => "i",
        'ķ' => "k",
        'ĺ' | 'ľ' | 'ł' | 'ļ' => "l",
        'ň' | 'ń' | 'ñ' | 'ņ' => "n",
        'ó' | 'ò' | 'ô' | 'ö' | 'ő' | 'õ' | 'ø' | 'ō' => "o",
        'ŕ' | 'ř' => "r",
        'š' | 'ś' | 'ş' | 'ș' => "s",
        'ť' | 'ţ' | 'ț' => "t",
        'ú' | 'ù' | 'û' | 'ü' | 'ű' | 'ů' | 'ū' | 'ų' => "u",
        'ý' | 'ÿ' => "y",
        'ž' | 'ź' | 'ż' => "z",
        'ß' => "ss",
        'æ' => "ae",
        'œ' => "oe",
        'ё' => "е",
        _ => {
            out.push(c);
            return;
        }
    };
    out.push_str(folded);
}

/// Chinese characters and Japanese kana, which are written without spaces.
pub fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x3040..=0x30FF   // Hiragana, Katakana
        | 0x3400..=0x4DBF // CJK Extension A
        | 0x4E00..=0x9FFF // CJK Unified Ideographs
        | 0xF900..=0xFAFF // CJK Compatibility Ideographs
        | 0x20000..=0x2FA1F) // Extensions B–F, compatibility supplement
}

/// Folds text for the search index or a query.
pub fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    for c in text.chars() {
        if is_cjk(c) {
            out.push(' ');
            out.push(c);
            out.push(' ');
            continue;
        }
        for lower in c.to_lowercase() {
            fold_char(lower, &mut out);
        }
    }
    out
}

/// Builds an FTS5 query from what the user typed: every word must occur
/// (as a word prefix), quoted words and Chinese runs must occur as written.
/// Returns `None` if nothing searchable is left.
pub fn fts_query(input: &str) -> Option<String> {
    let mut parts = Vec::new();
    let mut rest = input;
    while !rest.trim().is_empty() {
        rest = rest.trim_start();
        let (term, exact, next) = if let Some(after) = rest.strip_prefix('"') {
            match after.find('"') {
                Some(end) => (&after[..end], true, &after[end + 1..]),
                None => (after, true, ""),
            }
        } else {
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            (&rest[..end], false, &rest[end..])
        };
        rest = next;
        let folded = fold(term);
        // Keep letters, digits and CJK; everything else separates words.
        let words: Vec<String> = folded
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(str::to_owned)
            .collect();
        if words.is_empty() {
            continue;
        }
        let has_cjk = words.iter().any(|w| w.chars().any(is_cjk));
        let phrase = words.join(" ");
        if exact || has_cjk || words.len() > 1 {
            parts.push(format!("\"{phrase}\""));
        } else {
            parts.push(format!("\"{phrase}\"*"));
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" AND "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_the_interface_languages() {
        assert_eq!(fold("Výlet na Štrbské Pleso"), "vylet na strbske pleso");
        assert_eq!(
            fold("Příliš žluťoučký kůň úpěl ďábelské ódy"),
            "prilis zlutoucky kun upel dabelske ody"
        );
        assert_eq!(fold("Árvíztűrő tükörfúrógép"), "arvizturo tukorfurogep");
        assert_eq!(fold("Zażółć gęślą jaźń, ŁÓDŹ"), "zazolc gesla jazn, lodz");
        assert_eq!(fold("Ёлка и йогурт"), "елка и йогурт");
        assert_eq!(fold("Straße, Æble, Œuvre"), "strasse, aeble, oeuvre");
        assert_eq!(fold("加密笔记"), " 加  密  笔  记 ");
    }

    #[test]
    fn builds_queries() {
        assert_eq!(fts_query("Výlet"), Some("\"vylet\"*".into()));
        assert_eq!(
            fts_query("tatry  výlet"),
            Some("\"tatry\"* AND \"vylet\"*".into())
        );
        assert_eq!(
            fts_query("\"štrbské pleso\" leto"),
            Some("\"strbske pleso\" AND \"leto\"*".into())
        );
        assert_eq!(fts_query("笔记"), Some("\"笔 记\"".into()));
        assert_eq!(fts_query("e-mail"), Some("\"e mail\"".into()));
        // Quotes and FTS syntax in the input cannot break the query.
        assert_eq!(
            fts_query("a\" OR b*"),
            Some("\"a\"* AND \"or\"* AND \"b\"*".into())
        );
        assert_eq!(fts_query("\"neuzavretá"), Some("\"neuzavreta\"".into()));
        assert_eq!(fts_query("  -- ** "), None);
    }
}
