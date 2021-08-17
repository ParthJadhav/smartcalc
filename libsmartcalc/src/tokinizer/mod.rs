mod number;
mod operator;
mod text;
mod whitespace;
mod field;
mod percent;
mod atom;
mod time;
mod money;
mod comment;
mod month;

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use alloc::string::ToString;
use crate::config::SmartCalcConfig;
use crate::{token::ui_token::UiTokenCollection, types::*};
use crate::tokinizer::time::time_regex_parser;
use crate::tokinizer::number::number_regex_parser;
use crate::tokinizer::percent::percent_regex_parser;
use crate::tokinizer::money::money_regex_parser;
use crate::tokinizer::text::text_regex_parser;
use crate::tokinizer::field::field_regex_parser;
use crate::tokinizer::atom::{atom_regex_parser, get_atom};
use crate::tokinizer::whitespace::whitespace_regex_parser;
use crate::tokinizer::comment::comment_regex_parser;

use operator::operator_regex_parser;
use regex::{Match, Regex};
use lazy_static::*;
use alloc::collections::btree_map::BTreeMap;

use self::month::month_parser;

lazy_static! {
    pub static ref TOKEN_REGEX_PARSER: Vec<(&'static str, RegexParser)> = {
        let m = vec![
        ("comment",    comment_regex_parser    as RegexParser),
        ("field",      field_regex_parser      as RegexParser),
        ("money",      money_regex_parser      as RegexParser),
        ("atom",       atom_regex_parser       as RegexParser),
        ("percent",    percent_regex_parser    as RegexParser),
        ("time",       time_regex_parser       as RegexParser),
        ("number",     number_regex_parser     as RegexParser),
        ("text",       text_regex_parser       as RegexParser),
        ("whitespace", whitespace_regex_parser as RegexParser),
        ("operator",   operator_regex_parser   as RegexParser)];
        m
    };
}

lazy_static! {
    pub static ref LANGUAGE_BASED_TOKEN_PARSER: Vec<Parser> = {
        let m = vec![month_parser as Parser];
        m
    };
}


pub type TokenParser = fn(config: &SmartCalcConfig, tokinizer: &mut Tokinizer) -> TokenParserResult;
pub type RegexParser = fn(config: &SmartCalcConfig, tokinizer: &mut Tokinizer, group_item: &[Regex]);
pub type Parser      = fn(config: &SmartCalcConfig, tokinizer: &mut Tokinizer, data: &str);

pub struct Tokinizer<'a> {
    pub column: u16,
    pub tokens: Vec<TokenType>,
    pub iter: Vec<char>,
    pub data: String,
    pub index: u16,
    pub indexer: usize,
    pub total: usize,
    pub token_infos: Vec<TokenInfo>,
    pub ui_tokens: UiTokenCollection,
    pub language: &'a str,
    pub config: &'a SmartCalcConfig
}

#[derive(Debug)]
#[derive(Clone)]
#[derive(PartialEq)]
pub enum TokenInfoStatus {
    Active,
    Removed
}

#[derive(Debug)]
#[derive(Clone)]
pub struct TokenInfo {
    pub start: usize,
    pub end: usize,
    pub token_type: Option<TokenType>,
    pub original_text: String,
    pub status: TokenInfoStatus
}

unsafe impl Send for TokenInfo {}
unsafe impl Sync for TokenInfo {}

impl<'a> Tokinizer<'a> {
    pub fn new(language: &'a str, data: &str, config: &'a SmartCalcConfig) -> Tokinizer<'a> {
        Tokinizer {
            column: 0,
            tokens: Vec::new(),
            iter: data.chars().collect(),
            data: data.to_string(),
            index: 0,
            indexer: 0,
            total: data.chars().count(),
            token_infos: Vec::new(),
            ui_tokens: UiTokenCollection::new(data),
            language,
            config
        }
    }

    pub fn token_infos(language: &'a str, data: &str, config: &'a SmartCalcConfig) -> Vec<TokenInfo> {
        let mut tokinizer = Tokinizer {
            column: 0,
            tokens: Vec::new(),
            iter: data.chars().collect(),
            data: data.to_string(),
            index: 0,
            indexer: 0,
            total: data.chars().count(),
            token_infos: Vec::new(),
            ui_tokens: UiTokenCollection::new(data),
            language,
            config
        };

        tokinizer.tokinize_with_regex();
        tokinizer.apply_aliases();

        tokinizer.token_infos
    }

    pub fn language_based_tokinize(&mut self) {
        let lowercase_data = self.data.to_lowercase();
        for func in LANGUAGE_BASED_TOKEN_PARSER.iter() {
            func(self.config, self, &lowercase_data);
        }
    }

    pub fn tokinize_with_regex(&mut self) {
        /* Token parser with regex */
        for (key, func) in TOKEN_REGEX_PARSER.iter() {
            if let Some(items) = self.config.token_parse_regex.get(&key.to_string()) { 
                func(self.config, self, items) 
            }
        }

        self.token_infos.retain(|x| x.token_type.is_some());
        self.token_infos.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
        //self.ui_tokens.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
    }

    pub fn apply_aliases(&mut self) {
        for token in &mut self.token_infos {
            for (re, data) in self.config.alias_regex.get(self.language).unwrap().iter() {
                if re.is_match(&token.original_text.to_lowercase()) {
                    let new_values = match self.config.token_parse_regex.get("atom") {
                        Some(items) => get_atom(data, items),
                        _ => Vec::new()
                    };

                    match new_values.len() {
                        1 => {
                            if let Some(token_type) = &new_values[0].2 {
                                token.token_type = Some(token_type.clone());
                                break;
                            }
                        },
                        0 => {
                            token.token_type = Some(TokenType::Text(data.to_string()));
                            break;
                        },
                        _ => log::warn!("{} has multiple atoms. It is not allowed", data)
                    };
                }
            }
        }
    }

    pub fn apply_rules(&mut self) {
        if let Some(language) = self.config.rule.get(self.language) {

            let mut execute_rules = true;
            while execute_rules {
                execute_rules = false;

                for (function_name, function, tokens_list) in language.iter() {
                    if cfg!(feature="debug-rules") {
                        log::debug!("# Checking for '{}'", function_name);
                    }

                    for rule_tokens in tokens_list {

                        let total_rule_token       = rule_tokens.len();
                        let mut rule_token_index   = 0;
                        let mut target_token_index = 0;
                        let mut start_token_index  = 0;
                        let mut fields             = BTreeMap::new();

                        while let Some(token) = self.token_infos.get(target_token_index) {
                            target_token_index += 1;
                            if token.status == TokenInfoStatus::Removed {
                                continue;
                            }

                            match &token.token_type {
                                Some(token_type) => {

                                    if let TokenType::Variable(variable) = &token_type {
                                        let is_same = TokenType::variable_compare(&rule_tokens[rule_token_index], variable.data.borrow().clone());
                                        if is_same {
                                            match TokenType::get_field_name(&rule_tokens[rule_token_index]) {
                                                Some(field_name) => fields.insert(field_name.to_string(), token),
                                                None => None
                                            };

                                            rule_token_index   += 1;
                                        } else {
                                            rule_token_index    = 0;
                                            start_token_index   = target_token_index;
                                        }
                                    }
                                    else if token == &rule_tokens[rule_token_index] {
                                        match TokenType::get_field_name(&rule_tokens[rule_token_index]) {
                                            Some(field_name) => fields.insert(field_name.to_string(), token),
                                            None => None
                                        };

                                        if cfg!(feature="debug-rules") {
                                            log::debug!("Ok, {:?} == {:?}", token.token_type, &rule_tokens[rule_token_index].token_type);
                                        }

                                        rule_token_index   += 1;
                                    }
                                    else {
                                        if cfg!(feature="debug-rules") {
                                            log::debug!("No, {:?} == {:?}", token.token_type, &rule_tokens[rule_token_index].token_type);
                                        }
                                        rule_token_index    = 0;
                                        start_token_index   = target_token_index;
                                    }

                                    if total_rule_token == rule_token_index { break; }
                                },
                                _ => ()
                            }
                        }

                        if total_rule_token == rule_token_index {
                            if cfg!(feature="debug-rules") {
                                log::debug!(" --------- {} executing", function_name);
                            }

                            match function(self.config, self, &fields) {
                                Ok(token) => {
                                    if cfg!(feature="debug-rules") {
                                        log::debug!("Rule function success with new token: {:?}", token);
                                    }

                                    let text_start_position = self.token_infos[start_token_index].start;
                                    let text_end_position   = self.token_infos[target_token_index - 1].end;
                                    execute_rules = true;

                                    for index in start_token_index..target_token_index {
                                        self.token_infos[index].status = TokenInfoStatus::Removed;
                                    }

                                    self.token_infos.insert(start_token_index, TokenInfo {
                                        start: text_start_position,
                                        end: text_end_position,
                                        token_type: Some(token),
                                        original_text: "".to_string(),
                                        status: TokenInfoStatus::Active
                                    });
                                    break;
                                },
                                Err(error) => log::info!("Rule execution error, {}", error)
                            }
                        }
                    }
                }
            }
        }

        if cfg!(feature="debug-rules") {
            log::debug!("Updated token_infos: {:?}", self.token_infos);
        }
    }

    pub fn add_token_location(&mut self, start: usize, end: usize, token_type: Option<TokenType>, text: String) -> bool {
        for item in &self.token_infos {
            if (item.start <= start && item.end > start) || (item.start < end && item.end >= end) {
                return false
            }
        }

        self.token_infos.push(TokenInfo {
            start,
            end,
            token_type,
            original_text: text,
            status: TokenInfoStatus::Active
        });
        true
    }

    pub fn add_token<'t>(&mut self, capture: &Option<Match<'t>>, token_type: Option<TokenType>) -> bool {
        match capture {
            Some(content) => self.add_token_location(content.start(), content.end(), token_type, content.as_str().to_string()),
            None => false
        }
    }

    pub fn is_end(&mut self) -> bool {
        self.total <= self.indexer
    }

    pub fn get_char(&mut self) -> char {
        return match self.iter.get(self.indexer) {
            Some(&c) => c,
            None => '\0'
        };
    }

    pub fn get_next_char(&mut self) -> char {
        return match self.iter.get(self.indexer + 1) {
            Some(&c) => c,
            None => '\0'
        };
    }

    pub fn get_indexer(&self) -> TokinizerBackup {
        TokinizerBackup {
            indexer: self.indexer,
            index: self.index,
            column: self.column
        }
    }

    pub fn set_indexer(&mut self, backup: TokinizerBackup) {
        self.indexer = backup.indexer;
        self.index   = backup.index;
        self.column  = backup.column;
    }

    pub fn increase_index(&mut self) {
        self.index   += self.get_char().len_utf8() as u16;
        self.indexer += 1;
        self.column  += 1;
    }
}

#[cfg(test)]
extern crate alloc;

#[cfg(test)]
pub mod test {
    use crate::executer::initialize;
    use crate::tokinizer::Tokinizer;
    use crate::types::TokenType;
    use alloc::vec::Vec;
    use alloc::string::String;
    use alloc::string::ToString;
    use crate::token::ui_token::UiTokenCollection;
    use crate::config::SmartCalcConfig;
    use lazy_static::*;

    lazy_static! {
        pub static ref STATIC_CONF: SmartCalcConfig = {
            let m = SmartCalcConfig::default();
            m
        };
    }

    pub fn setup<'a>(data: String) -> Tokinizer<'a> {
        let tokinizer = Tokinizer {
            column: 0,
            tokens: Vec::new(),
            iter: data.chars().collect(),
            data: data.to_string(),
            index: 0,
            indexer: 0,
            total: data.chars().count(),
            token_infos: Vec::new(),
            ui_tokens: UiTokenCollection::new(""),
            language: "en",
            config: &STATIC_CONF
        };
        initialize();
        tokinizer
    }

    #[cfg(test)]
    #[test]
    fn alias_test() {
        use alloc::string::ToString;
        use crate::tokinizer::test::setup;
        let mut tokinizer_mut = setup("add 1024 percent".to_string());

        tokinizer_mut.tokinize_with_regex();
        tokinizer_mut.apply_aliases();
        let tokens = &tokinizer_mut.token_infos;

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].start, 0);
        assert_eq!(tokens[0].end, 3);
        assert_eq!(tokens[0].token_type, Some(TokenType::Operator('+')));

        assert_eq!(tokens[1].start, 4);
        assert_eq!(tokens[1].end, 8);
        assert_eq!(tokens[1].token_type, Some(TokenType::Number(1024.0)));

        assert_eq!(tokens[2].start, 9);
        assert_eq!(tokens[2].end, 16);
        //assert_eq!(tokens[2].token_type, Some(TokenType::Operator('%')));
    }
}