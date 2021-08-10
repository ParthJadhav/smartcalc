pub mod primative;
pub mod unary;
pub mod util;
pub mod binary;
pub mod assignment;
pub mod statement;

use alloc::vec::Vec;
use core::cell::{Cell};

use crate::syntax::util::map_parser;

use crate::types::*;
use alloc::rc::Rc;
use crate::app::Storage;
use crate::syntax::assignment::AssignmentParser;
use crate::syntax::binary::AddSubtractParser;

pub type ParseType = fn(parser: &mut SyntaxParser) -> AstResult;

pub struct SyntaxParser<'a> {
    pub tokens: Rc<Vec<TokenType>>,
    pub index: Cell<usize>,
    pub storage: &'a mut Storage
}

pub trait SyntaxParserTrait {
    fn parse(parser: &mut SyntaxParser) -> AstResult;
}

impl<'a> SyntaxParser<'a> {
    pub fn new(tokens: Rc<Vec<TokenType>>, storage: &'a mut Storage) -> SyntaxParser {
        SyntaxParser {
            tokens,
            index: Cell::new(0),
            storage
        }
    }

    pub fn parse(&mut self) -> AstResult {
        let ast = map_parser(self, &[AssignmentParser::parse, AddSubtractParser::parse])?;
        Ok(ast)
    }

    pub fn set_index(&self, index: usize) {
        self.index.set(index);
    }

    pub fn get_index(&self) -> usize {
        self.index.get()
    }

    #[allow(clippy::result_unit_err)]
    pub fn peek_token(&self) -> Result<&TokenType, ()> {
        match self.tokens.get(self.index.get()) {
            Some(token) => Ok(token),
            None => Err(())
        }
    }

    #[allow(clippy::result_unit_err)]
    pub fn next_token(&self) -> Result<&TokenType, ()> {
        match self.tokens.get(self.index.get() + 1) {
            Some(token) => Ok(token),
            None => Err(())
        }
    }
    
    pub fn consume_token(&self) -> Option<&TokenType> {
        self.index.set(self.index.get() + 1);
        self.tokens.get(self.index.get())
    }

    fn match_operator(&self, operators: &[char]) -> Option<char> {
        for operator in operators {
            if self.check_operator(*operator) {
                self.consume_token();
                return Some(*operator);
            }
        }

        None
    }

    fn check_operator(&self, operator: char) -> bool {
        match self.peek_token() {
            Ok(TokenType::Operator(token_operator)) => {
                operator == *token_operator
            },
            _ => false
        }
    }
}