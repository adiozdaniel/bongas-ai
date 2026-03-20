//! Phase 3.3: Sheng-Native Search Analyzer
//!
//! Implements a custom Tantivy analyzer that uses jieba-rs for tokenization,
//! enabling better matching for Swahili and Sheng dialects.

use std::sync::Arc;
use tantivy::tokenizer::{Tokenizer, TokenStream, Token, TextAnalyzer, LowerCaser};
use jieba_rs::Jieba;

/// A tokenizer that uses Jieba for segmenting Swahili/Sheng/Mixed text.
#[derive(Clone)]
pub struct ShengTokenizer {
    jieba: Arc<Jieba>,
}

impl ShengTokenizer {
    pub fn new() -> Self {
        Self {
            jieba: Arc::new(Jieba::new()),
        }
    }
}

impl Default for ShengTokenizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Tokenizer for ShengTokenizer {
    type TokenStream<'a> = ShengTokenStream;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> ShengTokenStream {
        let words = self.jieba.cut(text, false);
        let mut tokens = Vec::new();
        let mut offset = 0;

        for word in words {
            let word_len = word.len();
            tokens.push(Token {
                offset_from: offset,
                offset_to: offset + word_len,
                position: tokens.len(),
                text: word.to_string(),
                position_length: 1,
            });
            offset += word_len;
        }

        ShengTokenStream {
            tokens,
            index: 0,
        }
    }
}

pub struct ShengTokenStream {
    tokens: Vec<Token>,
    index: usize,
}

impl TokenStream for ShengTokenStream {
    fn advance(&mut self) -> bool {
        if self.index < self.tokens.len() {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn token(&self) -> &Token {
        &self.tokens[self.index - 1]
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.tokens[self.index - 1]
    }
}

/// Create the specialized "sheng" analyzer.
pub fn sheng_analyzer() -> TextAnalyzer {
    TextAnalyzer::builder(ShengTokenizer::new())
        .filter(LowerCaser)
        .build()
}
