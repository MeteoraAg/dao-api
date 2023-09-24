// Unwrap a Result, print the error if failed, and stop the function execution
#[macro_export]
macro_rules! unwrap_ok_or {
    ($res: expr, $args: tt) => {
        match $res {
            Ok(v) => v,
            Err(_e) => {
                println!("{:?}", $args);
                return;
            }
        }
    };
}

// Unwrap an Option, print the error if failed, and stop the function execution
#[macro_export]
macro_rules! unwrap_some_or {
    ($res: expr, $args: tt) => {
        match $res {
            Some(v) => v,
            None => {
                println!("{:?}", $args);
                return;
            }
        }
    };
}

#[cfg(test)]
mod test_macros {
    use super::*;
    #[test]
    fn test_unwrap_ok_or() {
        let something_ok: Result<&str, &str> = Result::Ok("Ok");
        let something_err: Result<&str, &str> = Result::Err("Some error message");
        let ok = unwrap_ok_or!(something_ok, "Error when unwrap Result");
        assert_eq!(ok, "Ok");
        unwrap_ok_or!(something_err, "This error message will be printed! And, the code below this unwrap will not be executed!");
        panic!("This shall not be reached!");
    }

    #[test]
    fn test_unwrap_some_or() {
        let something_ok: Option<&str> = Option::Some("Ok");
        let something_none: Option<&str> = Option::None;
        let ok = unwrap_some_or!(something_ok, "Error when unwrap Option");
        assert_eq!(ok, "Ok");
        unwrap_some_or!(something_none, "This error message will be printed! And, the code below this unwrap will not be executed!");
        panic!("This shall not be reached!");
    }
}
