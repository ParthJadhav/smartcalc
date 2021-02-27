use alloc::string::String;
use alloc::string::ToString;
use alloc::collections::btree_map::BTreeMap;

use chrono::{Duration, Timelike};

use crate::{constants::{CONSTANT_PAIRS, ConstantType}, types::{TokenType}, worker::tools::{get_number, get_text}};
use crate::tokinizer::{TokenInfo};
use crate::formatter::{MINUTE, HOUR, DAY, WEEK, MONTH, YEAR};

pub fn duration_parse(fields: &BTreeMap<String, &TokenInfo>) -> core::result::Result<TokenType, String> {
    if (fields.contains_key("duration")) && fields.contains_key("type") {
        let duration = match get_number("duration", fields) {
            Some(number) => number as i64,
            _ => return Err("Duration information not valid".to_string())
        };

        let duration_type = match get_text("type", fields) {
            Some(number) => number,
            _ => return Err("Duration type information not valid".to_string())
        };

        let constant_type = match CONSTANT_PAIRS.read().unwrap().get("en").unwrap().get(&duration_type) {
            Some(constant) => constant.clone(),
            None => return Err("Duration type not valid".to_string())
        };

        let calculated_duration = match constant_type {
            ConstantType::Day => Duration::days(duration),
            ConstantType::Second => Duration::seconds(duration),
            ConstantType::Minute => Duration::minutes(duration),
            ConstantType::Hour => Duration::hours(duration),
            ConstantType::Week => Duration::weeks(duration),
            _ => return Err("Duration type not valid".to_string()) 
        };

        return Ok(TokenType::Duration(calculated_duration, duration, constant_type));
    }
    Err("Date type not valid".to_string())
}

pub fn as_duration(fields: &BTreeMap<String, &TokenInfo>) -> core::result::Result<TokenType, String> {
    if (fields.contains_key("source")) && fields.contains_key("type") {
        let duration_type = match get_text("type", fields) {
            Some(number) => number,
            _ => return Err("Duration type information not valid".to_string())
        };

        let constant_type = match CONSTANT_PAIRS.read().unwrap().get("en").unwrap().get(&duration_type) {
            Some(constant) => constant.clone(),
            None => return Err("Duration type not valid".to_string())
        };

        match fields.get("source") {
            Some(token_info) => match token_info.token_type {
                Some(TokenType::Duration(duration, _, _)) => {
                    let seconds = duration.num_seconds().abs() as f64;
                    
                    return match constant_type {
                        ConstantType::Day => Ok(TokenType::Number(seconds / DAY as f64)),
                        ConstantType::Second => Ok(TokenType::Number(seconds)),
                        ConstantType::Minute => Ok(TokenType::Number(seconds / MINUTE as f64)),
                        ConstantType::Hour => Ok(TokenType::Number(seconds / HOUR as f64)),
                        ConstantType::Week => Ok(TokenType::Number(seconds / WEEK as f64)),
                        _ => return Err("Duration type not valid".to_string()) 
                    };
                },
                Some(TokenType::Time(time)) => {
                    let seconds = time.num_seconds_from_midnight() as f64;
                    
                    return match constant_type {
                        ConstantType::Day => Ok(TokenType::Number(seconds / DAY as f64)),
                        ConstantType::Month => Ok(TokenType::Number(seconds / MONTH as f64)),
                        ConstantType::Year => Ok(TokenType::Number(seconds / YEAR as f64)),
                        ConstantType::Second => Ok(TokenType::Number(seconds)),
                        ConstantType::Minute => Ok(TokenType::Number(seconds / MINUTE as f64)),
                        ConstantType::Hour => Ok(TokenType::Number(seconds / HOUR as f64)),
                        ConstantType::Week => Ok(TokenType::Number(seconds / WEEK as f64)),
                        _ => return Err("Duration type not valid".to_string()) 
                    };
                }
                _ => ()
            },
            None => return Err("Source information not valid".to_string())
        };
        
        
        let duration = match get_number("duration", fields) {
            Some(number) => number as i64,
            _ => return Err("Duration information not valid".to_string())
        };

        let calculated_duration = match constant_type {
            ConstantType::Day => Duration::days(duration),
            ConstantType::Month => Duration::days(duration * 30),
            ConstantType::Year => Duration::days(duration * 365),
            ConstantType::Second => Duration::seconds(duration),
            ConstantType::Minute => Duration::minutes(duration),
            ConstantType::Hour => Duration::hours(duration),
            _ => return Err("Duration type not valid".to_string()) 
        };

        return Ok(TokenType::Duration(calculated_duration, duration, constant_type));
    }
    Err("Date type not valid".to_string())
}

#[cfg(test)]
#[test]
fn duration_parse_test_1() {
    use crate::tokinizer::test::setup;
    use crate::executer::token_generator;
    use crate::executer::token_cleaner;
    let tokinizer_mut = setup("10 days".to_string());

    tokinizer_mut.borrow_mut().language_based_tokinize();
    tokinizer_mut.borrow_mut().tokinize_with_regex();
    tokinizer_mut.borrow_mut().apply_aliases();
    tokinizer_mut.borrow_mut().apply_rules();

    let tokens = &tokinizer_mut.borrow().token_infos;

    let mut tokens = token_generator(&tokens);
    token_cleaner(&mut tokens);

    assert_eq!(tokens.len(), 1);
    
    assert_eq!(tokens[0], TokenType::Duration(Duration::days(10), 10, ConstantType::Day));
}

#[cfg(test)]
#[test]
fn duration_parse_test_2() {
    use crate::tokinizer::test::setup;
    use crate::executer::token_generator;
    use crate::executer::token_cleaner;
    let tokinizer_mut = setup("10 weeks".to_string());

    tokinizer_mut.borrow_mut().language_based_tokinize();
    tokinizer_mut.borrow_mut().tokinize_with_regex();
    tokinizer_mut.borrow_mut().apply_aliases();
    tokinizer_mut.borrow_mut().apply_rules();

    let tokens = &tokinizer_mut.borrow().token_infos;

    let mut tokens = token_generator(&tokens);
    token_cleaner(&mut tokens);

    assert_eq!(tokens.len(), 1);
    
    assert_eq!(tokens[0], TokenType::Duration(Duration::weeks(10), 10, ConstantType::Week));
}

#[cfg(test)]
#[test]
fn duration_parse_test_3() {
    use crate::tokinizer::test::setup;
    use crate::executer::token_generator;
    use crate::executer::token_cleaner;
    let tokinizer_mut = setup("60 minutes".to_string());

    tokinizer_mut.borrow_mut().language_based_tokinize();
    tokinizer_mut.borrow_mut().tokinize_with_regex();
    tokinizer_mut.borrow_mut().apply_aliases();
    tokinizer_mut.borrow_mut().apply_rules();

    let tokens = &tokinizer_mut.borrow().token_infos;

    let mut tokens = token_generator(&tokens);
    token_cleaner(&mut tokens);

    assert_eq!(tokens.len(), 1);
    
    assert_eq!(tokens[0], TokenType::Duration(Duration::minutes(60), 60, ConstantType::Minute));
}

#[cfg(test)]
#[test]
fn duration_parse_test_4() {
    use crate::tokinizer::test::setup;
    use crate::executer::token_generator;
    use crate::executer::token_cleaner;
    let tokinizer_mut = setup("5 weeks as seconds".to_string());

    tokinizer_mut.borrow_mut().language_based_tokinize();
    tokinizer_mut.borrow_mut().tokinize_with_regex();
    tokinizer_mut.borrow_mut().apply_aliases();
    tokinizer_mut.borrow_mut().apply_rules();

    let tokens = &tokinizer_mut.borrow().token_infos;

    let mut tokens = token_generator(&tokens);
    token_cleaner(&mut tokens);

    assert_eq!(tokens.len(), 1);
    
    assert_eq!(tokens[0], TokenType::Number(3024000.0));
}

#[cfg(test)]
#[test]
fn duration_parse_test_5() {
    use crate::tokinizer::test::setup;
    use crate::executer::token_generator;
    use crate::executer::token_cleaner;
    let tokinizer_mut = setup("48 weeks as hours".to_string());

    tokinizer_mut.borrow_mut().language_based_tokinize();
    tokinizer_mut.borrow_mut().tokinize_with_regex();
    tokinizer_mut.borrow_mut().apply_aliases();
    tokinizer_mut.borrow_mut().apply_rules();

    let tokens = &tokinizer_mut.borrow().token_infos;

    let mut tokens = token_generator(&tokens);
    token_cleaner(&mut tokens);

    assert_eq!(tokens.len(), 1);
    
    assert_eq!(tokens[0], TokenType::Number(8064.0));
}

#[cfg(test)]
#[test]
fn duration_parse_test_6() {
    use crate::tokinizer::test::setup;
    use crate::executer::token_generator;
    use crate::executer::token_cleaner;
    let tokinizer_mut = setup("11:50 as hour".to_string());

    tokinizer_mut.borrow_mut().language_based_tokinize();
    tokinizer_mut.borrow_mut().tokinize_with_regex();
    tokinizer_mut.borrow_mut().apply_aliases();
    tokinizer_mut.borrow_mut().apply_rules();

    let tokens = &tokinizer_mut.borrow().token_infos;

    let mut tokens = token_generator(&tokens);
    token_cleaner(&mut tokens);

    assert_eq!(tokens.len(), 1);
    
    assert_eq!(tokens[0], TokenType::Number(11.833333333333334));
}