use crate::interpreter::{ExecuteError, ExecuteResult, Interpreter, Value};

pub fn print(_executor: &mut Interpreter, args: Vec<Value>) -> ExecuteResult<Option<Value>> {
    let arg_strs: Vec<_> = args.iter().map(ToString::to_string).collect();
    println!("{}", arg_strs.join(" "));
    Ok(None)
}

pub fn len(_executor: &mut Interpreter, args: Vec<Value>) -> ExecuteResult<Option<Value>> {
    if args.len() != 1 {
        return Err(ExecuteError::MismatchedFuncArgs {
            expected: 1,
            got: args.len(),
        });
    }

    let list = args[0].as_list()?;
    let len = list.borrow().len();

    Ok(Some(Value::Int(len.try_into().unwrap())))
}
