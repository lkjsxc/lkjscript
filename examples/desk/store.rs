//! The Python desk's JSON lines, with one shared writer lock and durable replacement.
use std::json::{self, Value};

pub fn io<T>(result: Result<T, i64>, action: &str) -> Result<T, String> {
    match result { Ok(value) => Ok(value), Err(errno) => Err(format!("{action}: errno {errno}")) }
}
pub fn field(value: &Value, name: &str) -> Result<String, String> {
    match value.get(name) {
        None => Ok(String::new()),
        Some(value) => match value.as_str() {
            Some(text) => Ok(text.to_string()),
            None => Err(format!("desk field {name} must be a string")),
        },
    }
}
fn text(value: &str) -> Value { Value::String(value.to_string()) }
pub fn json_text(value: &Value) -> Result<String, String> {
    match json::stringify(value) {
        Ok(value) => Ok(value),
        Err(problem) => Err(format!("JSON byte {}: {}", problem.offset, problem.message)),
    }
}
#[derive(Clone)]
pub struct Row {
    pub id: String,
    pub time: i64,
    pub kind: String,
    pub who: String,
    pub text: String,
    pub options: Vec<String>,
    pub has_options: bool,
    pub reference: String,
    pub re: String,
    pub request: String,
}
impl Row {
    pub fn new(kind: &str, who: &str, text: &str) -> Row {
        Row { id: String::new(), time: 0i64, kind: kind.to_string(), who: who.to_string(),
            text: text.to_string(), options: Vec::new(), has_options: false,
            reference: String::new(), re: String::new(), request: String::new() }
    }
    pub fn parse(value: &Value) -> Result<Row, String> {
        let time = match value.get("t") {
            Some(value) => match value.as_number() {
                Some(number) => match number.parse::<i64>() {
                    Ok(number) => number,
                    Err(_) => return Err("desk field t must be signed integer seconds".to_string()),
                },
                None => return Err("desk field t must be an integer".to_string()),
            },
            None => return Err("desk row is missing t".to_string()),
        };
        let mut row = Row::new(&field(value, "kind")?, &field(value, "from")?, &field(value, "text")?);
        row.time = time;
        row.id = field(value, "id")?;
        if row.id.is_empty() { return Err("desk row is missing id".to_string()); }
        row.reference = field(value, "ref")?;
        row.re = field(value, "re")?;
        row.request = field(value, "request")?;
        if let Some(value) = value.get("options") {
            row.has_options = true;
            match value.as_array() {
                Some(options) => for option in options {
                    match option.as_str() {
                        Some(option) => row.options.push(option.to_string()),
                        None => return Err("desk option must be a string".to_string()),
                    }
                },
                None => return Err("desk options must be an array".to_string()),
            }
        }
        Ok(row)
    }
    pub fn value(&self, read: Option<bool>, open: Option<bool>) -> Value {
        let mut fields = vec![("id".to_string(), text(&self.id)),
            ("t".to_string(), Value::Number(format!("{}", self.time)))];
        if !self.kind.is_empty() {
            fields.push(("kind".to_string(), text(&self.kind)));
            if self.kind == "close" {
                fields.push(("ref".to_string(), text(&self.reference)));
                fields.push(("from".to_string(), text(&self.who)));
            } else {
                fields.push(("from".to_string(), text(&self.who)));
                fields.push(("text".to_string(), text(&self.text)));
            }
        } else {
            fields.push(("text".to_string(), text(&self.text)));
        }
        if self.has_options {
            let mut options = Vec::new();
            for option in &self.options { options.push(text(option)); }
            fields.push(("options".to_string(), Value::Array(options)));
        }
        if !self.re.is_empty() { fields.push(("re".to_string(), text(&self.re))); }
        if !self.request.is_empty() { fields.push(("request".to_string(), text(&self.request))); }
        if let Some(read) = read {
            fields.push(("read".to_string(), Value::Bool(read)));
            fields.push(("kind".to_string(), text("owner")));
        }
        if let Some(open) = open { fields.push(("open".to_string(), Value::Bool(open))); }
        Value::Object(fields)
    }
}
pub fn lock(root: &mut Directory) -> Result<File, String> {
    let mut lock = io(root.open_append(".lock"), "open desk lock")?;
    io(lock.lock_wait(), "wait for desk writer")?;
    Ok(lock)
}
pub fn read(root: &mut Directory, path: &str) -> Result<String, String> {
    let mut file = match root.open(path) {
        Ok(file) => file,
        Err(2i64) => return Ok(String::new()),
        Err(errno) => return Err(format!("open desk {path}: errno {errno}")),
    };
    let bytes = io(file.read_to_end(), "read desk file")?;
    match String::from_utf8(bytes) {
        Ok(text) => Ok(text), Err(_) => Err(format!("desk {path} is not UTF-8")),
    }
}
pub fn parse_rows(text: &str) -> Result<Vec<Row>, String> {
    let mut rows = Vec::new();
    let mut line = 0usize;
    for text in text.split('\n') {
        line += 1usize;
        if !std::text::trim(text).is_empty() {
            let value = match json::parse(text.as_bytes()) {
                Ok(value) => value,
                Err(problem) => return Err(format!("desk line {line}, JSON byte {}: {}", problem.offset, problem.message)),
            };
            rows.push(Row::parse(&value)?);
        }
    }
    Ok(rows)
}
pub fn rows(root: &mut Directory, path: &str) -> Result<Vec<Row>, String> {
    let text = read(root, path)?;
    parse_rows(&text)
}
// Copying the JSONL prefix costs linear I/O, but a crash cannot expose a torn
// JSON line to the other implementation. Both implementations hold .lock.
pub fn replace(root: &mut Directory, path: &str, bytes: &[u8]) -> Result<(), String> {
    let temporary = format!(".{path}-next");
    // Remove a crash leftover by name, never through its possible symlink.
    match root.remove(&temporary) {
        Ok(()) | Err(2i64) => {},
        Err(errno) => return Err(format!("remove desk replacement: errno {errno}")),
    }
    let mut file = io(root.create_new(&temporary), "create desk replacement")?;
    io(file.write_all(bytes), "write desk replacement")?;
    io(file.sync(), "sync desk replacement")?;
    drop(file);
    io(root.rename(&temporary, path), "publish desk replacement")?;
    io(root.sync(), "sync desk directory")
}
pub fn touch_alive(root: &mut Directory) -> Result<(), String> {
    let lock = lock(root)?;
    replace(root, "alive", b"")?;
    drop(lock);
    Ok(())
}
pub fn append_locked(root: &mut Directory, name: &str, mut row: Row,
    clock: &mut Clock, entropy: &mut Entropy) -> Result<Row, String> {
    let nanos = io(clock.unix_nanoseconds(), "read desk realtime")?;
    row.time = (nanos / 1_000_000_000usize) as i64;
    let mut random = [0u8; 8];
    io(entropy.fill(&mut random), "generate desk id")?;
    let mut id = format!("{nanos:016x}");
    for byte in &random[..] { id.push_str(&format!("{byte:02x}")); }
    row.id = id;
    let mut previous = read(root, name)?;
    // Existing bytes are preserved. Reading the log validates its records;
    // appending does not parse and reconstruct every historical row.
    if !previous.is_empty() && !previous.ends_with('\n') {
        return Err(format!("desk {name} has an incomplete final line; no write performed"));
    }
    previous.push_str(&json_text(&row.value(None, None))?);
    previous.push('\n');
    replace(root, name, previous.as_bytes())?;
    Ok(row)
}
pub fn append(root: &mut Directory, name: &str, row: Row,
    clock: &mut Clock, entropy: &mut Entropy) -> Result<Row, String> {
    let lock = lock(root)?;
    let row = append_locked(root, name, row, clock, entropy)?;
    drop(lock);
    Ok(row)
}
pub fn seen(root: &mut Directory) -> Result<usize, String> {
    let text = read(root, "seen")?;
    let text = std::text::trim(&text);
    if text.is_empty() { return Ok(0usize); }
    match text.parse::<usize>() { Ok(value) => Ok(value), Err(_) => Err("desk seen must be an unsigned integer".to_string()) }
}
pub fn closed(notes: &[Row], inbox: &[Row], id: &str) -> bool {
    for note in notes { if note.kind == "close" && note.reference == id { return true; } }
    for row in inbox { if row.re == id && !row.re.is_empty() { return true; } }
    false
}
pub fn unread_line(row: &Row, notes: &[Row]) -> String {
    for note in notes {
        if note.id == row.re && !row.re.is_empty() {
            if !note.text.is_empty() { return format!("[answer to: {}] {}", note.text, row.text); }
            break;
        }
    }
    row.text.clone()
}
pub fn inbox(root: &mut Directory, peek: bool, offset: i64) -> Result<String, String> {
    let lock = lock(root)?;
    let notes = rows(root, "notes.jsonl")?;
    let inbox = rows(root, "inbox.jsonl")?;
    let count = seen(root)?;
    let mut lines = String::new();
    for index in count..inbox.len() {
        let row = &inbox[index];
        let stamp = std::time::local_minute(row.time, offset)?;
        lines.push_str(&format!("{stamp} {}\n", unread_line(row, &notes)));
    }
    if !peek {
        replace(root, "seen", format!("{}", inbox.len()).as_bytes())?;
        match root.remove("ping") {
            Ok(()) | Err(2i64) => {},
            Err(errno) => return Err(format!("remove desk ping: errno {errno}")),
        }
        io(root.sync(), "sync desk inbox read")?;
    }
    drop(lock);
    touch_alive(root)?;
    Ok(lines)
}
fn ping(root: &mut Directory, notes: &[Row], inbox: &[Row]) -> Result<(), String> {
    let count = seen(root)?;
    let mut text = String::new();
    for index in count..inbox.len() {
        text.push_str(&unread_line(&inbox[index], notes));
        text.push('\n');
    }
    // A retry after inbox marked the row read must not recreate an empty ping.
    // Comparing also repairs a crash between publishing inbox and publishing ping.
    if text.is_empty() {
        match root.remove("ping") {
            Ok(()) => io(root.sync(), "sync empty desk ping")?,
            Err(2i64) => {},
            Err(errno) => return Err(format!("remove empty desk ping: errno {errno}")),
        }
        return Ok(());
    }
    if read(root, "ping")? == text { return Ok(()); }
    replace(root, "ping", text.as_bytes())
}
pub fn record(root: &mut Directory, text: &str, re: &str, request: &str,
    clock: &mut Clock, entropy: &mut Entropy) -> Result<Row, String> {
    let lock = lock(root)?;
    let notes = rows(root, "notes.jsonl")?;
    let mut inbox = rows(root, "inbox.jsonl")?;
    if !re.is_empty() {
        for row in &inbox {
            if row.re == re {
                let result = row.clone();
                ping(root, &notes, &inbox)?;
                return Ok(result);
            }
        }
        let mut found = false;
        for note in &notes { if note.id == re && note.kind == "ask" { found = true; } }
        if !found || closed(&notes, &inbox, re) { return Err("Question is closed or missing".to_string()); }
    } else if !request.is_empty() {
        for row in &inbox {
            if row.request == request {
                let result = row.clone();
                ping(root, &notes, &inbox)?;
                return Ok(result);
            }
        }
    }
    let mut row = Row::new("", "", text);
    row.re = re.to_string();
    row.request = request.to_string();
    let row = append_locked(root, "inbox.jsonl", row, clock, entropy)?;
    inbox.push(row.clone());
    ping(root, &notes, &inbox)?;
    drop(lock);
    Ok(row)
}
pub fn state(root: &mut Directory, now: i64) -> Result<Value, String> {
    let lock = lock(root)?;
    let notes = rows(root, "notes.jsonl")?;
    let inbox = rows(root, "inbox.jsonl")?;
    let count = seen(root)?;
    let mut feed: Vec<(Row, Option<bool>)> = Vec::new();
    for row in &notes { if row.kind != "close" { feed.push((row.clone(), None)); } }
    for index in 0usize..inbox.len() { feed.push((inbox[index].clone(), Some(index < count))); }
    drop(lock);
    let mut done = std::collections::BTreeSet::new();
    for row in &notes { if row.kind == "close" { done.insert(row.reference.clone()); } }
    for row in &inbox { if !row.re.is_empty() { done.insert(row.re.clone()); } }
    let mut order: Vec<(i64, String, usize)> = Vec::new();
    for index in 0usize..feed.len() {
        order.push((feed[index].0.time, feed[index].0.id.clone(), index));
    }
    order.sort();
    let mut asks = Vec::new();
    for item in &order {
        let row = &feed[item.2].0;
        if row.kind == "ask" && !done.contains(&row.id) {
            asks.push(row.value(None, Some(true)));
        }
    }
    let mut values = Vec::new();
    let mut index = order.len();
    while index > 0usize && values.len() < 80usize {
        index -= 1usize;
        let source = order[index].2;
        let row = &feed[source].0;
        let open = if row.kind == "ask" { Some(!done.contains(&row.id)) } else { None };
        values.push(row.value(feed[source].1, open));
    }
    Ok(Value::Object(vec![("asks".to_string(), Value::Array(asks)),
        ("feed".to_string(), Value::Array(values)),
        ("jobs".to_string(), Value::Array(Vec::new())),
        ("detail".to_string(), Value::String(String::new())),
        ("now".to_string(), Value::Number(format!("{now}")))]))
}

#[test]
fn parsed_rows_preserve_unicode_and_order() {
    let data = "{\"id\":\"a\",\"t\":1,\"kind\":\"ask\",\"from\":\"chatgpt\",\"text\":\"日本語 😀\",\"options\":[\"はい\",\"いいえ\"]}\n";
    let rows = parse_rows(data).unwrap();
    assert!(rows.len() == 1usize && rows[0usize].text == "日本語 😀");
    let result = json_text(&rows[0usize].value(None, Some(true))).unwrap();
    let value = json::parse(result.as_bytes()).unwrap();
    assert!(value.get("open").unwrap().as_bool() == Some(true));
    assert!(field(&value, "text").unwrap() == "日本語 😀");
}
#[test]
fn answer_and_close_semantics() {
    let mut ask = Row::new("ask", "claude", "Choose?"); ask.id = "a".to_string();
    let mut answer = Row::new("", "", "はい"); answer.re = "a".to_string();
    let notes = vec![ask]; let inbox = vec![answer];
    assert!(closed(&notes, &inbox, "a"));
    assert!(unread_line(&inbox[0usize], &notes) == "[answer to: Choose?] はい");
}
