use super::token::*;
use super::Source;

pub struct Snapshot {
  pub index: usize,
  pub pos:   (usize, usize),
}

impl Snapshot {
  fn new(index: usize, pos: (usize, usize)) -> Self {
    Snapshot {
      index,
      pos,
    }
  }
}



pub struct Tokenizer<'t> {
  pub pos: (usize, usize),

  index:     usize,
  source:    &'t Source,
  items:     Vec<char>,
  snapshots: Vec<Snapshot>
}

impl<'t> Tokenizer<'t> {
  pub fn new(source: &'t Source, items: Vec<char>) -> Self {
    Tokenizer {
      pos: (0, 0),

      source,
      items,
      index:     0,
      snapshots: Vec::new(),
    }
  }

  pub fn end(&self) -> bool {
    self.index >= self.items.len()
  }

  pub fn advance(&mut self) {
    if let Some(item) = self.items.get(self.index + 1) {
      match *item {
        '\n' => {
          self.pos.0 += 1;
          self.pos.1 += 0;
        },

        _ => self.pos.1 += 1,
      }
    }

    self.index += 1
  }

  pub fn advance_n(&mut self, n: usize) {
    for _ in 0 .. n {
      self.advance()
    }
  }

  pub fn peek_range(&self, n: usize) -> Option<String> {
    self.items.get(self.index..self.index + n).map(|chars| chars.iter().collect::<String>())
  }

  pub fn peek_n(&self, n: usize) -> Option<char> {
    self.items.get(self.index + n).cloned()
  }

  pub fn peek(&self) -> Option<char> {
    self.peek_n(0)
  }

  pub fn take_snapshot(&mut self) {
    self.snapshots.push(Snapshot::new(self.index, self.pos));
  }

  pub fn peek_snapshot(&self) -> Option<&Snapshot> {
    self.snapshots.last()
  }

  pub fn rollback_snapshot(&mut self) {
    let snapshot = self.snapshots.pop().unwrap();
    self.index = snapshot.index;
    self.pos = snapshot.pos;
  }

  pub fn commit_snapshot(&mut self) {
    self.snapshots.pop();
  }

  pub fn last_position(&self) -> (usize, usize) {
    self.peek_snapshot().unwrap().pos
  }

  pub fn try_match_token(&mut self, matcher: &Matcher) -> Result<Option<Token>, ()> {
    if self.end() {
      return Ok(Some(Token::new(TokenType::EOF, (self.pos.0, &self.source.lines[self.pos.0]), (self.pos.1, 0), "")));
    }

    self.take_snapshot();
    match matcher.try_match(self)? {
      Some(t) => {
        self.commit_snapshot();
        Ok(Some(t))
      }

      None => {
        self.rollback_snapshot();
        Ok(None)
      }
    }
  }
}