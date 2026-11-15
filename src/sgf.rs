use crate::game::{Move, Rules, BOARD_SIZES};

#[derive(Clone, Debug, PartialEq)]
pub struct SgfRecord {
    pub size: usize,
    pub rules: Rules,
    pub komi: f64,
    pub moves: Vec<Move>,
}

#[derive(Default)]
struct Node {
    props: Vec<(String, Vec<String>)>,
}

impl Node {
    fn first(&self, name: &str) -> Option<&str> {
        self.props
            .iter()
            .find(|(key, _)| key == name)
            .and_then(|(_, values)| values.first())
            .map(String::as_str)
    }
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn new(text: &str) -> Self {
        Self {
            chars: text.trim_start_matches('\u{feff}').chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let value = self.peek()?;
        self.pos += 1;
        Some(value)
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.pos += 1;
        }
    }

    fn parse_game(&mut self) -> Result<Vec<Node>, String> {
        self.skip_ws();
        if self.bump() != Some('(') {
            return Err("SGF 必须以 ( 开始".into());
        }

        let mut nodes = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some(';') => {
                    self.bump();
                    nodes.push(self.parse_node()?);
                }
                Some('(') => self.skip_variation()?,
                Some(')') => {
                    self.bump();
                    break;
                }
                None => return Err("SGF 缺少右括号".into()),
                Some(ch) => return Err(format!("SGF 中无法解析的字符: {ch}")),
            }
        }

        if nodes.is_empty() {
            return Err("SGF 中没有节点".into());
        }
        Ok(nodes)
    }

    fn parse_node(&mut self) -> Result<Node, String> {
        let mut node = Node::default();

        loop {
            self.skip_ws();
            match self.peek() {
                Some(';') | Some('(') | Some(')') | None => break,
                Some(ch) if ch.is_ascii_uppercase() => {
                    let name = self.parse_property_name();
                    self.skip_ws();
                    let mut values = Vec::new();
                    while self.peek() == Some('[') {
                        values.push(self.parse_value()?);
                        self.skip_ws();
                    }
                    if values.is_empty() {
                        return Err(format!("SGF 属性 {name} 没有值"));
                    }
                    node.props.push((name, values));
                }
                Some(ch) => return Err(format!("SGF 节点中无法解析的字符: {ch}")),
            }
        }

        Ok(node)
    }

    fn parse_property_name(&mut self) -> String {
        let mut name = String::new();
        while let Some(ch) = self.peek() {
            if !ch.is_ascii_uppercase() {
                break;
            }
            name.push(ch);
            self.pos += 1;
        }
        name
    }

    fn parse_value(&mut self) -> Result<String, String> {
        if self.bump() != Some('[') {
            return Err("SGF 属性值缺少 [".into());
        }

        let mut value = String::new();
        loop {
            let Some(ch) = self.bump() else {
                return Err("SGF 属性值缺少 ]".into());
            };
            match ch {
                ']' => break,
                '\\' => {
                    let Some(escaped) = self.bump() else {
                        return Err("SGF 属性值以转义符结尾".into());
                    };
                    if escaped == '\n' {
                        continue;
                    }
                    if escaped == '\r' {
                        if self.peek() == Some('\n') {
                            self.pos += 1;
                        }
                        continue;
                    }
                    value.push(escaped);
                }
                _ => value.push(ch),
            }
        }
        Ok(value)
    }

    fn skip_variation(&mut self) -> Result<(), String> {
        if self.bump() != Some('(') {
            return Err("SGF 变化图缺少 (".into());
        }

        let mut depth = 1;
        while depth > 0 {
            let Some(ch) = self.bump() else {
                return Err("SGF 变化图缺少右括号".into());
            };
            match ch {
                '[' => self.skip_value()?,
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
        }
        Ok(())
    }

    fn skip_value(&mut self) -> Result<(), String> {
        loop {
            let Some(ch) = self.bump() else {
                return Err("SGF 变化图属性缺少 ]".into());
            };
            match ch {
                '\\' => {
                    if self.bump().is_none() {
                        return Err("SGF 变化图属性以转义符结尾".into());
                    }
                }
                ']' => return Ok(()),
                _ => {}
            }
        }
    }
}

pub fn parse_sgf(text: &str) -> Result<SgfRecord, String> {
    let nodes = Parser::new(text).parse_game()?;

    for node in &nodes {
        if node
            .props
            .iter()
            .any(|(name, _)| matches!(name.as_str(), "AB" | "AW" | "AE" | "HA"))
        {
            return Err("暂不支持带让子或初始摆子的 SGF".into());
        }
    }

    let size = match first_property(&nodes, "SZ") {
        Some(value) => value
            .trim()
            .parse::<usize>()
            .map_err(|_| format!("无效的棋盘大小: {value}"))?,
        None => 19,
    };
    if !BOARD_SIZES.contains(&size) {
        return Err(format!("暂不支持 {size} 路棋盘，仅支持 9、13、19 路"));
    }

    let rules = match first_property(&nodes, "RU") {
        Some(value) => parse_rules(value)?,
        None => Rules::Chinese,
    };

    let komi = match first_property(&nodes, "KM") {
        Some(value) => value
            .trim()
            .parse::<f64>()
            .map_err(|_| format!("无效的贴目: {value}"))?,
        None => default_komi(rules),
    };
    if !komi.is_finite() {
        return Err("无效的贴目".into());
    }

    let mut moves = Vec::new();
    let mut black_to_play = true;
    for node in &nodes {
        let black = node.first("B");
        let white = node.first("W");
        if black.is_some() && white.is_some() {
            return Err(format!("第 {} 手同时包含黑白落子", moves.len() + 1));
        }

        let (value, expected_black) = if let Some(value) = black {
            (value, true)
        } else if let Some(value) = white {
            (value, false)
        } else {
            continue;
        };

        if expected_black != black_to_play {
            return Err(format!("第 {} 手颜色不连续", moves.len() + 1));
        }

        moves.push(parse_move(value, size)?);
        black_to_play = !black_to_play;
    }

    Ok(SgfRecord {
        size,
        rules,
        komi,
        moves,
    })
}

pub fn write_sgf(record: &SgfRecord) -> String {
    let mut text = format!(
        "(;GM[1]FF[4]CA[UTF-8]AP[LazyGo:{}]SZ[{}]KM[{}]RU[{}]",
        env!("CARGO_PKG_VERSION"),
        record.size,
        format_komi(record.komi),
        rules_name(record.rules),
    );

    for (index, mv) in record.moves.iter().enumerate() {
        let color = if index % 2 == 0 { 'B' } else { 'W' };
        match *mv {
            Move::Play(x, y) => {
                text.push_str(&format!(";{color}[{}{}]", coord_char(x), coord_char(y)));
            }
            Move::Pass => text.push_str(&format!(";{color}[]")),
        }
    }

    text.push(')');
    text
}

pub fn save_to_downloads(filename: &str, contents: &str) -> Result<String, String> {
    #[cfg(target_os = "android")]
    {
        android_save::save_to_downloads(filename, contents)
    }
    #[cfg(not(target_os = "android"))]
    {
        let dir = std::env::temp_dir().join("LazyGo");
        std::fs::create_dir_all(&dir).map_err(|error| format!("创建保存目录失败: {error}"))?;
        let path = dir.join(filename);
        std::fs::write(&path, contents).map_err(|error| format!("保存 SGF 失败: {error}"))?;
        Ok(path.display().to_string())
    }
}

pub fn suggested_filename() -> String {
    format!(
        "lazygo-{}.sgf",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    )
}

fn first_property<'a>(nodes: &'a [Node], name: &str) -> Option<&'a str> {
    nodes.iter().find_map(|node| node.first(name))
}

fn parse_rules(value: &str) -> Result<Rules, String> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.starts_with("chinese") || normalized == "cn" {
        Ok(Rules::Chinese)
    } else if normalized.starts_with("japanese") || normalized == "jp" || normalized == "jpn" {
        Ok(Rules::Japanese)
    } else if normalized.starts_with("ing") {
        Ok(Rules::Ing)
    } else {
        Err(format!("暂不支持 SGF 规则: {value}"))
    }
}

fn default_komi(rules: Rules) -> f64 {
    match rules {
        Rules::Japanese => 6.5,
        Rules::Chinese | Rules::Ing => 7.5,
    }
}

fn parse_move(value: &str, size: usize) -> Result<Move, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(Move::Pass);
    }

    let mut chars = value.chars();
    let (Some(x), Some(y)) = (chars.next(), chars.next()) else {
        return Err(format!("无效的落子坐标: {value}"));
    };
    if chars.next().is_some() || !x.is_ascii_lowercase() || !y.is_ascii_lowercase() {
        return Err(format!("无效的落子坐标: {value}"));
    }

    let x = x as u8 - b'a';
    let y = y as u8 - b'a';
    let x = x as usize;
    let y = y as usize;
    if x >= size || y >= size {
        if value == "tt" {
            return Ok(Move::Pass);
        }
        return Err(format!("落子坐标 {value} 超出 {size} 路棋盘"));
    }

    Ok(Move::Play(x, y))
}

fn coord_char(value: usize) -> char {
    (b'a' + value as u8) as char
}

fn rules_name(rules: Rules) -> &'static str {
    match rules {
        Rules::Chinese => "Chinese",
        Rules::Japanese => "Japanese",
        Rules::Ing => "Ing",
    }
}

fn format_komi(komi: f64) -> String {
    if komi.fract() == 0.0 {
        format!("{komi:.1}")
    } else {
        komi.to_string()
    }
}

#[cfg(target_os = "android")]
mod android_save;

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> SgfRecord {
        SgfRecord {
            size: 9,
            rules: Rules::Chinese,
            komi: 7.5,
            moves: vec![
                Move::Play(2, 2),
                Move::Play(6, 6),
                Move::Pass,
                Move::Play(4, 4),
            ],
        }
    }

    #[test]
    fn round_trips_a_record() {
        let record = record();
        let text = write_sgf(&record);
        assert_eq!(parse_sgf(&text).unwrap(), record);
    }

    #[test]
    fn round_trips_the_largest_board() {
        let record = SgfRecord {
            size: 19,
            rules: Rules::Ing,
            komi: 8.0,
            moves: vec![Move::Play(18, 18), Move::Play(0, 0)],
        };
        let text = write_sgf(&record);
        assert_eq!(parse_sgf(&text).unwrap(), record);
        assert!(text.contains("SZ[19]"));
        assert!(text.contains("B[ss]"));
    }

    #[test]
    fn handles_pass_and_variations() {
        let text = "(;GM[1]SZ[9]KM[6.5]RU[Japanese];B[ee];W[];(;B[cc])(;B[dd]))";
        let record = parse_sgf(text).unwrap();
        assert_eq!(record.size, 9);
        assert_eq!(record.rules, Rules::Japanese);
        assert_eq!(record.komi, 6.5);
        assert_eq!(record.moves, vec![Move::Play(4, 4), Move::Pass]);
    }

    #[test]
    fn accepts_legacy_tt_pass() {
        let record = parse_sgf("(;SZ[19];B[tt];W[dd])").unwrap();
        assert_eq!(record.moves, vec![Move::Pass, Move::Play(3, 3)]);
    }

    #[test]
    fn rejects_setup_stones() {
        let error = parse_sgf("(;SZ[9]AB[cc];W[dd])").unwrap_err();
        assert!(error.contains("让子"));
    }

    #[test]
    fn rejects_invalid_size_and_colors() {
        assert!(parse_sgf("(;SZ[10];B[aa])").unwrap_err().contains("10"));
        assert!(parse_sgf("(;SZ[9];B[aa];B[bb])")
            .unwrap_err()
            .contains("颜色"));
    }
}
