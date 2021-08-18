use alloc::rc::Rc;
use alloc::string::String;
use alloc::string::ToString;
use alloc::format;
use alloc::sync::Arc;
use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Timelike};

use crate::app::Storage;
use crate::config::SmartCalcConfig;
use crate::{formatter::{DAY, MONTH, YEAR}, types::*};
use crate::tools::convert_currency;
use crate::formatter::{MINUTE, HOUR};

pub struct Interpreter;

impl Interpreter {
    pub fn execute(config: &SmartCalcConfig, ast: Rc<BramaAstType>, storage: &mut Storage) -> Result<Rc<BramaAstType>, String> {
        Interpreter::execute_ast(config, storage, ast)
    }

    fn execute_ast(config: &SmartCalcConfig, storage: &mut Storage, ast: Rc<BramaAstType>) -> Result<Rc<BramaAstType>, String> {
        match &*ast {
            BramaAstType::Binary { left, operator, right } => Interpreter::executer_binary(config, storage, left.clone(), *operator, right.clone()),
            BramaAstType::Assignment { index, expression } => Interpreter::executer_assignment(config, storage, *index, expression.clone()),
            BramaAstType::Variable(variable)               => Ok(Interpreter::executer_variable(variable.clone())),
            BramaAstType::Percent(_)                       => Ok(ast),
            BramaAstType::Number(_)                        => Ok(ast),
            BramaAstType::Time(_)                          => Ok(ast),
            BramaAstType::Date(_)                          => Ok(ast),
            BramaAstType::Money(_, _)                      => Ok(ast),
            BramaAstType::Duration(_)                      => Ok(ast),
            BramaAstType::Month(_)                         => Ok(ast),
            BramaAstType::PrefixUnary(ch, ast)             => Interpreter::executer_unary(config, storage, *ch, ast.clone()),
            BramaAstType::None                             => Ok(Rc::new(BramaAstType::None)),
            _ => {
                log::debug!("Operation not implemented {:?}", ast);
                Ok(Rc::new(BramaAstType::None))
            }
        }
    }

    fn executer_variable(variable: Rc<VariableInfo>) -> Rc<BramaAstType> {
        variable.data.borrow().clone()
    }

    fn executer_assignment(config: &SmartCalcConfig, storage: &mut Storage, index: usize, expression: Rc<BramaAstType>) -> Result<Rc<BramaAstType>, String> {
        let computed  = Interpreter::execute_ast(config, storage, expression)?;
        *storage.variables[index].data.borrow_mut() = computed.clone();
        Ok(computed)
    }


    fn detect_target_currency(left: Rc<BramaAstType>, right: Rc<BramaAstType>) -> Option<Arc<CurrencyInfo>> {
        let left_currency = match &*left {
            BramaAstType::Money(_, currency) => Some(currency.clone()),
            BramaAstType::Variable(variable) => match &**variable.data.borrow() {
                BramaAstType::Money(_, currency) => Some(currency.clone()),
                _ => None
            },
            _ => None
        };

        let right_currency = match &*right {
            BramaAstType::Money(_, currency) => Some(currency.clone()),
            BramaAstType::Variable(variable) => match &**variable.data.borrow() {
                BramaAstType::Money(_, currency) => Some(currency.clone()),
                _ => None
            },
            _ => None
        };

        match (left_currency, right_currency) {
            (Some(_), Some(r)) => Some(r.clone()),
            (None, Some(r)) => Some(r.clone()),
            (Some(l), None) => Some(l.clone()),
            _ => None
        }
    }

    fn get_first_percent(left: Rc<BramaAstType>, right: Rc<BramaAstType>) -> Option<f64> {
        match &*left {
            BramaAstType::Percent(percent) => return Some(*percent),
            BramaAstType::Variable(variable) => if let BramaAstType::Percent(percent) = **variable.data.borrow() { return Some(percent) },
            _ => ()
        };

        match &*right {
            BramaAstType::Percent(percent) => Some(*percent),
            BramaAstType::Variable(variable) => match **variable.data.borrow() {
                BramaAstType::Percent(percent) => Some(percent),
                _ => None
            },
            _ => None
        }
    }

    fn get_first_money(left: Rc<BramaAstType>, right: Rc<BramaAstType>) -> Option<f64> {
        match &*left {
            BramaAstType::Money(money, _) => return Some(*money),
            BramaAstType::Variable(variable) => if let BramaAstType::Money(money, _) = **variable.data.borrow() { return Some(money) },
            _ => ()
        };

        match &*right {
            BramaAstType::Money(money, _) => Some(*money),
            BramaAstType::Variable(variable) => match **variable.data.borrow() {
                BramaAstType::Money(money, _) => Some(money),
                _ => None
            },
            _ => None
        }
    }

    fn get_first_number(left: Rc<BramaAstType>, right: Rc<BramaAstType>) -> Option<f64> {
        match &*left {
            BramaAstType::Number(number) => return Some(*number),
            BramaAstType::Variable(variable) => if let BramaAstType::Number(number) = **variable.data.borrow() { 
                return Some(number)
            },
            _ => ()
        };

        match &*right {
            BramaAstType::Number(number) => Some(*number),
            BramaAstType::Variable(variable) => match **variable.data.borrow() {
                BramaAstType::Number(number) => Some(number),
                _ => None
            },
            _ => None
        }
    }

    fn get_moneys(left: Rc<BramaAstType>, right: Rc<BramaAstType>) -> Option<(Money, Money)> {
        let left_money = match &*left {
            BramaAstType::Money(price, currency) => Money(*price, currency.clone()),
            BramaAstType::Variable(variable) => match &**variable.data.borrow() {
                BramaAstType::Money(price, currency) => Money(*price, currency.clone()),
                _ => return None
            },
            _ => return None
        };

        let right_number = match &*right {
            BramaAstType::Money(price, currency) => Money(*price, currency.clone()),
            BramaAstType::Variable(variable) => match &**variable.data.borrow() {
                BramaAstType::Money(price, currency) => Money(*price, currency.clone()),
                _ => return None
            },
            _ => return None
        };

        Some((left_money, right_number))
    }

    fn get_numbers(left: Rc<BramaAstType>, right: Rc<BramaAstType>) -> Option<(f64, f64)> {
        let left_number = match &*left {
            BramaAstType::Number(number) => *number,
            BramaAstType::Money(price, _) => *price,
            BramaAstType::Percent(percent) => *percent,
            BramaAstType::Duration(duration) => Interpreter::get_high_duration_number(*duration) as f64,
            BramaAstType::Variable(variable) => match **variable.data.borrow() {
                BramaAstType::Number(number) => number,
                BramaAstType::Money(price, _) => price,
                BramaAstType::Percent(percent) => percent,
                BramaAstType::Duration(duration) => Interpreter::get_high_duration_number(duration) as f64,
                _ => return None
            },
            _ => return None
        };

        let right_number = match &*right {
            BramaAstType::Number(number) => *number,
            BramaAstType::Money(price, _) => *price,
            BramaAstType::Percent(percent) => *percent,
            BramaAstType::Duration(duration) => Interpreter::get_high_duration_number(*duration) as f64,
            BramaAstType::Variable(variable) => match **variable.data.borrow() {
                BramaAstType::Number(number) => number,
                BramaAstType::Money(price, _) => price,
                BramaAstType::Percent(percent) => percent,
                BramaAstType::Duration(duration) => Interpreter::get_high_duration_number(duration) as f64,
                _ => return None
            },
            _ => return None
        };

        Some((left_number, right_number))
    }

    fn get_duration(ast: Rc<BramaAstType>) -> Option<Duration> {
        let number = match &*ast {
            BramaAstType::Duration(duration) => *duration,
            BramaAstType::Variable(variable) => match **variable.data.borrow() {
                BramaAstType::Duration(duration) => duration,
                _ => return None
            },
            _ => return None
        };
        
        Some(number)
    }

    fn get_high_duration_number(duration: Duration) -> i64 {
        let duration_info = duration.num_seconds().abs();
        if duration_info >= YEAR {
            return duration_info / YEAR;
        }

        if duration_info >= MONTH {
            return (duration_info / MONTH) % 30;
        }

        if duration_info >= DAY {
            return duration_info / DAY;
        }

        if duration_info >= HOUR {
            return (duration_info / HOUR) % 24;
        }

        if duration_info >= MINUTE {
            return (duration_info / MINUTE) % 60;
        }

        duration_info
    }

    fn duration_to_time(duration: i64) -> NaiveTime {
        let mut duration_info = duration.abs();
        let mut hours         = 0;
        let mut minutes       = 0;
        let seconds;

        if duration_info >= HOUR {
            hours = (duration_info / HOUR) % 24;
            duration_info %= HOUR
        }

        if duration_info >= MINUTE {
            minutes = (duration_info / MINUTE) % 60;
            duration_info %= MINUTE
        }

        seconds = duration_info;
        NaiveTime::from_hms(hours as u32, minutes as u32, seconds as u32)
    }

    fn get_month_from_duration(duration: Duration) -> i64 {
        duration.num_seconds().abs() / MONTH
    }

    fn get_year_from_duration(duration: Duration) -> i64 {
        duration.num_seconds().abs() / YEAR
    }

    fn get_durations(left: Rc<BramaAstType>, right: Rc<BramaAstType>) -> Option<(Duration, Duration)> {
        let left_time = match &*left {
            BramaAstType::Duration(duration) => *duration,
            BramaAstType::Variable(variable) => match **variable.data.borrow() {
                BramaAstType::Duration(duration) => duration,
                _ => return None
            },
            _ => return None
        };

        let right_time = match &*right {
            BramaAstType::Duration(duration) => *duration,
            BramaAstType::Variable(variable) => match **variable.data.borrow() {
                BramaAstType::Duration(duration) => duration,
                _ => return None
            },
            _ => return None
        };

        Some((left_time, right_time))
    }
    
    fn get_date(ast: Rc<BramaAstType>) -> Option<NaiveDate> {
        match &*ast {
            BramaAstType::Date(date) => Some(*date),
            BramaAstType::Variable(variable) => match **variable.data.borrow() {
                BramaAstType::Date(date) => Some(date),
                _ => None
            },
            _ => None
        }
    }

    fn get_times(left: Rc<BramaAstType>, right: Rc<BramaAstType>) -> Option<(NaiveTime, NaiveTime, bool)> {
        let left_time = match &*left {
            BramaAstType::Time(time) => *time,
            BramaAstType::Duration(duration) => Interpreter::duration_to_time(duration.num_seconds()),
            BramaAstType::Variable(variable) => match **variable.data.borrow() {
                BramaAstType::Time(time) => time,
                BramaAstType::Duration(duration) => Interpreter::duration_to_time(duration.num_seconds()),
                _ => return None
            },
            _ => return None
        };

        let (right_time, is_negative) = match &*right {
            BramaAstType::Time(time) => (*time, false),
            BramaAstType::Duration(duration) => (Interpreter::duration_to_time(duration.num_seconds()), duration.num_seconds().is_negative()),
            BramaAstType::Variable(variable) => match **variable.data.borrow() {
                BramaAstType::Time(time) => (time, false),
                BramaAstType::Duration(duration) => (Interpreter::duration_to_time(duration.num_seconds()), duration.num_seconds().is_negative()),
                _ => return None
            },
            _ => return None
        };

        Some((left_time, right_time, is_negative))
    }

    fn calculate_number(operator: char, left: Rc<BramaAstType>, right: Rc<BramaAstType>) -> Result<Rc<BramaAstType>, String> {
        /* Percent operation */
        if let Some(percent) = Interpreter::get_first_percent(left.clone(), right.clone()) {
            let number = match Interpreter::get_first_number(left, right) {
                Some(num) => num,
                None => return Err("Number information not valid".to_string())
            };

            return match operator {
                '+' => Ok(Rc::new(BramaAstType::Number(number + ((number * percent) / 100.0)))),
                '-' => Ok(Rc::new(BramaAstType::Number(number - ((number * percent) / 100.0)))),
                '*' => Ok(Rc::new(BramaAstType::Number(number * ((number * percent) / 100.0)))),
                '/' => Ok(Rc::new(BramaAstType::Number(Interpreter::do_divition(number, Interpreter::do_divition(number * percent, 100.0))))),
                _ => Err(format!("Unknown operator. ({})", operator))
            };
        };
        
        match Interpreter::get_numbers(left, right) {
            Some((left_number, right_number)) => {
                match operator {
                    '+' => Ok(Rc::new(BramaAstType::Number(left_number + right_number))),
                    '-' => Ok(Rc::new(BramaAstType::Number(left_number - right_number))),
                    '/' => Ok(Rc::new(BramaAstType::Number(Interpreter::do_divition(left_number, right_number)))),
                    '*' => Ok(Rc::new(BramaAstType::Number(left_number * right_number))),
                    _ => Err(format!("Unknown operator. ({})", operator))
                }
            },
            None => Err("Unknown calculation".to_string())
        }
    }
    
    fn calculate_date(operator: char, left: Rc<BramaAstType>, right: Rc<BramaAstType>) -> Result<Rc<BramaAstType>, String> {
        match (Interpreter::get_date(left), Interpreter::get_duration(right)) {
            (Some(date), Some(duration)) => {
                let mut date     = date;
                let mut duration = duration;

                return match operator {
                    '+' => {
                        match Interpreter::get_year_from_duration(duration) {
                            0 => (),
                            n => {
                                let years_diff = date.year() + n as i32;
                                date     = NaiveDate::from_ymd(years_diff as i32, date.month() as u32, date.day());
                                duration = Duration::seconds(duration.num_seconds() - (YEAR * n))
                            }
                        };

                        match Interpreter::get_month_from_duration(duration) {
                            0 => (),
                            n => {
                                let years_diff = (date.month() + n as u32) / 12;
                                let month = (date.month() + n as u32) % 12;
                                date     = NaiveDate::from_ymd(date.year() + years_diff as i32, month as u32, date.day());
                                duration = Duration::seconds(duration.num_seconds() - (MONTH * n))
                            }
                        };
                        Ok(Rc::new(BramaAstType::Date(date + duration)))
                    },

                    '-' => {
                        match Interpreter::get_year_from_duration(duration) {
                            0 => (),
                            n => {
                                let years_diff = date.year() - n as i32;
                                date     = NaiveDate::from_ymd(years_diff as i32, date.month() as u32, date.day());
                                duration = Duration::seconds(duration.num_seconds() - (YEAR * n))
                            }
                        };

                        match Interpreter::get_month_from_duration(duration) {
                            0 => (),
                            n => {
                                let years = date.year() - (n as i32 / 12);
                                let mut months = date.month() as i32 - (n as i32 % 12);
                                if months < 0 {
                                    months += 12;
                                }

                                date = NaiveDate::from_ymd(years as i32, months as u32, date.day());
                                duration = Duration::seconds(duration.num_seconds() - (MONTH * n))
                            }
                        };
                        Ok(Rc::new(BramaAstType::Date(date - duration)))
                    },
                    _ => Err(format!("Unknown operator. ({})", operator))
                };
                
            },
            _ => Err(format!("Unknown operator. ({})", operator))
        }
    }

    fn calculate_time(operator: char, left: Rc<BramaAstType>, right: Rc<BramaAstType>) -> Result<Rc<BramaAstType>, String> {

        /* Time calculation operation */
        match Interpreter::get_times(left, right) {
            Some((left_time, right_time, is_negative)) => {
                let calculated_right = Duration::seconds(right_time.num_seconds_from_midnight() as i64);

                if is_negative {
                    return Ok(Rc::new(BramaAstType::Time(left_time - calculated_right)));
                }
                
                return match operator {
                    '+' => Ok(Rc::new(BramaAstType::Time(left_time + calculated_right))),
                    '-' => Ok(Rc::new(BramaAstType::Time(left_time - calculated_right))),
                    _ => return Err(format!("Unknown operator. ({})", operator))
                };
            },
            None => Err(format!("Unknown operator. ({})", operator))
        }
    }

    fn calculate_duration(operator: char, left: Rc<BramaAstType>, right: Rc<BramaAstType>) -> Result<Rc<BramaAstType>, String> {
        /* Time calculation operation */
        match Interpreter::get_durations(left, right) {
            Some((left_time, right_time)) => {
                
                return match operator {
                    '+' => Ok(Rc::new(BramaAstType::Duration(left_time + right_time))),
                    '-' => Ok(Rc::new(BramaAstType::Duration(left_time - right_time))),
                    _ => return Err(format!("Unknown operator. ({})", operator))
                };
            },
            None => Err(format!("Unknown operator. ({})", operator))
        }
    }

    fn calculate_money(config: &SmartCalcConfig, operator: char, left: Rc<BramaAstType>, right: Rc<BramaAstType>) -> Result<Rc<BramaAstType>, String> {
        let to_currency = match Interpreter::detect_target_currency(left.clone(), right.clone()) {
            Some(currency) => currency,
            None => return Err("Currency information not valid".to_string())
        };
        
        /* Percent operation */
        if let Some(percent) = Interpreter::get_first_percent(left.clone(), right.clone()) {
            let price = match Interpreter::get_first_money(left, right) {
                Some(currency) => currency,
                None => return Err("Price information not valid".to_string())
            };

            return match operator {
                '+' => Ok(Rc::new(BramaAstType::Money(price + ((price * percent) / 100.0), to_currency))),
                '-' => Ok(Rc::new(BramaAstType::Money(price - ((price * percent) / 100.0), to_currency))),
                '*' => Ok(Rc::new(BramaAstType::Money(price * ((price * percent) / 100.0), to_currency))),
                '/' => Ok(Rc::new(BramaAstType::Money(Interpreter::do_divition(price, Interpreter::do_divition(price * percent, 100.0)), to_currency))),
                _ => Err(format!("Unknown operator. ({})", operator))
            };
        }
        
        /* Money calculation operation */
    
        if let Some((left, right)) = Interpreter::get_moneys(left.clone(), right.clone()) {
            let left_price = convert_currency(config, &left, &right);
            return match operator {
                '+' => Ok(Rc::new(BramaAstType::Money(left_price + right.get_price(), right.get_currency()))),
                '-' => Ok(Rc::new(BramaAstType::Money(left_price - right.get_price(), right.get_currency()))),
                '/' => Ok(Rc::new(BramaAstType::Money(Interpreter::do_divition(left_price, right.get_price()), right.get_currency()))),
                '*' => Ok(Rc::new(BramaAstType::Money(left_price * right.get_price(), right.get_currency()))),
                _ => Err(format!("Unknown operator. ({})", operator))
            };
        }
        
        /* Normal operation */
        match Interpreter::get_numbers(left, right) {
            Some((left_number, right_number)) => {
                match operator {
                    '+' => Ok(Rc::new(BramaAstType::Money(left_number + right_number, to_currency))),
                    '-' => Ok(Rc::new(BramaAstType::Money(left_number - right_number, to_currency))),
                    '/' => Ok(Rc::new(BramaAstType::Money(Interpreter::do_divition(left_number, right_number), to_currency))),
                    '*' => Ok(Rc::new(BramaAstType::Money(left_number * right_number, to_currency))),
                    _ => Err(format!("Unknown operator. ({})", operator))
                }
            },
            None => Err("Unknown calculation".to_string())
        }
    }

    fn do_divition(left: f64, right: f64) -> f64 {
        let mut calculation = left / right;
        if calculation.is_infinite() || calculation.is_nan() {
            calculation = 0.0;
        }
        calculation
    }

    fn executer_binary(config: &SmartCalcConfig, storage: &mut Storage, left: Rc<BramaAstType>, operator: char, right: Rc<BramaAstType>) -> Result<Rc<BramaAstType>, String> {
        let computed_left  = Interpreter::execute_ast(config, storage, left)?;
        let computed_right = Interpreter::execute_ast(config, storage, right)?;

        match (&*computed_left, &*computed_right) {
            (BramaAstType::Money(_, _), _)       | (_, BramaAstType::Money(_, _))       => Interpreter::calculate_money(config, operator, computed_left.clone(), computed_right.clone()),
            (BramaAstType::Date(_), _)           | (_, BramaAstType::Date(_))           => Interpreter::calculate_date(operator, computed_left.clone(), computed_right.clone()),
            (BramaAstType::Time(_), _)           | (_, BramaAstType::Time(_))           => Interpreter::calculate_time(operator, computed_left.clone(), computed_right.clone()),
            (BramaAstType::Duration(_), _)       | (_, BramaAstType::Duration(_))       => Interpreter::calculate_duration(operator, computed_left.clone(), computed_right.clone()),
            (BramaAstType::Number(_), _)         | (_, BramaAstType::Number(_))         => Interpreter::calculate_number(operator, computed_left.clone(), computed_right.clone()),
            _ => Err("Uknown calculation result".to_string())
        }
    }

    fn executer_unary(config: &SmartCalcConfig, storage: &mut Storage, operator: char, ast: Rc<BramaAstType>) -> Result<Rc<BramaAstType>, String> {
        let computed = Interpreter::execute_ast(config, storage, ast)?;

        let result = match operator {
            '+' => return Ok(computed),
            '-' => match &*computed {
                BramaAstType::Money(money, currency) => BramaAstType::Money(*money * -1.0, currency.clone()),
                BramaAstType::Percent(percent)       => BramaAstType::Percent(*percent * -1.0),
                BramaAstType::Number(number)         => BramaAstType::Number(*number * -1.0),
                _ => return Err("Syntax error".to_string())
            },
            _ => return Err("Syntax error".to_string())
        };

        Ok(Rc::new(result))
    }
}
