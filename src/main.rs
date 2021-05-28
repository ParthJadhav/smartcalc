extern crate libsmartcalc;

use libsmartcalc::executer::{initialize};
use libsmartcalc::app::SmartCalc;

mod webserver;

fn main() {
    if cfg!(feature="webserver") {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            match webserver::start_webserver().await {
                Ok(_) => println!("Webserver stopped"),
                Err(error) => println!("Error : {:}", error),
            };
        });
    } else {
        let test_data = r"
        date information = 11:30
        date information add 1 hour 1 minute 31 second".to_string();
        initialize();
    
        let app = SmartCalc::default();
        let language = "en".to_string();
        let results = app.execute(&language, &test_data);
        
        for result in results {
            match result {
                Ok((tokens, ast)) => {
                    println!("{:?}", tokens);
                    println!("{}", app.format_result(&language, ast))              
                },
                Err(error) => println!("Error : {}", error)
            };
        }
    }
}
