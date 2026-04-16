use crate::interpreter::{ExecuteResult, Interpreter, Value, core};

pub fn print(_interpreter: &mut Interpreter, args: Vec<Value>) -> ExecuteResult<Option<Value>> {
    let arg_strs: Vec<_> = args.iter().map(ToString::to_string).collect();
    println!("{}", arg_strs.join(" "));
    Ok(None)
}

pub fn len(_interpreter: &mut Interpreter, args: Vec<Value>) -> ExecuteResult<Option<Value>> {
    core::expect_arg_count(1, args.len())?;

    let list = args[0].as_list()?;
    let len = list.borrow().len();

    Ok(Some(Value::Int(len as i64)))
}
