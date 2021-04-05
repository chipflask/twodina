use std::{error::Error as StdError, fmt::Display, path::PathBuf};

use anyhow;
use ruruby::{self, *};

#[derive(Debug, Clone)]
pub struct ScriptVm {
    pub vm: ruruby::VMRef,
    pub vm_context: ruruby::ContextRef,
    pub parser: ruruby::Parser,
}

impl ScriptVm {
    pub fn new() -> ScriptVm {
        let vm = new_interpreter();

        ScriptVm { vm,
                   vm_context: context(vm),
                   parser: ruruby::Parser::new() }
    }

    pub fn eval_repl_code(&mut self, code: &str) -> anyhow::Result<Value> {
        let parse_result = self.parser
                               .clone()
                               .parse_program_repl(PathBuf::from("dialogue"),
                                                   code.as_ref(),
                                                   Some(self.vm_context))
                               .map_err(|e| RubyStdError { source: e })?;
        let value = self.vm
                        .run_repl(parse_result, self.vm_context)
                        .map_err(|e| RubyStdError { source: e })?;

        Ok(value)
    }
}

// Wrap RubyError so we can implement std::error::Error.
#[derive(Debug)]
pub struct RubyStdError {
    source: RubyError,
}

impl StdError for RubyStdError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        None
    }

    fn cause(&self) -> Option<&dyn StdError> {
        None
    }
}

impl Display for RubyStdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RubyStdError: {:?}", self.source)
    }
}

pub fn new_interpreter() -> VMRef {
    let mut globals = GlobalsRef::new_globals();
    let vm = globals.create_main_fiber();
    let vals: Vec<Value> = vec!["script.rb"].iter()
                                            .map(|x| Value::string(*x))
                                            .collect();
    let argv = Value::array_from(vals);
    globals.set_toplevel_constant("ARGV", argv);

    // Patch with custom functions.
    let class = Module::class_under_object();
    // BuiltinClass::set_toplevel_constant() is private.
    BuiltinClass::object().set_const_by_str("ScriptCore", class.into());
    class.add_builtin_class_method("say", say);

    vm
}

pub fn context(vm: VMRef) -> ContextRef {
    ContextRef::new_heap(vm.globals.main_object,
                         Block::None,
                         ISeqRef::default(),
                         None)
}

fn say(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 2)?;

    eprintln!("Hello, from Rust.  Called with: {:?}", args);

    match &args.block {
        Block::None => {
            // No block argument.
        }
        block => {
            let mut args = Args::new(1);
            let arg = Value::string("A string constructed in Rust.");
            args[0] = arg;
            vm.eval_block(block, &args)?;
        }
    }

    if args.len() >= 1 {
        // Extract first arg.
        let mut arg0 = args[0];
        let s = arg0.expect_string("first argument")?;
        // Print it.
        eprintln!("Message: {}", s);
    }

    if args.len() >= 2 {
        // Extract second arg.
        let num = args[1].expect_integer("second argument")?;
        // Return incremented.
        return Ok(Value::integer(num + 1));
    }

    Ok(Value::nil())
}
