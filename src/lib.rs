use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(windows)] {
        mod null;
        mod windows;
        use windows as implementation;
    } else if #[cfg(not(windows))] {
        mod unix;
        use unix as implementation;
    } else {
        compile_error!("target platform is not supported");
    }
}

#[derive(Clone, Debug)]
pub struct SpellingError {
    start: usize,
    length: usize,
    text: String,
}

impl SpellingError {
    /// Start index and len
    pub fn pos(&self) -> (usize, usize) {
        (self.start, self.length)
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

/// Instance of the system spell checker.
#[derive(Debug)]
pub struct Checker(implementation::Checker);

impl Checker {
    /// Create an instance of the system spell checker.
    pub fn new() -> Self {
        Checker(implementation::Checker::new())
    }

    /// Check a text for spelling errors. Returns an iterator over the errors present in the text.
    pub fn check<'a, 'b: 'a>(
        &'b mut self,
        text: &'a str,
    ) -> impl Iterator<Item = SpellingError> + 'a {
        self.0.check(text)
    }

    /// Suggest alternatives for a misspelled word.
    pub fn suggest<'a>(&'a mut self, word: &str) -> impl Iterator<Item = String> + 'a {
        self.0.suggest(word)
    }

    /// Instructs the spell checker to ignore a word in future checks. The word is temporarily
    /// added to the spell checker's ignore list, and other instances of the spell checker will not
    /// ignore the word.
    pub fn ignore(&mut self, word: &str) {
        self.0.ignore(word)
    }
}

impl Default for Checker {
    fn default() -> Self {
        Self::new()
    }
}
