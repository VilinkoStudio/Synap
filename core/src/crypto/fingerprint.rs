/// 根据公钥指纹生成一个稳定但更易辨认的颜文字标识。
///
/// 这个标识不是安全校验手段，只适合用在 UI 或日志里帮助用户快速识别
/// 真正的校验仍然应该看完整指纹。
pub fn generate_kaomoji_fingerprint(fingerprint: &[u8]) -> String {
    if fingerprint.len() < 4 {
        return "(?_?)".to_string();
    }

    let contours = [
        ("", ""),
        ("(", ")"),
        ("ヽ(", ")ﾉ"),
        ("o(", ")o"),
        ("q(", ")p"),
        ("ʕ", "ʔ"),
        ("꒰", "꒱"),
        ("【", "】"),
        ("⊂(", ")⊃"),
        ("ᕙ(", ")ᕗ"),
        ("૮", "ა"),
        ("[(", ")]"),
    ];

    let eyes = [
        ("•", "•"),
        ("◕", "◕"),
        ("ಠ", "ಠ"),
        (">", "<"),
        ("T", "T"),
        ("^", "^"),
        ("≖", "≖"),
        ("☆", "☆"),
        ("◉", "◉"),
        ("눈", "눈"),
        ("@", "@"),
        ("♥", "♥"),
        ("x", "x"),
        ("-", "-"),
        ("O", "O"),
        ("+", "+"),
        ("⌐■", "■"),
        ("$ ", " $"),
    ];

    let mouths = [
        "_", ".", "ω", "▽", "△", "﹏", "Д", "x", "o", "v", "~", "‿", " ⌂ ", "∀", "ㅂ", "ᴥ", "ε",
        "罒", "ー",
    ];

    let accessories = [
        "", "✨", "💦", "💢", "🌸", "🔥", "💤", "⚡", "💻", "☕", "🔧", "BUG", "🚀", "📡", "🤖",
        "⭐",
    ];

    let contour = contours[fingerprint[0] as usize % contours.len()];
    let eye = eyes[fingerprint[1] as usize % eyes.len()];
    let mouth = mouths[fingerprint[2] as usize % mouths.len()];
    let accessory = accessories[fingerprint[3] as usize % accessories.len()];

    format!(
        "{}{}{}{}{}{}",
        contour.0, eye.0, mouth, eye.1, contour.1, accessory
    )
}

#[cfg(test)]
mod tests {
    use super::generate_kaomoji_fingerprint;

    #[test]
    fn test_generate_kaomoji_fingerprint_is_stable() {
        let fingerprint = [12u8, 45, 120, 8];
        assert_eq!(generate_kaomoji_fingerprint(&fingerprint), "눈Д눈💻");
    }

    #[test]
    fn test_generate_kaomoji_fingerprint_falls_back_for_short_input() {
        assert_eq!(generate_kaomoji_fingerprint(&[1, 2, 3]), "(?_?)");
    }
}
